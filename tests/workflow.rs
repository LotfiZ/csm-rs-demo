//! Browser-to-Rust workflow tests: the HTTP handler runs the real matcher and
//! the controls change real inputs.

use axum::body::Body;
use csm_rs_demo::{app, FrameResponse};
use serde_json::json;
use tower::ServiceExt;

async fn post_frame(config: serde_json::Value) -> FrameResponse {
    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/api/frame")
        .header("content-type", "application/json")
        .body(Body::from(config.to_string()))
        .expect("request");
    let response = app().oneshot(request).await.expect("router call");
    assert_eq!(response.status(), 200, "frame endpoint should succeed");
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    serde_json::from_slice(&bytes).expect("frame response json")
}

fn base(step: u64) -> serde_json::Value {
    json!({
        "generation": {
            "scenario": "asymmetric_room",
            "seed": 7,
            "motion": 1.0,
            "noise": 0.01,
            "dropout": 0.0,
            "initial_error": 0.05,
            "step": step
        },
        "matcher": {},
        "reference_mode": "fixed",
        "request_id": step + 1
    })
}

#[tokio::test]
async fn index_is_served() {
    let response = app()
        .oneshot(
            axum::http::Request::builder()
                .uri("/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("csm-rs workbench"));
}

#[tokio::test]
async fn explicit_initial_guess_is_used_verbatim() {
    let mut request = base(4);
    request["generation"]["initial_guess"] = json!([1.25, -0.5, 0.3]);
    let response = post_frame(request).await;
    assert_eq!(response.initial_pose, [1.25, -0.5, 0.3]);
}

#[tokio::test]
async fn controls_change_real_inputs() {
    let baseline = post_frame(base(4)).await;

    let mut moved = base(4);
    moved["generation"]["motion"] = json!(2.0);
    assert_ne!(post_frame(moved).await.truth_pose, baseline.truth_pose);

    let mut noisy = base(4);
    noisy["generation"]["noise"] = json!(0.2);
    assert_ne!(post_frame(noisy).await.reference, baseline.reference);

    let mut guessed = base(4);
    guessed["generation"]["initial_error"] = json!(1.2);
    assert_ne!(
        post_frame(guessed).await.initial_pose,
        baseline.initial_pose
    );

    let mut placed = base(4);
    placed["generation"]["initial_guess"] = json!([0.4, -0.3, 0.05]);
    assert_eq!(post_frame(placed).await.initial_pose, [0.4, -0.3, 0.05]);

    let mut dropped = base(4);
    dropped["generation"]["dropout"] = json!(0.6);
    assert!(
        post_frame(dropped).await.sensor_unaligned.len() < baseline.sensor_unaligned.len(),
        "dropout should remove rays from the actual sensor input"
    );
}

#[tokio::test]
async fn stepping_is_reproducible_for_a_seed() {
    let mut first = Vec::new();
    let mut second = Vec::new();
    for step in 0..6 {
        first.push(post_frame(base(step)).await);
    }
    for step in 0..6 {
        second.push(post_frame(base(step)).await);
    }
    for (a, b) in first.iter().zip(&second) {
        assert_eq!(a.estimated_pose, b.estimated_pose);
        assert_eq!(a.reference, b.reference);
        assert_eq!(a.sensor_unaligned, b.sensor_unaligned);
        assert_eq!(a.termination, b.termination);
    }
}

#[tokio::test]
async fn different_seeds_produce_different_inputs() {
    let mut other = base(4);
    other["generation"]["seed"] = json!(1234);
    let a = post_frame(base(4)).await;
    let b = post_frame(other).await;
    assert_ne!(a.reference, b.reference);
}

#[tokio::test]
async fn response_echoes_request_id_for_stale_guarding() {
    let mut first = base(1);
    first["request_id"] = json!(11);
    let mut second = base(2);
    second["request_id"] = json!(12);
    assert_eq!(post_frame(first).await.request_id, 11);
    assert_eq!(post_frame(second).await.request_id, 12);
}

#[tokio::test]
async fn scenario_changes_geometry_and_reference() {
    let mut corridor = base(3);
    corridor["generation"]["scenario"] = json!("ambiguous_corridor");
    let mut partial = base(3);
    partial["generation"]["scenario"] = json!("partial_overlap");
    assert_ne!(
        post_frame(base(3)).await.reference,
        post_frame(corridor).await.reference
    );
    assert_ne!(
        post_frame(base(3)).await.reference,
        post_frame(partial).await.reference
    );
}

#[tokio::test]
async fn reference_policy_switching_replaces_the_reference() {
    let mut previous = base(6);
    previous["reference_mode"] = json!("previous_frame");
    let fixed = post_frame(base(6)).await;
    let previous = post_frame(previous).await;
    assert_ne!(fixed.reference, previous.reference);
    assert_eq!(previous.reference_mode, "previous_frame");
}

#[tokio::test]
async fn previous_frame_mode_accumulates_drift() {
    let mut early = base(3);
    early["reference_mode"] = json!("previous_frame");
    early["generation"]["initial_error"] = json!(0.3);
    let mut late = base(12);
    late["reference_mode"] = json!("previous_frame");
    late["generation"]["initial_error"] = json!(0.3);

    let early = post_frame(early).await;
    let late = post_frame(late).await;
    let drift = |r: &FrameResponse| r.drift_pose[0].hypot(r.drift_pose[1]);
    assert!(
        drift(&late) > drift(&early),
        "accumulated drift should grow with the number of composed frames: {} vs {}",
        drift(&late),
        drift(&early)
    );
}

#[tokio::test]
async fn strict_configuration_fails_generated_and_imported_alike() {
    // Generated: an impossible correspondence distance must fail.
    let mut strict = base(4);
    strict["matcher"]["max_correspondence_dist"] = json!(0.0001);
    let generated = post_frame(strict).await;
    assert!(!generated.valid);
    assert_eq!(generated.termination, "NoCorrespondences");

    // Imported: the same matcher setting, with a guess far from the truth.
    let mut far = sample_pair();
    far["initial_guess"] = json!([10.0, 10.0, 0.5]);
    far["config"] = json!({ "max_correspondence_dist": 0.0001 });
    let (status, body) = post_raw("/api/import", far).await;
    assert_eq!(status, 200, "{body}");
    let report: csm_rs_demo::ImportResponse = serde_json::from_str(&body).unwrap();
    assert!(!report.valid);
    assert_eq!(report.termination, "NoCorrespondences");

    // The same import at the default distance still matches.
    let mut lenient = sample_pair();
    lenient["initial_guess"] = json!([0.05, 0.0, 0.0]);
    let (status, body) = post_raw("/api/import", lenient).await;
    assert_eq!(status, 200, "{body}");
    let report: csm_rs_demo::ImportResponse = serde_json::from_str(&body).unwrap();
    assert!(report.valid, "{body}");
}

#[tokio::test]
async fn invalid_matcher_configuration_is_rejected_identically() {
    let mut bad = base(4);
    bad["matcher"]["max_iterations"] = json!(-1);
    let (generated_status, generated_body) = post_raw("/api/frame", bad).await;
    assert_eq!(generated_status, 400);

    let mut pair = sample_pair();
    pair["config"] = json!({ "max_iterations": -1 });
    let (import_status, import_body) = post_raw("/api/import", pair).await;
    assert_eq!(import_status, 400);

    assert!(
        generated_body.contains("iteration limit"),
        "{generated_body}"
    );
    assert!(import_body.contains("iteration limit"), "{import_body}");
}

#[tokio::test]
async fn every_matcher_field_is_accepted_through_the_shared_mapping() {
    // A configuration naming every public matcher setting must be accepted and
    // produce a real match for both sources.
    let full = json!({
        "reading_min": 0.0,
        "reading_max": 50.0,
        "max_angular_deg": 45.0,
        "max_linear": 1.0,
        "max_iterations": 50,
        "epsilon_xy": 0.0005,
        "epsilon_theta": 0.0005,
        "search": "naive",
        "metric": "point_to_point",
        "max_correspondence_dist": 2.0,
        "sigma": 0.02,
        "do_alpha_test": false,
        "alpha_test_threshold_deg": 20.0,
        "do_visibility_test": false,
        "clustering_threshold": 0.05,
        "orientation_neighbourhood": 3,
        "outliers_max_perc": 0.9,
        "adaptive_order": 0.7,
        "adaptive_mult": 2.0,
        "remove_doubles": true,
        "restart": false,
        "restart_threshold_mean_error": 0.02,
        "restart_dt": 0.02,
        "restart_dtheta": 0.03,
        "ml_weighting": false,
        "sigma_weighting": false,
        "do_compute_covariance": true,
        "debug_verify_tricks": false
    });

    let mut generated = base(4);
    generated["matcher"] = full.clone();
    let generated = post_frame(generated).await;
    assert!(generated.valid);
    assert_eq!(generated.covariance_status, "Computed");

    let mut pair = sample_pair();
    pair["config"] = full;
    let (status, body) = post_raw("/api/import", pair).await;
    assert_eq!(status, 200, "{body}");
    let report: csm_rs_demo::ImportResponse = serde_json::from_str(&body).unwrap();
    assert!(report.valid);
    assert_eq!(report.covariance_status, "Computed");
}

#[tokio::test]
async fn iteration_tracing_is_opt_in_and_agrees_with_plain_matching() {
    let plain = post_frame(base(4)).await;
    assert!(plain.trace.is_none(), "tracing must be opt-in");
    assert!(
        plain.normal_ms > 0.0,
        "ordinary runtime is reported without tracing"
    );
    assert_eq!(plain.instrumented_ms, 0.0);

    let mut traced_config = base(4);
    traced_config["trace"] = json!(true);
    let traced = post_frame(traced_config).await;

    let trace = traced.trace.as_ref().expect("trace requested");
    assert!(!trace.is_empty(), "trace should contain real iterations");
    assert!(traced.normal_ms.is_finite() && traced.instrumented_ms.is_finite());
    assert!(
        traced.normal_ms > 0.0 && traced.instrumented_ms > 0.0,
        "both timings must be reported separately"
    );
    assert!(
        trace
            .iter()
            .any(|iteration| !iteration.correspondences.is_empty()),
        "traced iterations should include correspondences"
    );
    for iteration in trace {
        assert!(iteration.pose.iter().all(|value| value.is_finite()));
        assert!(iteration.error.is_finite());
        assert_eq!(
            iteration.valid_correspondences,
            iteration.correspondences.len()
        );
    }
    // Instrumentation must not change the match result.
    for (a, b) in traced.estimated_pose.iter().zip(plain.estimated_pose) {
        assert!((a - b).abs() < 1e-9, "traced pose diverged: {a} != {b}");
    }
    assert_eq!(traced.termination, plain.termination);
}

#[tokio::test]
async fn newly_exposed_reading_bounds_change_results() {
    // Default: a clean frame matches.
    assert!(post_frame(base(4)).await.valid);

    // Clipping every reading as out of range leaves no geometry to match.
    let mut clipped = base(4);
    clipped["matcher"]["reading_max"] = json!(0.5);
    assert!(!post_frame(clipped).await.valid);

    // The same setting changes imported results through the shared mapping.
    let mut pair = sample_pair();
    pair["config"] = json!({ "reading_max": 0.5 });
    let (status, body) = post_raw("/api/import", pair).await;
    assert_eq!(status, 200, "{body}");
    let report: csm_rs_demo::ImportResponse = serde_json::from_str(&body).unwrap();
    assert!(!report.valid, "clipped imported scans should not match");
}

#[tokio::test]
async fn generation_resolution_and_field_of_view_change_inputs() {
    let baseline = post_frame(base(4)).await;

    let mut coarse = base(4);
    coarse["generation"]["ray_count"] = json!(91);
    let coarse = post_frame(coarse).await;
    assert!(coarse.reference.len() < baseline.reference.len());
    assert!(coarse.reference.len() > 10);

    let mut narrow = base(4);
    narrow["generation"]["half_span"] = json!(0.6);
    assert_ne!(post_frame(narrow).await.reference, baseline.reference);
}

#[tokio::test]
async fn overlap_separation_changes_shared_geometry() {
    let near = post_frame(base(6)).await;
    let mut far = base(6);
    far["generation"]["overlap"] = json!(2.0);
    let far = post_frame(far).await;
    assert_ne!(near.truth_pose, far.truth_pose);
    assert_ne!(near.sensor_true, far.sensor_true);
}

#[tokio::test]
async fn preview_generates_scans_without_matching() {
    let (status, body) = post_raw(
        "/api/preview",
        json!({ "generation": { "step": 4, "seed": 3 } }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let value: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(value["reference"].as_array().unwrap().len() > 100);
    // A preview carries no matcher result.
    assert!(value.get("estimated_pose").is_none());
    assert!(value.get("termination").is_none());
}

#[tokio::test]
async fn invalid_generation_is_rejected_for_run_and_preview() {
    let mut bad = base(4);
    bad["generation"]["ray_count"] = json!(2);
    let (run_status, run_body) = post_raw("/api/frame", bad).await;
    assert_eq!(run_status, 400);
    assert!(run_body.contains("at least 3"), "{run_body}");

    let (preview_status, preview_body) = post_raw(
        "/api/preview",
        json!({ "generation": { "half_span": 0.0 } }),
    )
    .await;
    assert_eq!(preview_status, 400);
    assert!(preview_body.contains("field of view"), "{preview_body}");
}

#[tokio::test]
async fn a_b_share_inputs_and_differ_only_by_parameters() {
    let request = json!({
        "generation": { "step": 4, "seed": 7 },
        "reference_mode": "fixed",
        "matcher_a": {},
        "matcher_b": { "max_correspondence_dist": 0.0001 }
    });
    let (status, body) = post_raw("/api/compare", request).await;
    assert_eq!(status, 200, "{body}");
    let value: serde_json::Value = serde_json::from_str(&body).unwrap();

    // The scans and poses are shared, not duplicated per side.
    assert!(value["shared"]["reference"].as_array().unwrap().len() > 100);
    assert!(value["a"].get("reference").is_none());
    assert!(value["b"].get("reference").is_none());

    // Same problem, different parameters: only the outcome differs.
    assert_eq!(
        value["a"]["relative_truth_pose"],
        value["b"]["relative_truth_pose"]
    );
    assert_eq!(value["a"]["valid"], true);
    assert_eq!(value["b"]["valid"], false);
    assert_eq!(value["b"]["termination"], "NoCorrespondences");
}

#[tokio::test]
async fn a_b_with_identical_parameters_agree() {
    let request = json!({
        "generation": { "step": 4 },
        "reference_mode": "fixed",
        "matcher_a": {},
        "matcher_b": {}
    });
    let (status, body) = post_raw("/api/compare", request).await;
    assert_eq!(status, 200, "{body}");
    let value: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(value["a"]["estimated_pose"], value["b"]["estimated_pose"]);
    assert_eq!(value["a"]["termination"], value["b"]["termination"]);
    assert_eq!(value["a"]["sensor_aligned"], value["b"]["sensor_aligned"]);
}

#[tokio::test]
async fn sequence_step_uses_supplied_prior_without_recomputing() {
    // A step that is given its prior accumulation must use it, not rebuild the
    // whole prefix from frame 1.
    let mut step_body = base(2);
    step_body["reference_mode"] = json!("previous_frame");
    step_body["prior_estimate"] = json!([5.0, -3.0, 0.4]);
    let step = post_frame(step_body).await;

    let mut full_body = base(2);
    full_body["reference_mode"] = json!("previous_frame");
    let full = post_frame(full_body).await;

    assert!(step.accepted, "{}", step.termination);
    // Prior is honoured, so the accumulated result differs from a full rebuild.
    assert_ne!(step.estimated_pose, full.estimated_pose);
    // But the pair itself is unchanged: only the accumulation differs.
    for (a, b) in step
        .relative_estimated_pose
        .iter()
        .zip(full.relative_estimated_pose)
    {
        assert!((a - b).abs() < 1e-9, "pair estimate changed: {a} != {b}");
    }
}

#[tokio::test]
async fn rejected_sequence_update_does_not_advance_the_trajectory() {
    let mut body = base(2);
    body["reference_mode"] = json!("previous_frame");
    body["prior_estimate"] = json!([1.0, 2.0, 0.3]);
    body["matcher"]["max_correspondence_dist"] = json!(0.0001);
    let response = post_frame(body).await;
    assert!(!response.accepted);
    assert_eq!(
        response.estimated_pose,
        [1.0, 2.0, 0.3],
        "a rejected match must not move the accumulated estimate"
    );
}

#[tokio::test]
async fn uncertainty_is_only_reported_when_requested_and_computed() {
    let plain = post_frame(base(4)).await;
    assert_eq!(plain.covariance_status, "Disabled");
    assert!(
        plain.covariance.is_none(),
        "uncertainty must not be fabricated when not requested"
    );

    let mut body = base(4);
    body["matcher"]["do_compute_covariance"] = json!(true);
    let with_covariance = post_frame(body).await;
    assert_eq!(with_covariance.covariance_status, "Computed");
    let diagonal = with_covariance.covariance.expect("covariance diagonal");
    assert!(diagonal
        .iter()
        .all(|value| value.is_finite() && *value >= 0.0));
}

#[tokio::test]
async fn benchmark_reports_samples_and_separates_preparation() {
    let body = json!({
        "generation": { "step": 4 },
        "reference_mode": "fixed",
        "matcher_a": {},
        "matcher_b": { "metric": "point_to_point" },
        "warmup": 2,
        "samples": 4
    });
    let (status, text) = post_raw("/api/benchmark", body).await;
    assert_eq!(status, 200, "{text}");
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["samples"], 4);
    assert_eq!(value["warmup"], 2);

    for side in ["a", "b"] {
        assert_eq!(value[side]["samples"], 4);
        assert!(value[side]["prepare_ms"].as_f64().unwrap() >= 0.0);
        let times = value[side]["match_ms"].as_array().unwrap();
        assert_eq!(times.len(), 4);
        for time in times {
            let millis = time.as_f64().unwrap();
            assert!(millis.is_finite() && millis >= 0.0);
        }
        assert!(value[side]["median_ms"].as_f64().unwrap() >= 0.0);
        // Measurement is uninstrumented: no trace is ever returned.
        assert!(value[side].get("trace").is_none());
    }
    // Both sides measured the same generated problem.
    assert!(value["shared"]["reference"].as_array().unwrap().len() > 100);
}

async fn post_raw(path: &str, body: serde_json::Value) -> (u16, String) {
    let request = axum::http::Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("request");
    let response = app().oneshot(request).await.expect("router call");
    let status = response.status().as_u16();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

async fn post_raw_router(
    router: axum::Router,
    path: &str,
    body: serde_json::Value,
) -> (u16, String) {
    let request = axum::http::Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("request");
    let response = router.oneshot(request).await.expect("router call");
    let status = response.status().as_u16();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

async fn get_raw(router: axum::Router, path: &str) -> (u16, String) {
    let request = axum::http::Request::builder()
        .method("GET")
        .uri(path)
        .body(Body::empty())
        .expect("request");
    let response = router.oneshot(request).await.expect("router call");
    let status = response.status().as_u16();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("csm-demo-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[tokio::test]
async fn experiments_persist_across_instances_with_a_saved_snapshot() {
    let dir = temp_dir("persist");
    let router = csm_rs_demo::app_with_data_dir(dir.clone());
    let body = json!({
        "name": "easy_alignment",
        "run": { "generation": { "step": 4, "seed": 7 }, "matcher": {}, "reference_mode": "fixed", "request_id": 1 }
    });
    let (status, text) = post_raw_router(router, "/api/experiments", body).await;
    assert_eq!(status, 200, "{text}");
    let saved: serde_json::Value = serde_json::from_str(&text).unwrap();
    let snapshot = saved["session"]["result"]["estimated_pose"].clone();
    assert!(!snapshot.is_null());
    assert_eq!(saved["versions"]["generator"], "1");
    assert!(!saved["session"]["reference_angles"]
        .as_array()
        .unwrap()
        .is_empty());

    // A fresh router instance stands in for an application restart.
    let restarted = csm_rs_demo::app_with_data_dir(dir.clone());
    let (status, text) = get_raw(restarted.clone(), "/api/experiments").await;
    assert_eq!(status, 200, "{text}");
    let list: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(list["names"]
        .as_array()
        .unwrap()
        .iter()
        .any(|name| name == "easy_alignment"));

    let (status, text) = get_raw(restarted.clone(), "/api/experiments/easy_alignment").await;
    assert_eq!(status, 200, "{text}");
    let loaded: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        loaded["document"]["session"]["result"]["estimated_pose"],
        snapshot
    );

    // A rerun is produced separately from the stored observation.
    let (status, text) = post_raw_router(
        restarted,
        "/api/replay",
        loaded["document"]["session"].clone(),
    )
    .await;
    assert_eq!(status, 200, "{text}");
    let rerun: serde_json::Value = serde_json::from_str(&text).unwrap();
    let rerun_pose = rerun["estimated_pose"].as_array().unwrap();
    let snapshot_pose = snapshot.as_array().unwrap();
    for (a, b) in rerun_pose.iter().zip(snapshot_pose) {
        let (a, b) = (a.as_f64().unwrap(), b.as_f64().unwrap());
        assert!((a - b).abs() < 1e-9, "rerun {a} != snapshot {b}");
    }
    assert!(rerun.get("session").is_none());

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn experiment_names_reject_path_traversal() {
    let dir = temp_dir("traversal");
    let router = csm_rs_demo::app_with_data_dir(dir.clone());
    let body = json!({
        "name": "../escape",
        "run": { "generation": { "step": 1 } }
    });
    let (status, text) = post_raw_router(router, "/api/experiments", body).await;
    assert_eq!(status, 400, "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn portable_experiment_round_trips_and_warns_on_version_mismatch() {
    let dir = temp_dir("portable");
    let router = csm_rs_demo::app_with_data_dir(dir.clone());
    let body = json!({
        "name": "portable",
        "run": { "generation": { "step": 4, "seed": 7 }, "matcher": {}, "reference_mode": "fixed", "request_id": 1 }
    });
    let (status, text) = post_raw_router(router.clone(), "/api/experiments", body).await;
    assert_eq!(status, 200, "{text}");
    let document: serde_json::Value = serde_json::from_str(&text).unwrap();
    // Portable documents stay compact: no bulky traces are stored.
    assert!(document["session"].get("trace").is_none());

    // Round-trip: importing the same document validates with no warnings.
    let (status, text) =
        post_raw_router(router.clone(), "/api/experiments/import", document.clone()).await;
    assert_eq!(status, 200, "{text}");
    let outcome: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(outcome["warnings"].as_array().unwrap().is_empty());
    assert_eq!(outcome["document"]["name"], "portable");

    // Unsupported versions are rejected clearly.
    let mut future = document.clone();
    future["version"] = json!(99);
    let (status, text) = post_raw_router(router.clone(), "/api/experiments/import", future).await;
    assert_eq!(status, 400);
    assert!(text.contains("version"), "{text}");

    // A library mismatch is a warning, not a rejection.
    let mut mismatched = document;
    mismatched["versions"]["library"] = json!("0000000000000000000000000000000000000000");
    let (status, text) = post_raw_router(router, "/api/experiments/import", mismatched).await;
    assert_eq!(status, 200, "{text}");
    let outcome: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(outcome["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|warning| warning.as_str().unwrap().contains("csm-rs")));

    let _ = std::fs::remove_dir_all(&dir);
}

fn sample_pair() -> serde_json::Value {
    let angles: Vec<f64> = (0..61).map(|i| -1.5 + i as f64 * 0.05).collect();
    let readings: Vec<f64> = angles.iter().map(|a| 6.0 + 0.5 * (3.0 * a).sin()).collect();
    json!({
        "format": "csm-rs-scan-pair",
        "version": 1,
        "initial_guess": [0.0, 0.0, 0.0],
        "reference": { "kind": "polar", "angles": angles, "readings": readings, "valid": vec![true; 61] },
        "sensor": { "kind": "polar", "angles": angles, "readings": readings, "valid": vec![true; 61] },
    })
}

#[tokio::test]
async fn imports_a_valid_polar_pair_without_ground_truth() {
    let (status, body) = post_raw("/api/import", sample_pair()).await;
    assert_eq!(status, 200, "{body}");
    let response: csm_rs_demo::ImportResponse = serde_json::from_str(&body).unwrap();
    assert!(response.valid);
    assert!(!response.reference.is_empty());
    // No ground-truth field exists on the response at all.
    let value: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(value.get("truth_pose").is_none());
}

#[tokio::test]
async fn imports_a_cartesian_pair() {
    let angles: Vec<f64> = (0..61).map(|i| -1.5 + i as f64 * 0.05).collect();
    let points: Vec<[f64; 2]> = angles
        .iter()
        .map(|a| {
            let r = 6.0 + 0.5 * (3.0 * a).sin();
            [r * a.cos(), r * a.sin()]
        })
        .collect();
    let pair = json!({
        "format": "csm-rs-scan-pair",
        "version": 1,
        "reference": { "kind": "cartesian", "points": points, "valid": vec![true; 61] },
        "sensor": { "kind": "cartesian", "points": points, "valid": vec![true; 61] },
    });
    let (status, body) = post_raw("/api/import", pair).await;
    assert_eq!(status, 200, "{body}");
}

#[tokio::test]
async fn malformed_imports_are_rejected_clearly() {
    let mut mismatched = sample_pair();
    mismatched["sensor"]["readings"] = json!(vec![1.0; 60]);
    let (status, body) = post_raw("/api/import", mismatched).await;
    assert_eq!(status, 400);
    assert!(body.to_lowercase().contains("length"), "{body}");

    // A null reading on a valid ray is malformed input.
    let mut invalid_missing = sample_pair();
    invalid_missing["sensor"]["readings"][3] = serde_json::Value::Null;
    let (status, body) = post_raw("/api/import", invalid_missing).await;
    assert_eq!(status, 400);
    assert!(body.to_lowercase().contains("missing"), "{body}");

    // Missing returns (null + valid=false) are accepted.
    let mut missing = sample_pair();
    missing["sensor"]["readings"][3] = serde_json::Value::Null;
    missing["sensor"]["valid"][3] = json!(false);
    let (status, _) = post_raw("/api/import", missing).await;
    assert_eq!(status, 200);

    let mut unsupported = sample_pair();
    unsupported["version"] = json!(2);
    let (status, body) = post_raw("/api/import", unsupported).await;
    assert_eq!(status, 400);
    assert!(body.contains("version"), "{body}");
}

#[tokio::test]
async fn export_replay_round_trips() {
    let (status, body) = post_raw(
        "/api/export",
        json!({ "generation": { "step": 5, "seed": 42 } }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let record: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(record["version"], 1);
    assert!(record["result"]["estimated_pose"].is_array());

    let (status, replay_body) = post_raw("/api/replay", record.clone()).await;
    assert_eq!(status, 200, "{replay_body}");
    let replay: serde_json::Value = serde_json::from_str(&replay_body).unwrap();
    let stored = &record["result"]["estimated_pose"];
    for i in 0..3 {
        let delta =
            (replay["estimated_pose"][i].as_f64().unwrap() - stored[i].as_f64().unwrap()).abs();
        assert!(delta < 1e-9, "replay diverged at {i}: {delta}");
    }

    // Unsupported versions are rejected clearly.
    let mut future = record;
    future["version"] = json!(2);
    let (status, body) = post_raw("/api/replay", future).await;
    assert_eq!(status, 400);
    assert!(body.contains("version"), "{body}");
}
