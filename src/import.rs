//! Scan-pair import, versioned session export, and replay.
//!
//! ## Import format (`csm-rs-scan-pair`, version 1)
//!
//! ```json
//! {
//!   "format": "csm-rs-scan-pair",
//!   "version": 1,
//!   "initial_guess": [0.0, 0.0, 0.0],
//!   "reference": { "kind": "polar", "angles": [...], "readings": [5.0, null, ...], "valid": [...] },
//!   "sensor":    { "kind": "cartesian", "points": [[8.0, 0.0], ...], "valid": [...] },
//!   "config": { }
//! }
//! ```
//!
//! - Polar rays are ordered by bearing, in radians; readings are metres and
//!   `null` marks a missing return (`valid` must be false there).
//! - Cartesian points are ordered by bearing; `angles` may supply explicit
//!   bearings, otherwise `atan2(y, x)` is derived.
//! - `sigma` and `true_alpha` are optional per-ray arrays for the weighting
//!   paths.
//! - `config` is an optional [`MatcherConfig`] (any matcher field);
//!   simulation-only fields are not part of this document.
//!
//! Imported data has no ground truth, so responses never fabricate one.

use csm_rs::{Pose, PreparedPolarScan};
use serde::{Deserialize, Serialize};

use crate::config::{MatcherConfig, RunRequest};
use crate::engine::{match_pair, prepare_frame, scan_points};
use crate::scene::Scene;
use crate::simulation::{
    guess_rng, initial_guess, pose_at, relative_pose, scan_for, sensor_pose, SessionRecord,
    SessionResult,
};

/// One scan in an imported pair.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScanInput {
    Polar {
        angles: Vec<f64>,
        readings: Vec<Option<f64>>,
        valid: Vec<bool>,
        #[serde(default)]
        sigma: Option<Vec<f64>>,
        #[serde(default)]
        true_alpha: Option<Vec<f64>>,
    },
    Cartesian {
        points: Vec<[f64; 2]>,
        valid: Vec<bool>,
        #[serde(default)]
        angles: Option<Vec<f64>>,
    },
}

impl ScanInput {
    fn to_prepared(&self) -> Result<PreparedPolarScan, String> {
        match self {
            ScanInput::Polar {
                angles,
                readings,
                valid,
                sigma,
                true_alpha,
            } => {
                if readings.len() != angles.len() || valid.len() != angles.len() {
                    return Err("polar scan arrays must have equal length".to_owned());
                }
                let readings: Vec<f64> = readings
                    .iter()
                    .zip(valid)
                    .enumerate()
                    .map(|(i, (value, ok))| match value {
                        Some(value) => Ok(*value),
                        None if !ok => Ok(f64::NAN),
                        None => Err(format!("ray {i} is valid but has a missing reading")),
                    })
                    .collect::<Result<_, _>>()?;
                // Validate via the public contract before building storage.
                let _ = csm_rs::PolarScan::with_inputs(
                    angles,
                    &readings,
                    valid,
                    sigma.as_deref(),
                    true_alpha.as_deref(),
                )
                .map_err(|error| error.to_string())?;
                PreparedPolarScan::from_polar_with_inputs(
                    angles.clone(),
                    readings,
                    valid.clone(),
                    sigma.clone(),
                    true_alpha.clone(),
                )
                .map_err(|error| error.to_string())
            }
            ScanInput::Cartesian {
                points,
                valid,
                angles,
            } => {
                if let Some(angles) = angles {
                    PreparedPolarScan::from_cartesian_with_angles(points, angles, valid)
                        .map_err(|error| error.to_string())
                } else {
                    PreparedPolarScan::from_cartesian(points, valid)
                        .map_err(|error| error.to_string())
                }
            }
        }
    }
}

/// A complete scan-pair import document.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScanPair {
    pub format: String,
    #[serde(default = "default_format_version")]
    pub version: u32,
    pub reference: ScanInput,
    pub sensor: ScanInput,
    #[serde(default)]
    pub initial_guess: Option<[f64; 3]>,
    #[serde(default)]
    pub config: Option<MatcherConfig>,
}

fn default_format_version() -> u32 {
    1
}

impl ScanPair {
    fn validate(&self) -> Result<(), String> {
        if self.format != "csm-rs-scan-pair" {
            return Err(format!("unsupported import format '{}'", self.format));
        }
        if self.version != 1 {
            return Err(format!("unsupported import version {}", self.version));
        }
        Ok(())
    }
}

/// Result of matching an imported or replayed pair. There is no ground truth.
#[derive(Serialize, Deserialize)]
pub struct ImportResponse {
    pub reference: Vec<[f64; 2]>,
    pub sensor_unaligned: Vec<[f64; 2]>,
    pub sensor_aligned: Vec<[f64; 2]>,
    pub initial_pose: [f64; 3],
    pub estimated_pose: [f64; 3],
    pub valid: bool,
    pub accepted: bool,
    pub termination: String,
    pub iterations: i32,
    pub nvalid: i32,
    pub error: f64,
    pub covariance_status: String,
    pub covariance: Option<[f64; 3]>,
    pub extent: f64,
}

/// Match an imported pair through the shared execution path. Ground truth is
/// never invented.
pub fn run_import(pair: &ScanPair) -> Result<ImportResponse, String> {
    pair.validate()?;
    let config = pair.config.clone().unwrap_or_default();
    let reference = pair.reference.to_prepared()?;
    let sensor = pair.sensor.to_prepared()?;
    let guess = pair
        .initial_guess
        .map(Pose::from_array)
        .unwrap_or(Pose::IDENTITY);

    let reference_points = scan_points(&reference, Pose::IDENTITY);
    let sensor_unaligned = scan_points(&sensor, guess);
    let report = match_pair(reference.clone(), sensor.clone(), guess, &config, false)?;
    let estimate = Pose::from_array(report.estimated_pose);

    Ok(ImportResponse {
        reference: reference_points,
        sensor_unaligned,
        sensor_aligned: scan_points(&sensor, estimate),
        initial_pose: guess.to_array(),
        estimated_pose: report.estimated_pose,
        valid: report.valid,
        accepted: report.accepted,
        termination: report.termination,
        iterations: report.iterations,
        nvalid: report.nvalid,
        error: report.error,
        covariance_status: report.covariance_status,
        covariance: report.covariance,
        extent: bounding_extent(&reference, &sensor, guess, estimate),
    })
}

fn bounding_extent(
    reference: &PreparedPolarScan,
    sensor: &PreparedPolarScan,
    guess: Pose,
    estimate: Pose,
) -> f64 {
    let mut largest: f64 = 1.0;
    for (scan, transform) in [
        (reference, Pose::IDENTITY),
        (sensor, guess),
        (sensor, estimate),
    ] {
        for point in scan_points(scan, transform) {
            largest = largest.max(point[0].abs()).max(point[1].abs());
        }
    }
    largest * 1.1
}

/// Generate the simulated frame for `request` and package a versioned session.
pub fn export_session(request: &RunRequest) -> Result<SessionRecord, String> {
    let gen = &request.generation;
    gen.validate()?;
    let scene = Scene::by_name(&gen.scenario);
    let reference_pose = pose_at(&gen.scenario, 0, gen.motion);
    let sensor_pose_world = sensor_pose(gen, reference_pose, gen.step);
    let reference = scan_for(&scene, reference_pose, gen, 0);
    let sensor = scan_for(&scene, sensor_pose_world, gen, gen.step);
    let truth = relative_pose(reference_pose, sensor_pose_world);
    let mut rng = guess_rng(gen.seed, gen.step);
    let guess = initial_guess(truth, gen.initial_error, &mut rng);

    let report = match_pair(
        prepare_frame(&reference)?,
        prepare_frame(&sensor)?,
        guess,
        &request.matcher,
        false,
    )?;

    Ok(SessionRecord {
        version: 1,
        generation: gen.clone(),
        reference_mode: request.reference_mode.clone(),
        matcher: request.matcher.clone(),
        reference_angles: reference.angles,
        reference_readings: option_readings(&reference.readings),
        reference_valid: reference.valid,
        sensor_angles: sensor.angles,
        sensor_readings: option_readings(&sensor.readings),
        sensor_valid: sensor.valid,
        initial_guess: guess.to_array(),
        result: SessionResult {
            estimated_pose: report.estimated_pose,
            valid: report.valid,
            termination: report.termination,
            iterations: report.iterations,
            nvalid: report.nvalid,
            error: report.error,
        },
    })
}

fn option_readings(readings: &[f64]) -> Vec<Option<f64>> {
    readings
        .iter()
        .map(|value| value.is_finite().then_some(*value))
        .collect()
}

/// Rerun the matcher on an exported session and compare with its stored result.
pub fn replay_session(record: &SessionRecord) -> Result<ImportResponse, String> {
    if record.version != 1 {
        return Err(format!("unsupported session version {}", record.version));
    }
    let pair = ScanPair {
        format: "csm-rs-scan-pair".to_owned(),
        version: 1,
        reference: ScanInput::Polar {
            angles: record.reference_angles.clone(),
            readings: record.reference_readings.clone(),
            valid: record.reference_valid.clone(),
            sigma: None,
            true_alpha: None,
        },
        sensor: ScanInput::Polar {
            angles: record.sensor_angles.clone(),
            readings: record.sensor_readings.clone(),
            valid: record.sensor_valid.clone(),
            sigma: None,
            true_alpha: None,
        },
        initial_guess: Some(record.initial_guess),
        config: Some(record.matcher.clone()),
    };
    let response = run_import(&pair)?;
    let stored = &record.result;
    if response.valid != stored.valid || response.termination != stored.termination {
        return Err(format!(
            "replay diverged: {} != {}",
            response.termination, stored.termination
        ));
    }
    for (a, b) in response.estimated_pose.iter().zip(stored.estimated_pose) {
        if (a - b).abs() > 1e-9 {
            return Err(format!("replay pose diverged: {a} != {b}"));
        }
    }
    Ok(response)
}
