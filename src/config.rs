//! Runtime configuration.
//!
//! Two concerns are kept apart: [`GenerationConfig`] shapes the synthetic
//! problem, and [`MatcherConfig`] is the single mapping onto the pinned
//! library's [`Params`]. A [`RunRequest`] carries both plus run-only controls.

use csm_rs::{CorrespondenceSearch, DistanceMetric, Params};
use serde::{Deserialize, Serialize};

/// Simulation-only controls. These change the generated inputs, never the
/// matcher.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct GenerationConfig {
    pub scenario: String,
    pub seed: u64,
    /// Motion scale: multiplies the per-step displacement and rotation.
    pub motion: f64,
    /// Standard deviation added to each valid range reading.
    pub noise: f64,
    /// Probability that a ray is dropped (marked invalid).
    pub dropout: f64,
    /// Magnitude of the initial-pose estimate error.
    pub initial_error: f64,
    /// Rays per scan. Higher resolution costs more per match.
    pub ray_count: usize,
    /// Half the angular span of the sensor, in radians.
    pub half_span: f64,
    /// Extra sensor separation along its path, in metres. More separation
    /// means less shared geometry; the achievable overlap is measured, not set.
    pub overlap: f64,
    /// Simulation step index (0 is the reference pose in fixed mode).
    pub step: u64,
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self {
            scenario: "asymmetric_room".to_owned(),
            seed: 7,
            motion: 1.0,
            noise: 0.01,
            dropout: 0.0,
            initial_error: 0.05,
            ray_count: 181,
            half_span: 2.2,
            overlap: 0.0,
            step: 0,
        }
    }
}

impl GenerationConfig {
    /// Validate generation controls before generating any scan.
    pub fn validate(&self) -> Result<(), String> {
        if self.ray_count < 3 {
            return Err("sensor resolution needs at least 3 rays".to_owned());
        }
        if !self.half_span.is_finite() || self.half_span <= 0.0 {
            return Err("field of view must be positive".to_owned());
        }
        if self.half_span > std::f64::consts::PI {
            return Err("field of view cannot exceed a full turn".to_owned());
        }
        if !self.motion.is_finite() || self.motion < 0.0 {
            return Err("motion scale must be zero or more".to_owned());
        }
        if !self.noise.is_finite() || self.noise < 0.0 {
            return Err("noise must be zero or more".to_owned());
        }
        if !self.dropout.is_finite() || !(0.0..=1.0).contains(&self.dropout) {
            return Err("dropout must be between 0 and 1".to_owned());
        }
        if !self.initial_error.is_finite() || self.initial_error < 0.0 {
            return Err("initial guess error must be zero or more".to_owned());
        }
        if !self.overlap.is_finite() || self.overlap < 0.0 {
            return Err("overlap separation must be zero or more".to_owned());
        }
        Ok(())
    }
}

/// A run request: generation controls, matcher configuration, and controls
/// that belong to the run itself (reference policy, tracing, stale guarding).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct RunRequest {
    pub generation: GenerationConfig,
    pub matcher: MatcherConfig,
    /// Reference policy: `"fixed"` or `"previous_frame"`.
    pub reference_mode: String,
    /// Whether to collect per-iteration instrumentation.
    pub trace: bool,
    pub request_id: u64,
}

impl Default for RunRequest {
    fn default() -> Self {
        Self {
            generation: GenerationConfig::default(),
            matcher: MatcherConfig::default(),
            reference_mode: "fixed".to_owned(),
            trace: false,
            request_id: 0,
        }
    }
}

/// A request for an immediate scene preview. Generation only: no matching.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct PreviewRequest {
    pub generation: GenerationConfig,
    pub reference_mode: String,
}

impl Default for PreviewRequest {
    fn default() -> Self {
        Self {
            generation: GenerationConfig::default(),
            reference_mode: "fixed".to_owned(),
        }
    }
}

/// Every public matcher setting of the pinned csm-rs revision.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct MatcherConfig {
    // Reading bounds.
    pub reading_min: f64,
    pub reading_max: f64,
    // Correction limits.
    pub max_angular_deg: f64,
    pub max_linear: f64,
    // Stopping criteria.
    pub max_iterations: i32,
    pub epsilon_xy: f64,
    pub epsilon_theta: f64,
    // Correspondence.
    pub search: String,
    pub metric: String,
    pub max_correspondence_dist: f64,
    pub sigma: f64,
    pub do_alpha_test: bool,
    pub alpha_test_threshold_deg: f64,
    pub do_visibility_test: bool,
    pub clustering_threshold: f64,
    pub orientation_neighbourhood: i32,
    // Outlier rejection.
    pub outliers_max_perc: f64,
    pub adaptive_order: f64,
    pub adaptive_mult: f64,
    pub remove_doubles: bool,
    // Restart shell.
    pub restart: bool,
    pub restart_threshold_mean_error: f64,
    pub restart_dt: f64,
    pub restart_dtheta: f64,
    // Weighting.
    pub ml_weighting: bool,
    pub sigma_weighting: bool,
    // Diagnostics.
    pub do_compute_covariance: bool,
    pub debug_verify_tricks: bool,
}

impl Default for MatcherConfig {
    /// Read defaults from the library so this mapping cannot silently drift
    /// from [`Params::default`].
    fn default() -> Self {
        let p = Params::default();
        Self {
            reading_min: p.reading_bounds.min,
            reading_max: p.reading_bounds.max,
            max_angular_deg: p.correction_limits.max_angular_deg,
            max_linear: p.correction_limits.max_linear,
            max_iterations: p.stopping.max_iterations,
            epsilon_xy: p.stopping.epsilon_xy,
            epsilon_theta: p.stopping.epsilon_theta,
            search: match p.correspondence.search {
                CorrespondenceSearch::Naive => "naive".to_owned(),
                CorrespondenceSearch::Tricks => "tricks".to_owned(),
            },
            metric: match p.correspondence.metric {
                DistanceMetric::PointToPoint => "point_to_point".to_owned(),
                DistanceMetric::PointToLine => "point_to_line".to_owned(),
            },
            max_correspondence_dist: p.correspondence.max_dist,
            sigma: p.correspondence.sigma,
            do_alpha_test: p.correspondence.do_alpha_test,
            alpha_test_threshold_deg: p.correspondence.alpha_test_threshold_deg,
            do_visibility_test: p.correspondence.do_visibility_test,
            clustering_threshold: p.correspondence.clustering_threshold,
            orientation_neighbourhood: p.correspondence.orientation_neighbourhood,
            outliers_max_perc: p.outliers.max_perc,
            adaptive_order: p.outliers.adaptive_order,
            adaptive_mult: p.outliers.adaptive_mult,
            remove_doubles: p.outliers.remove_doubles,
            restart: p.restart.enabled,
            restart_threshold_mean_error: p.restart.threshold_mean_error,
            restart_dt: p.restart.dt,
            restart_dtheta: p.restart.dtheta,
            ml_weighting: p.weights.ml,
            sigma_weighting: p.weights.sigma,
            do_compute_covariance: p.do_compute_covariance,
            debug_verify_tricks: p.debug_verify_tricks,
        }
    }
}

impl MatcherConfig {
    /// The one mapping onto library parameters. Validates before returning, so
    /// generated and imported inputs reject the same configuration identically.
    pub fn to_params(&self) -> Result<Params, String> {
        let mut p = Params::default();
        p.reading_bounds.min = self.reading_min;
        p.reading_bounds.max = self.reading_max;
        p.correction_limits.max_angular_deg = self.max_angular_deg;
        p.correction_limits.max_linear = self.max_linear;
        p.stopping.max_iterations = self.max_iterations;
        p.stopping.epsilon_xy = self.epsilon_xy;
        p.stopping.epsilon_theta = self.epsilon_theta;
        p.correspondence.search = if self.search == "naive" {
            CorrespondenceSearch::Naive
        } else {
            CorrespondenceSearch::Tricks
        };
        p.correspondence.metric = if self.metric == "point_to_point" {
            DistanceMetric::PointToPoint
        } else {
            DistanceMetric::PointToLine
        };
        p.correspondence.max_dist = self.max_correspondence_dist;
        p.correspondence.sigma = self.sigma;
        p.correspondence.do_alpha_test = self.do_alpha_test;
        p.correspondence.alpha_test_threshold_deg = self.alpha_test_threshold_deg;
        p.correspondence.do_visibility_test = self.do_visibility_test;
        p.correspondence.clustering_threshold = self.clustering_threshold;
        p.correspondence.orientation_neighbourhood = self.orientation_neighbourhood;
        p.outliers.max_perc = self.outliers_max_perc;
        p.outliers.adaptive_order = self.adaptive_order;
        p.outliers.adaptive_mult = self.adaptive_mult;
        p.outliers.remove_doubles = self.remove_doubles;
        p.restart.enabled = self.restart;
        p.restart.threshold_mean_error = self.restart_threshold_mean_error;
        p.restart.dt = self.restart_dt;
        p.restart.dtheta = self.restart_dtheta;
        p.weights.ml = self.ml_weighting;
        p.weights.sigma = self.sigma_weighting;
        p.do_compute_covariance = self.do_compute_covariance;
        p.debug_verify_tricks = self.debug_verify_tricks;
        p.validate().map_err(|error| error.to_string())?;
        Ok(p)
    }
}
