//! Local browser demo for csm-rs: serves a single page and runs the real Rust
//! matcher for every frame request.

mod config;
mod engine;
mod import;
mod scene;
mod simulation;

use axum::{routing::post, Json, Router};
use csm_rs::Pose;
use serde::{Deserialize, Serialize};
use tower_http::services::ServeDir;

use axum::http::StatusCode;
use config::{CompareRequest, GenerationConfig, PreviewRequest, RunRequest};
use engine::{match_pair, prepare_frame, PairReport, TraceIteration};
pub use import::ImportResponse;
use import::{export_session, replay_session, run_import, ScanPair};
use scene::Scene;
use simulation::{
    accumulated_truth, guess_rng, initial_guess, pose_at, relative_pose, scan_for,
    sensor_pose as sensor_pose_at, ScanFrame, SessionRecord,
};

/// Build the demo router (shared by the server and the workflow tests).
///
/// The API routes are served directly; everything else falls through to the
/// built frontend in `web/dist` (see `web/`).
pub fn app() -> Router {
    Router::new()
        .route("/api/frame", post(frame))
        .route("/api/preview", post(preview))
        .route("/api/compare", post(compare))
        .route("/api/import", post(import))
        .route("/api/replay", post(replay))
        .route("/api/export", post(export))
        .fallback_service(ServeDir::new("web/dist"))
}

async fn import(Json(pair): Json<ScanPair>) -> Result<Json<ImportResponse>, (StatusCode, String)> {
    run_import(&pair)
        .map(Json)
        .map_err(|error| (StatusCode::BAD_REQUEST, error))
}

async fn replay(
    Json(record): Json<SessionRecord>,
) -> Result<Json<ImportResponse>, (StatusCode, String)> {
    replay_session(&record)
        .map(Json)
        .map_err(|error| (StatusCode::BAD_REQUEST, error))
}

async fn export(
    Json(request): Json<RunRequest>,
) -> Result<Json<SessionRecord>, (StatusCode, String)> {
    export_session(&request)
        .map(Json)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error))
}

#[derive(Serialize, Deserialize)]
pub struct FrameResponse {
    pub request_id: u64,
    pub scenario: String,
    pub reference_mode: String,
    pub step: u64,
    /// Total true sensor-to-reference-frame motion.
    pub truth_pose: [f64; 3],
    /// The initial relative guess supplied to the matcher.
    pub initial_pose: [f64; 3],
    /// Estimated total motion (accumulated across frames in previous-frame mode).
    pub estimated_pose: [f64; 3],
    /// True motion for the last matched pair (relative in previous-frame mode).
    pub relative_truth_pose: [f64; 3],
    /// Estimated motion for the last matched pair.
    pub relative_estimated_pose: [f64; 3],
    /// Accumulated estimation error, mapping truth into estimate coordinates.
    pub drift_pose: [f64; 3],
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
    pub reference: Vec<[f64; 2]>,
    pub sensor_unaligned: Vec<[f64; 2]>,
    pub sensor_aligned: Vec<[f64; 2]>,
    pub sensor_true: Vec<[f64; 2]>,
    pub extent: f64,
    pub segments: Vec<[[f64; 2]; 2]>,
}

fn world_points(scan: &ScanFrame, transform: Pose) -> Vec<[f64; 2]> {
    let mut points = Vec::new();
    for i in 0..scan.angles.len() {
        if !scan.valid[i] {
            continue;
        }
        let local = [
            scan.readings[i] * scan.angles[i].cos(),
            scan.readings[i] * scan.angles[i].sin(),
        ];
        points.push(transform.transform_point(local));
    }
    points
}

/// State shared by both reference policies.
struct PolicyState {
    reference: ScanFrame,
    sensor: ScanFrame,
    guess: Pose,
    estimate: Pose,
    total_truth: Pose,
    relative_truth: Pose,
    relative_estimate: Pose,
    report: PairReport,
}

fn pair_guess(gen: &GenerationConfig, truth: Pose, step: u64) -> Pose {
    let mut rng = guess_rng(gen.seed, step);
    initial_guess(truth, gen.initial_error, &mut rng)
}

/// Reference/sensor scans and initial guess for a fixed-reference step.
fn fixed_scans(scene: &Scene, gen: &GenerationConfig) -> (ScanFrame, ScanFrame, Pose, Pose) {
    let reference_pose = pose_at(&gen.scenario, 0, gen.motion);
    let sensor_pose_world = sensor_pose_at(gen, reference_pose, gen.step);
    let reference = scan_for(scene, reference_pose, gen, 0);
    let sensor = scan_for(scene, sensor_pose_world, gen, gen.step);
    let truth = relative_pose(reference_pose, sensor_pose_world);
    let guess = pair_guess(gen, truth, gen.step);
    (reference, sensor, truth, guess)
}

/// Reference/sensor scans and initial guess for one previous-frame step.
fn step_scans(
    scene: &Scene,
    gen: &GenerationConfig,
    step: u64,
) -> (ScanFrame, ScanFrame, Pose, Pose) {
    let previous_pose = pose_at(&gen.scenario, step - 1, gen.motion);
    let current_pose = sensor_pose_at(gen, previous_pose, step);
    let reference = scan_for(scene, previous_pose, gen, step - 1);
    let sensor = scan_for(scene, current_pose, gen, step);
    let truth = relative_pose(previous_pose, current_pose);
    let guess = pair_guess(gen, truth, step);
    (reference, sensor, truth, guess)
}

fn fixed_policy(scene: &Scene, request: &RunRequest) -> Result<PolicyState, String> {
    let gen = &request.generation;
    gen.validate()?;
    let (reference, sensor, truth, guess) = fixed_scans(scene, gen);
    let report = match_pair(
        prepare_frame(&reference)?,
        prepare_frame(&sensor)?,
        guess,
        &request.matcher,
        false,
    )?;
    let relative_estimate = Pose::from_array(report.estimated_pose);
    Ok(PolicyState {
        reference,
        sensor,
        guess,
        estimate: relative_estimate,
        total_truth: truth,
        relative_truth: truth,
        relative_estimate,
        report,
    })
}

fn previous_frame_policy(scene: &Scene, request: &RunRequest) -> Result<PolicyState, String> {
    let gen = &request.generation;
    gen.validate()?;
    let steps = gen.step.max(1);
    let total_truth = accumulated_truth(gen, steps);

    // Incremental: a supplied prior means only this pair is matched, so an
    // already-computed sequence is never rebuilt to inspect one frame.
    if let Some(prior) = request.prior_estimate {
        let (reference, sensor, relative_truth, guess) = step_scans(scene, gen, steps);
        let report = match_pair(
            prepare_frame(&reference)?,
            prepare_frame(&sensor)?,
            guess,
            &request.matcher,
            false,
        )?;
        let relative_estimate = Pose::from_array(report.estimated_pose);
        let prior = Pose::from_array(prior);
        // A rejected match must never advance the trajectory.
        let estimate = if report.accepted {
            prior.compose(relative_estimate)
        } else {
            prior
        };
        return Ok(PolicyState {
            reference,
            sensor,
            guess,
            estimate,
            total_truth,
            relative_truth,
            relative_estimate,
            report,
        });
    }

    let mut accumulated = Pose::IDENTITY;
    let mut state = None;
    for s in 1..=steps {
        let (reference, sensor, relative_truth, guess) = step_scans(scene, gen, s);
        let report = match_pair(
            prepare_frame(&reference)?,
            prepare_frame(&sensor)?,
            guess,
            &request.matcher,
            false,
        )?;
        let relative_estimate = Pose::from_array(report.estimated_pose);
        if report.accepted {
            accumulated = accumulated.compose(relative_estimate);
        }
        state = Some(PolicyState {
            reference,
            sensor,
            guess,
            estimate: accumulated,
            total_truth,
            relative_truth,
            relative_estimate,
            report,
        });
    }
    state.ok_or_else(|| "previous-frame policy requires a step".to_owned())
}

pub fn run_frame(request: &RunRequest) -> Result<FrameResponse, String> {
    let gen = &request.generation;
    let scene = Scene::by_name(&gen.scenario);
    let state = if request.reference_mode == "previous_frame" {
        previous_frame_policy(&scene, request)?
    } else {
        fixed_policy(&scene, request)?
    };
    let total_truth = state.total_truth;
    let drift = total_truth.inverse().compose(state.estimate);
    // Ordinary runtime is the uninstrumented pair match behind the result;
    // the traced duration is only meaningful when tracing was requested.
    let (trace, instrumented_ms) = if request.trace {
        let traced = match_pair(
            prepare_frame(&state.reference)?,
            prepare_frame(&state.sensor)?,
            state.guess,
            &request.matcher,
            true,
        )?;
        (traced.trace, traced.instrumented_ms)
    } else {
        (None, 0.0)
    };
    let report = &state.report;

    Ok(FrameResponse {
        request_id: request.request_id,
        scenario: scene.name.to_owned(),
        reference_mode: request.reference_mode.clone(),
        step: gen.step,
        truth_pose: total_truth.to_array(),
        initial_pose: state.guess.to_array(),
        estimated_pose: state.estimate.to_array(),
        relative_truth_pose: state.relative_truth.to_array(),
        relative_estimated_pose: state.relative_estimate.to_array(),
        drift_pose: drift.to_array(),
        valid: report.valid,
        accepted: report.accepted,
        termination: report.termination.clone(),
        iterations: report.iterations,
        nvalid: report.nvalid,
        error: report.error,
        covariance_status: report.covariance_status.clone(),
        trace,
        normal_ms: report.normal_ms,
        instrumented_ms,
        reference: world_points(&state.reference, Pose::IDENTITY),
        sensor_unaligned: world_points(&state.sensor, state.guess),
        sensor_aligned: world_points(&state.sensor, state.relative_estimate),
        sensor_true: world_points(&state.sensor, state.relative_truth),
        extent: scene.extent,
        segments: scene.segments.iter().map(|s| [s.a, s.b]).collect(),
    })
}

async fn frame(
    Json(request): Json<RunRequest>,
) -> Result<Json<FrameResponse>, (StatusCode, String)> {
    run_frame(&request)
        .map(Json)
        .map_err(|error| (StatusCode::BAD_REQUEST, error))
}

/// A generation preview: the scans and poses a run would use, with no matching.
#[derive(Serialize, Deserialize)]
pub struct PreviewResponse {
    pub reference: Vec<[f64; 2]>,
    pub sensor_unaligned: Vec<[f64; 2]>,
    pub sensor_true: Vec<[f64; 2]>,
    pub truth_pose: [f64; 3],
    pub initial_pose: [f64; 3],
    pub extent: f64,
    pub segments: Vec<[[f64; 2]; 2]>,
}

pub fn run_preview(request: &PreviewRequest) -> Result<PreviewResponse, String> {
    let gen = &request.generation;
    gen.validate()?;
    let scene = Scene::by_name(&gen.scenario);
    let (reference, sensor, truth, guess) = if request.reference_mode == "previous_frame" {
        step_scans(&scene, gen, gen.step.max(1))
    } else {
        fixed_scans(&scene, gen)
    };
    Ok(PreviewResponse {
        reference: world_points(&reference, Pose::IDENTITY),
        sensor_unaligned: world_points(&sensor, guess),
        sensor_true: world_points(&sensor, truth),
        truth_pose: truth.to_array(),
        initial_pose: guess.to_array(),
        extent: scene.extent,
        segments: scene.segments.iter().map(|s| [s.a, s.b]).collect(),
    })
}

async fn preview(
    Json(request): Json<PreviewRequest>,
) -> Result<Json<PreviewResponse>, (StatusCode, String)> {
    run_preview(&request)
        .map(Json)
        .map_err(|error| (StatusCode::BAD_REQUEST, error))
}

/// The scans and poses shared by both sides of an A/B comparison.
#[derive(Serialize, Deserialize)]
pub struct SharedScans {
    pub truth_pose: [f64; 3],
    pub initial_pose: [f64; 3],
    pub reference: Vec<[f64; 2]>,
    pub sensor_unaligned: Vec<[f64; 2]>,
    pub sensor_true: Vec<[f64; 2]>,
    pub extent: f64,
    pub segments: Vec<[[f64; 2]; 2]>,
}

/// One side's outcome: parameters differed, inputs did not.
#[derive(Serialize, Deserialize)]
pub struct CompareSide {
    pub relative_truth_pose: [f64; 3],
    pub estimated_pose: [f64; 3],
    pub sensor_aligned: Vec<[f64; 2]>,
    pub valid: bool,
    pub accepted: bool,
    pub termination: String,
    pub iterations: i32,
    pub nvalid: i32,
    pub error: f64,
    pub covariance_status: String,
    pub normal_ms: f64,
}

#[derive(Serialize, Deserialize)]
pub struct CompareResponse {
    pub request_id: u64,
    pub scenario: String,
    pub reference_mode: String,
    pub step: u64,
    pub shared: SharedScans,
    pub a: CompareSide,
    pub b: CompareSide,
}

fn compare_side(report: PairReport, sensor: &ScanFrame, truth: Pose) -> CompareSide {
    let estimate = Pose::from_array(report.estimated_pose);
    CompareSide {
        relative_truth_pose: truth.to_array(),
        estimated_pose: report.estimated_pose,
        sensor_aligned: world_points(sensor, estimate),
        valid: report.valid,
        accepted: report.accepted,
        termination: report.termination,
        iterations: report.iterations,
        nvalid: report.nvalid,
        error: report.error,
        covariance_status: report.covariance_status,
        normal_ms: report.normal_ms,
    }
}

/// Match two configurations on the *same* generated pair, so the comparison is
/// fair by construction: only the matcher parameters can differ.
pub fn run_compare(request: &CompareRequest) -> Result<CompareResponse, String> {
    let gen = &request.generation;
    gen.validate()?;
    let scene = Scene::by_name(&gen.scenario);
    let (reference, sensor, truth, guess) = if request.reference_mode == "previous_frame" {
        step_scans(&scene, gen, gen.step.max(1))
    } else {
        fixed_scans(&scene, gen)
    };
    let reference_prepared = prepare_frame(&reference)?;
    let sensor_prepared = prepare_frame(&sensor)?;
    let a = match_pair(
        reference_prepared.clone(),
        sensor_prepared.clone(),
        guess,
        &request.matcher_a,
        false,
    )?;
    let b = match_pair(
        reference_prepared,
        sensor_prepared,
        guess,
        &request.matcher_b,
        false,
    )?;

    Ok(CompareResponse {
        request_id: request.request_id,
        scenario: scene.name.to_owned(),
        reference_mode: request.reference_mode.clone(),
        step: gen.step,
        shared: SharedScans {
            truth_pose: truth.to_array(),
            initial_pose: guess.to_array(),
            reference: world_points(&reference, Pose::IDENTITY),
            sensor_unaligned: world_points(&sensor, guess),
            sensor_true: world_points(&sensor, truth),
            extent: scene.extent,
            segments: scene.segments.iter().map(|s| [s.a, s.b]).collect(),
        },
        a: compare_side(a, &sensor, truth),
        b: compare_side(b, &sensor, truth),
    })
}

async fn compare(
    Json(request): Json<CompareRequest>,
) -> Result<Json<CompareResponse>, (StatusCode, String)> {
    run_compare(&request)
        .map(Json)
        .map_err(|error| (StatusCode::BAD_REQUEST, error))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(step: u64, initial_error: f64) -> RunRequest {
        RunRequest {
            generation: GenerationConfig {
                step,
                initial_error,
                ..GenerationConfig::default()
            },
            ..RunRequest::default()
        }
    }

    #[test]
    fn frame_runs_the_real_matcher_and_tracks_truth() {
        let response = run_frame(&request(4, 0.05)).expect("frame runs");
        assert!(response.valid, "a well-separated frame should match");
        assert!(response.reference.len() > 100);
        assert!(!response.sensor_aligned.is_empty());
        let dx = response.truth_pose[0] - response.estimated_pose[0];
        let dy = response.truth_pose[1] - response.estimated_pose[1];
        assert!(dx.hypot(dy) < 0.1, "estimated pose should track truth");
    }

    #[test]
    fn frame_is_reproducible_for_a_seed() {
        let mut request = request(5, 0.05);
        request.generation.seed = 99;
        let first = run_frame(&request).expect("frame runs");
        let second = run_frame(&request).expect("frame runs");
        assert_eq!(first.estimated_pose, second.estimated_pose);
        assert_eq!(first.reference, second.reference);
    }
}
