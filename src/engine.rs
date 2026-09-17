//! The one experiment-execution path, shared by generated and imported scans.
//!
//! Both data sources build [`PreparedPolarScan`]s and run the real library
//! matcher through the same parameter mapping, validation, and report
//! construction. A failed match is a result, not an error.

use csm_rs::{Matcher, Pose, PreparedMatcher, PreparedPolarScan};
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::config::MatcherConfig;
use crate::simulation::ScanFrame;

/// One traced iteration.
#[derive(Serialize, Deserialize)]
pub struct TraceIteration {
    pub iteration: usize,
    pub pose: [f64; 3],
    pub error: f64,
    pub valid_correspondences: usize,
    pub restart: bool,
    pub correspondences: Vec<TraceCorrespondence>,
}

/// One correspondence in a traced iteration.
#[derive(Serialize, Deserialize)]
pub struct TraceCorrespondence {
    pub sensor_ray: usize,
    pub reference_j1: i32,
    pub reference_j2: i32,
    pub distance: f64,
    pub sensor_point: [f64; 2],
    pub reference_point: [f64; 2],
}

/// Diagnostic summary of one pair match.
#[derive(Serialize, Deserialize)]
pub struct PairReport {
    /// The initial relative guess supplied to the matcher.
    pub initial_pose: [f64; 3],
    /// Estimated relative motion for the pair.
    pub estimated_pose: [f64; 3],
    pub valid: bool,
    /// True only for accepted convergence, never for an unsuccessful candidate.
    pub accepted: bool,
    pub termination: String,
    pub iterations: i32,
    pub nvalid: i32,
    pub error: f64,
    pub covariance_status: String,
    /// Per-iteration instrumentation, present only when tracing is requested.
    pub trace: Option<Vec<TraceIteration>>,
    /// Wall-clock time for an uninstrumented prepared match, in milliseconds.
    pub normal_ms: f64,
    /// Wall-clock time for the instrumented match, in milliseconds.
    pub instrumented_ms: f64,
}

/// Match one prepared pair through the shared path.
///
/// When `trace` is set, an uninstrumented match runs first and its result is
/// reported; a second instrumented run supplies the trace and its own timing,
/// so tracing never changes the reported match.
pub fn match_pair(
    reference: PreparedPolarScan,
    sensor: PreparedPolarScan,
    guess: Pose,
    config: &MatcherConfig,
    trace: bool,
) -> Result<PairReport, String> {
    let matcher = Matcher::new(config.to_params()?).map_err(|error| error.to_string())?;
    let mut workspace =
        PreparedMatcher::new(matcher, reference, sensor).map_err(|error| error.to_string())?;

    let normal_start = Instant::now();
    let outcome = workspace
        .match_once_from(guess)
        .map_err(|error| error.to_string())?;
    let normal_ms = normal_start.elapsed().as_secs_f64() * 1e3;

    let (trace, instrumented_ms) = if trace {
        let start = Instant::now();
        let mut captured = Vec::new();
        workspace
            .match_once_traced(|snapshot| captured.push(convert_iteration(snapshot)))
            .map_err(|error| error.to_string())?;
        (Some(captured), start.elapsed().as_secs_f64() * 1e3)
    } else {
        (None, 0.0)
    };

    Ok(PairReport {
        initial_pose: guess.to_array(),
        estimated_pose: outcome.pose.to_array(),
        valid: outcome.valid,
        accepted: outcome.accepted(),
        termination: format!("{:?}", outcome.termination),
        iterations: outcome.iterations,
        nvalid: outcome.nvalid,
        error: outcome.error,
        covariance_status: format!("{:?}", outcome.covariance_status),
        trace,
        normal_ms,
        instrumented_ms,
    })
}

fn convert_iteration(snapshot: csm_rs::IterationSnapshot) -> TraceIteration {
    TraceIteration {
        iteration: snapshot.iteration,
        pose: snapshot.pose,
        error: snapshot.error,
        valid_correspondences: snapshot.valid_correspondences,
        restart: snapshot.restart,
        correspondences: snapshot
            .correspondences
            .into_iter()
            .map(|corr| TraceCorrespondence {
                sensor_ray: corr.sensor_ray,
                reference_j1: corr.reference_j1,
                reference_j2: corr.reference_j2,
                distance: corr.distance,
                sensor_point: corr.sensor_point,
                reference_point: corr.reference_point,
            })
            .collect(),
    }
}

/// Prepare a generated frame exactly as an imported scan is prepared.
pub fn prepare_frame(scan: &ScanFrame) -> Result<PreparedPolarScan, String> {
    PreparedPolarScan::from_polar(
        scan.angles.clone(),
        scan.readings.clone(),
        scan.valid.clone(),
    )
    .map_err(|error| error.to_string())
}

/// World points of a prepared scan under a transform, valid rays only.
pub fn scan_points(scan: &PreparedPolarScan, transform: Pose) -> Vec<[f64; 2]> {
    let mut points = Vec::new();
    for i in 0..scan.len() {
        if !scan.valid()[i] {
            continue;
        }
        let local = [
            scan.readings()[i] * scan.angles()[i].cos(),
            scan.readings()[i] * scan.angles()[i].sin(),
        ];
        points.push(transform.transform_point(local));
    }
    points
}
