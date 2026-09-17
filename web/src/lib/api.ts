// Wire types for the Rust HTTP API and the one call the workbench makes.

export interface GenerationConfig {
  scenario: string;
  seed: number;
  motion: number;
  noise: number;
  dropout: number;
  initial_error: number;
  ray_count: number;
  half_span: number;
  overlap: number;
  step: number;
}

export interface MatcherConfig {
  reading_min: number;
  reading_max: number;
  max_angular_deg: number;
  max_linear: number;
  max_iterations: number;
  epsilon_xy: number;
  epsilon_theta: number;
  search: string;
  metric: string;
  max_correspondence_dist: number;
  sigma: number;
  do_alpha_test: boolean;
  alpha_test_threshold_deg: number;
  do_visibility_test: boolean;
  clustering_threshold: number;
  orientation_neighbourhood: number;
  outliers_max_perc: number;
  adaptive_order: number;
  adaptive_mult: number;
  remove_doubles: boolean;
  restart: boolean;
  restart_threshold_mean_error: number;
  restart_dt: number;
  restart_dtheta: number;
  ml_weighting: boolean;
  sigma_weighting: boolean;
  do_compute_covariance: boolean;
  debug_verify_tricks: boolean;
}

export interface RunRequest {
  generation: GenerationConfig;
  matcher: MatcherConfig;
  reference_mode: string;
  trace: boolean;
  request_id: number;
}

export interface PreviewRequest {
  generation: GenerationConfig;
  reference_mode: string;
}

/** A generation preview: scans and poses, with no matcher result. */
export interface PreviewResponse {
  reference: [number, number][];
  sensor_unaligned: [number, number][];
  sensor_true: [number, number][];
  truth_pose: [number, number, number];
  initial_pose: [number, number, number];
  extent: number;
  segments: [[number, number], [number, number]][];
}

export interface TraceCorrespondence {
  sensor_ray: number;
  reference_j1: number;
  reference_j2: number;
  distance: number;
  sensor_point: [number, number];
  reference_point: [number, number];
}

export interface TraceIteration {
  iteration: number;
  pose: [number, number, number];
  error: number;
  valid_correspondences: number;
  restart: boolean;
  correspondences: TraceCorrespondence[];
}

export interface FrameResponse {
  request_id: number;
  scenario: string;
  reference_mode: string;
  step: number;
  truth_pose: [number, number, number];
  initial_pose: [number, number, number];
  estimated_pose: [number, number, number];
  relative_truth_pose: [number, number, number];
  relative_estimated_pose: [number, number, number];
  drift_pose: [number, number, number];
  valid: boolean;
  accepted: boolean;
  termination: string;
  iterations: number;
  nvalid: number;
  error: number;
  covariance_status: string;
  trace: TraceIteration[] | null;
  normal_ms: number;
  instrumented_ms: number;
  reference: [number, number][];
  sensor_unaligned: [number, number][];
  sensor_aligned: [number, number][];
  sensor_true: [number, number][];
  extent: number;
  segments: [[number, number], [number, number]][];
}

/** Library defaults, mirrored from `Params::default()` for a sensible start. */
export const DEFAULT_MATCHER: MatcherConfig = {
  reading_min: 0,
  reading_max: 1000,
  max_angular_deg: 90,
  max_linear: 2,
  max_iterations: 1000,
  epsilon_xy: 0.0001,
  epsilon_theta: 0.0001,
  search: 'tricks',
  metric: 'point_to_line',
  max_correspondence_dist: 2,
  sigma: 0.01,
  do_alpha_test: false,
  alpha_test_threshold_deg: 20,
  do_visibility_test: false,
  clustering_threshold: 0.05,
  orientation_neighbourhood: 3,
  outliers_max_perc: 0.95,
  adaptive_order: 0.7,
  adaptive_mult: 2,
  remove_doubles: true,
  restart: true,
  restart_threshold_mean_error: 0.01,
  restart_dt: 0.01,
  restart_dtheta: (1.5 * Math.PI) / 180,
  ml_weighting: false,
  sigma_weighting: false,
  do_compute_covariance: false,
  debug_verify_tricks: false,
};

export async function runFrame(request: RunRequest): Promise<FrameResponse> {
  const response = await fetch('/api/frame', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(request),
  });
  if (!response.ok) {
    throw new Error((await response.text()) || `request failed (${response.status})`);
  }
  return response.json();
}

export async function previewFrame(request: PreviewRequest): Promise<PreviewResponse> {
  const response = await fetch('/api/preview', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(request),
  });
  if (!response.ok) {
    throw new Error((await response.text()) || `preview failed (${response.status})`);
  }
  return response.json();
}