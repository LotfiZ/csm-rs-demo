import { DEFAULT_MATCHER, type MatcherConfig } from './api';

export type FieldKey = keyof MatcherConfig;

export interface FieldSpec {
  key: FieldKey;
  label: string;
  kind: 'number' | 'select' | 'toggle';
  unit?: string;
  help: string;
  /** Shown as guidance when a setting only matters under a condition. */
  depends?: string;
  min?: number;
  max?: number;
  step?: number;
  options?: { value: string; label: string }[];
}

export interface FieldGroup {
  id: string;
  title: string;
  /** Advanced groups start collapsed. */
  open: boolean;
  fields: FieldSpec[];
}

export const MATCHER_GROUPS: FieldGroup[] = [
  {
    id: 'correspondence',
    title: 'Correspondence',
    open: true,
    fields: [
      {
        key: 'search',
        label: 'Search strategy',
        kind: 'select',
        help: 'How neighbouring reference rays are found.',
        options: [
          { value: 'tricks', label: 'Tricks (jump table)' },
          { value: 'naive', label: 'Naive (full scan)' },
        ],
      },
      {
        key: 'metric',
        label: 'Distance metric',
        kind: 'select',
        help: 'What the solver minimises.',
        options: [
          { value: 'point_to_line', label: 'Point to line (PlICP)' },
          { value: 'point_to_point', label: 'Point to point' },
        ],
      },
      {
        key: 'max_correspondence_dist',
        label: 'Max correspondence distance',
        kind: 'number',
        unit: 'm',
        min: 0,
        step: 0.1,
        help: 'Pairs farther apart than this are not matched; too small fails early.',
      },
      {
        key: 'sigma',
        label: 'Assumed noise sigma',
        kind: 'number',
        unit: 'm',
        min: 0,
        step: 0.005,
        help: 'Noise used by the weighting paths.',
      },
    ],
  },
  {
    id: 'stopping',
    title: 'Stopping',
    open: true,
    fields: [
      {
        key: 'max_iterations',
        label: 'Max iterations',
        kind: 'number',
        min: 0,
        step: 1,
        help: 'Hard cap on ICP iterations.',
      },
      {
        key: 'epsilon_xy',
        label: 'Translation convergence',
        kind: 'number',
        unit: 'm',
        min: 0,
        step: 0.0001,
        help: 'Stop once the per-iteration translation change falls below this.',
      },
      {
        key: 'epsilon_theta',
        label: 'Rotation convergence',
        kind: 'number',
        unit: 'rad',
        min: 0,
        step: 0.0001,
        help: 'Stop once the per-iteration rotation change falls below this.',
      },
    ],
  },
  {
    id: 'outliers',
    title: 'Outliers',
    open: true,
    fields: [
      {
        key: 'outliers_max_perc',
        label: 'Kept correspondence fraction',
        kind: 'number',
        min: 0,
        max: 1,
        step: 0.01,
        help: 'Highest-error correspondences beyond this fraction are discarded.',
      },
      {
        key: 'remove_doubles',
        label: 'Remove duplicate correspondences',
        kind: 'toggle',
        help: 'Forbid two sensor rays matching the same reference point.',
      },
      {
        key: 'adaptive_order',
        label: 'Adaptive order',
        kind: 'number',
        min: 0,
        max: 1,
        step: 0.05,
        help: 'Percentile used for the adaptive error threshold.',
      },
      {
        key: 'adaptive_mult',
        label: 'Adaptive multiplier',
        kind: 'number',
        min: 0,
        step: 0.1,
        help: 'Multiplier applied to the percentile error.',
      },
    ],
  },
  {
    id: 'filters',
    title: 'Correspondence filters (advanced)',
    open: false,
    fields: [
      {
        key: 'do_alpha_test',
        label: 'Alpha orientation test',
        kind: 'toggle',
        help: 'Require surface orientations to agree before pairing.',
      },
      {
        key: 'alpha_test_threshold_deg',
        label: 'Alpha threshold',
        kind: 'number',
        unit: '°',
        min: 0,
        step: 1,
        depends: 'Only used when the alpha orientation test is on.',
        help: 'Orientation difference allowed between paired surfaces.',
      },
      {
        key: 'do_visibility_test',
        label: 'Visibility test',
        kind: 'toggle',
        help: 'Reject pairs hidden behind other geometry.',
      },
      {
        key: 'clustering_threshold',
        label: 'Clustering threshold',
        kind: 'number',
        unit: 'm',
        min: 0,
        step: 0.01,
        help: 'Distance under which adjacent rays are treated as one surface.',
      },
      {
        key: 'orientation_neighbourhood',
        label: 'Orientation neighbourhood',
        kind: 'number',
        min: 1,
        step: 1,
        help: 'Rays on each side used to estimate surface orientation.',
      },
    ],
  },
  {
    id: 'correction',
    title: 'Correction limits (advanced)',
    open: false,
    fields: [
      {
        key: 'max_angular_deg',
        label: 'Max rotation per iteration',
        kind: 'number',
        unit: '°',
        min: 0,
        step: 1,
        help: 'Caps how far one iteration may rotate.',
      },
      {
        key: 'max_linear',
        label: 'Max translation per iteration',
        kind: 'number',
        unit: 'm',
        min: 0,
        step: 0.1,
        help: 'Caps how far one iteration may translate.',
      },
    ],
  },
  {
    id: 'reading',
    title: 'Reading bounds (advanced)',
    open: false,
    fields: [
      {
        key: 'reading_min',
        label: 'Minimum reading',
        kind: 'number',
        unit: 'm',
        min: 0,
        step: 0.01,
        help: 'Readings below this are treated as invalid.',
      },
      {
        key: 'reading_max',
        label: 'Maximum reading',
        kind: 'number',
        unit: 'm',
        min: 0,
        step: 1,
        help: 'Readings above this are treated as invalid.',
      },
    ],
  },
  {
    id: 'restart',
    title: 'Restart shell (advanced)',
    open: false,
    fields: [
      {
        key: 'restart',
        label: 'Enable restart shell',
        kind: 'toggle',
        help: 'Retry from small perturbations when the fit stalls.',
      },
      {
        key: 'restart_threshold_mean_error',
        label: 'Restart mean-error threshold',
        kind: 'number',
        unit: 'm',
        min: 0,
        step: 0.005,
        depends: 'Only used when the restart shell is on.',
        help: 'Mean correspondence error above which a restart is attempted.',
      },
      {
        key: 'restart_dt',
        label: 'Restart translation step',
        kind: 'number',
        unit: 'm',
        min: 0,
        step: 0.005,
        depends: 'Only used when the restart shell is on.',
        help: 'Perturbation distance for restarts.',
      },
      {
        key: 'restart_dtheta',
        label: 'Restart rotation step',
        kind: 'number',
        unit: 'rad',
        min: 0,
        step: 0.001,
        depends: 'Only used when the restart shell is on.',
        help: 'Perturbation angle for restarts (library default ≈ 1.5°).',
      },
    ],
  },
  {
    id: 'weighting',
    title: 'Weighting (advanced)',
    open: false,
    fields: [
      {
        key: 'ml_weighting',
        label: 'Maximum-likelihood weighting',
        kind: 'toggle',
        help: 'Weight correspondences by an ML model.',
      },
      {
        key: 'sigma_weighting',
        label: 'Per-ray sigma weighting',
        kind: 'toggle',
        help: 'Weight by each ray’s reported sigma; needs per-ray sigma input.',
      },
    ],
  },
  {
    id: 'diagnostics',
    title: 'Diagnostics (advanced)',
    open: false,
    fields: [
      {
        key: 'do_compute_covariance',
        label: 'Compute covariance',
        kind: 'toggle',
        help: 'Produce pose uncertainty and derivative outputs.',
      },
      {
        key: 'debug_verify_tricks',
        label: 'Verify tricks against naive',
        kind: 'toggle',
        help: 'Run both searches as a debug check; slower.',
      },
    ],
  },
];

export interface Preset {
  id: string;
  name: string;
  overrides: Partial<MatcherConfig>;
}

export const PRESETS: Preset[] = [
  { id: 'defaults', name: 'Library defaults', overrides: {} },
  { id: 'plicp', name: 'Point to line (PlICP)', overrides: { metric: 'point_to_line' } },
  { id: 'p2p', name: 'Point to point', overrides: { metric: 'point_to_point' } },
  { id: 'fast', name: 'Fast (no restarts)', overrides: { restart: false } },
  {
    id: 'strict',
    name: 'Strict filtering',
    overrides: { outliers_max_perc: 0.7, do_alpha_test: true, do_visibility_test: true },
  },
  { id: 'uncertainty', name: 'With covariance', overrides: { do_compute_covariance: true } },
];

/** Mirrors the library's `Params::validate`, in plain language. */
export function matcherIssues(m: MatcherConfig): string[] {
  const issues: string[] = [];
  if (m.reading_min < 0 || !(m.reading_max > m.reading_min)) {
    issues.push('Reading bounds must satisfy 0 ≤ minimum < maximum.');
  }
  if (m.max_correspondence_dist <= 0) {
    issues.push('Max correspondence distance must be greater than 0.');
  }
  if (m.sigma <= 0) {
    issues.push('Assumed noise sigma must be greater than 0.');
  }
  if (m.max_iterations < 0) {
    issues.push('Max iterations must be 0 or more.');
  }
  if (m.epsilon_xy < 0 || m.epsilon_theta < 0) {
    issues.push('Convergence thresholds must be 0 or more.');
  }
  if (m.max_angular_deg < 0 || m.max_linear < 0) {
    issues.push('Correction limits must be 0 or more.');
  }
  if (m.alpha_test_threshold_deg < 0) {
    issues.push('Alpha threshold must be 0 or more.');
  }
  if (m.clustering_threshold < 0) {
    issues.push('Clustering threshold must be 0 or more.');
  }
  if (m.orientation_neighbourhood < 1) {
    issues.push('Orientation neighbourhood must be at least 1.');
  }
  if (m.outliers_max_perc < 0 || m.outliers_max_perc > 1) {
    issues.push('Kept correspondence fraction must be between 0 and 1.');
  }
  if (m.adaptive_order < 0) {
    issues.push('Adaptive order must be 0 or more.');
  }
  if (m.adaptive_mult <= 0) {
    issues.push('Adaptive multiplier must be greater than 0.');
  }
  if (m.restart_threshold_mean_error < 0 || m.restart_dt < 0 || m.restart_dtheta < 0) {
    issues.push('Restart settings must be 0 or more.');
  }
  return issues;
}

export function applyPreset(preset: Preset): MatcherConfig {
  return { ...DEFAULT_MATCHER, ...preset.overrides };
}