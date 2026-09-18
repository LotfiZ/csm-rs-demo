import { derived, get, writable } from 'svelte/store';
import {
  DEFAULT_MATCHER,
  benchmarkFrame,
  compareFrame,
  importScanPair,
  importExperiment,
  listExperiments,
  loadExperiment,
  previewFrame,
  replaySession,
  runFrame,
  saveExperiment,
  type BenchmarkResponse,
  type CompareResponse,
  type ExperimentDocument,
  type FrameResponse,
  type GenerationConfig,
  type ImportResponse,
  type MatcherConfig,
  type PreviewRequest,
  type PreviewResponse,
  type RunRequest,
} from './api';
import { exampleById, EXAMPLE_LIST } from './examples';
import { matcherIssues } from './matcherFields';

export type Theme = 'light' | 'dark';
export type Side = 'A' | 'B';

export const exampleId = writable(EXAMPLE_LIST[0].id);
export const generation = writable<GenerationConfig>({ ...EXAMPLE_LIST[0].generation });
export const referenceMode = writable('fixed');
export const trace = writable(false);

export const abMode = writable(false);
/** Step through one matcher iteration at a time. */
export const stepMode = writable(false);
export const matcher = writable<MatcherConfig>({ ...DEFAULT_MATCHER });
export const matcherB = writable<MatcherConfig>({ ...DEFAULT_MATCHER });
export const editingSide = writable<Side>('A');
export const matcherPreset = writable('');

/** Plain-language validation mirroring the library, for both sides in A/B. */
export const issues = derived([matcher, matcherB, abMode], ([$a, $b, $ab]) => {
  if (!$ab) return matcherIssues($a);
  return [
    ...matcherIssues($a).map((message) => `A: ${message}`),
    ...matcherIssues($b).map((message) => `B: ${message}`),
  ];
});

export const result = writable<FrameResponse | null>(null);
export const compare = writable<CompareResponse | null>(null);
export const benchmark = writable<BenchmarkResponse | null>(null);
export const benchmarking = writable(false);
export const preview = writable<PreviewResponse | null>(null);
export const running = writable(false);
export const error = writable<string | null>(null);

/** Number of matcher iterations requested by the step-through mode. */
export const stepTarget = writable(0);

export const importMode = writable(false);
export const imported = writable<ImportResponse | null>(null);

export const experiments = writable<string[]>([]);
export const loadedExperiment = writable<ExperimentDocument | null>(null);
export const experimentWarnings = writable<string[]>([]);
export const rerunResult = writable<ImportResponse | null>(null);

/** Monotonic id; the response carries it back so stale replies can be dropped. */
const requestId = writable(0);
/** The exact inputs behind the displayed result, for outdated detection. */
const resultKey = writable<string | null>(null);
/** True after a completed run has been invalidated by an input change. */
export const outdated = writable(false);

export const layers = writable({
  reference: true,
  scan: true,
  candidateB: true,
  truth: true,
  correspondences: false,
});

export const diagnosticsOpen = writable(true);
export const resultDetailsOpen = writable(true);
/** Selected iteration in the trace, for stepping and overlay. */
export const traceIteration = writable(0);

export interface Layout {
  left: number;
  right: number;
  bottom: number;
  leftCollapsed: boolean;
  rightCollapsed: boolean;
}

export const layout = writable<Layout>({
  left: 264,
  right: 264,
  bottom: 340,
  leftCollapsed: false,
  rightCollapsed: false,
});

export function initLayout() {
  const stored = localStorage.getItem('csm-layout');
  if (!stored) return;
  try {
    layout.update((current) => ({ ...current, ...(JSON.parse(stored) as Partial<Layout>) }));
  } catch {
    // Ignore malformed layout state.
  }
}

layout.subscribe((value) => {
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem('csm-layout', JSON.stringify(value));
  }
});

function activeKey(): string {
  return JSON.stringify([
    get(generation),
    get(matcher),
    get(referenceMode),
    get(trace),
    get(abMode),
    get(stepMode),
    get(abMode) ? get(matcherB) : null,
  ]);
}

/** The frame whose pair diagnostics are shown. */
export const activeFrame = derived(result, ($result) => $result);

/** What the plot draws: the live result when current, otherwise a preview. */
export interface ViewData {
  reference: [number, number][];
  sensor_unaligned: [number, number][];
  sensor_true?: [number, number][];
  sensor_aligned?: [number, number][];
  estimated_pose?: [number, number, number];
  sensor_aligned_b?: [number, number][];
  estimated_pose_b?: [number, number, number];
  truth_pose?: [number, number, number];
  initial_pose: [number, number, number];
  extent: number;
  segments: [[number, number], [number, number]][];
  trajectory_true?: [number, number][];
  trajectory_estimate?: [number, number][];
  rejected?: boolean;
  hasTruth?: boolean;
}

export const view = derived(
  [
    result,
    compare,
    preview,
    outdated,
    abMode,
    stepMode,
    traceIteration,
    importMode,
    imported,
  ],
  (
    [
      $result,
      $compare,
      $preview,
      $outdated,
      $ab,
      $stepMode,
      $traceIteration,
      $importMode,
      $imported,
    ],
  ): ViewData | null => {
    if ($importMode && $imported) {
      return {
        reference: $imported.reference,
        sensor_unaligned: $imported.sensor_unaligned,
        sensor_aligned: $imported.sensor_aligned,
        estimated_pose: $imported.estimated_pose,
        initial_pose: $imported.initial_pose,
        extent: $imported.extent,
        segments: [],
        hasTruth: false,
      };
    }
    if ($ab && $compare && !$outdated) {
      return {
        reference: $compare.shared.reference,
        sensor_unaligned: $compare.shared.sensor_unaligned,
        sensor_true: $compare.shared.sensor_true,
        truth_pose: $compare.shared.truth_pose,
        initial_pose: $compare.shared.initial_pose,
        extent: $compare.shared.extent,
        segments: $compare.shared.segments,
        sensor_aligned: $compare.a.sensor_aligned,
        estimated_pose: $compare.a.estimated_pose,
        sensor_aligned_b: $compare.b.sensor_aligned,
        estimated_pose_b: $compare.b.estimated_pose,
        hasTruth: true,
      };
    }
    if (!$ab && $result && !$outdated) {
      const trace = $result.trace;
      const index = Math.min($traceIteration, Math.max(0, (trace?.length ?? 1) - 1));
      const iteration = $stepMode && trace && trace.length > 0 ? trace[index] : null;
      return iteration
        ? { ...($result as ViewData), estimated_pose: iteration.pose }
        : ($result as ViewData);
    }
    return ($preview as ViewData | null) ?? ($result as ViewData | null);
  },
);

/** Remove every visual and metric from a run before a new input is shown. */
function invalidateResults(clearScene: boolean) {
  const hasRun =
    get(resultKey) !== null || get(result) !== null || get(compare) !== null || get(imported) !== null;
  if (!hasRun) {
    if (clearScene) preview.set(null);
    return;
  }
  result.set(null);
  compare.set(null);
  benchmark.set(null);
  importMode.set(false);
  imported.set(null);
  resultKey.set(null);
  stepTarget.set(0);
  traceIteration.set(0);
  if (clearScene) preview.set(null);
  outdated.set(true);
}

export function loadExample(id: string) {
  invalidateResults(true);
  const example = exampleById(id);
  exampleId.set(example.id);
  generation.set({ ...example.generation, initial_guess: null });
  importMode.set(false);
  imported.set(null);
  placed.set(false);
  placing.set(false);
  stepTarget.set(0);
  traceIteration.set(0);
  outdated.set(false);
}

/** True while the scan can be dragged to set the initial guess by hand. */
export const placing = writable(false);
/** True once the scan has been placed by hand, so the UI can offer a reset. */
export const placed = writable(false);

export function setPlacing(value: boolean) {
  placing.set(value);
}

function movePoint(
  point: [number, number],
  base: [number, number, number],
  pose: [number, number, number],
): [number, number] {
  const dtheta = pose[2] - base[2];
  const cos = Math.cos(dtheta);
  const sin = Math.sin(dtheta);
  const dx = pose[0] - base[0];
  const dy = pose[1] - base[1];
  const rx = point[0] - base[0];
  const ry = point[1] - base[1];
  return [base[0] + cos * rx - sin * ry + dx, base[1] + sin * rx + cos * ry + dy];
}

/** Commit a hand-placed scan as the explicit initial guess. Matching stays explicit. */
export function placeScan(pose: [number, number, number]) {
  // Imported pairs already contain their own matcher result and do not use the
  // generated scene state. Do not let a placement callback switch the view
  // back to a generated scenario.
  if (get(importMode) && get(imported)) return;

  generation.update((current) => ({ ...current, initial_guess: pose }));
  // Keep the already rendered cloud in the new pose while the lightweight
  // generation preview catches up. Move the points as well as the metadata;
  // otherwise the next paint would interpret old world points with the new
  // pose and briefly jump to the wrong location.
  preview.update((current) => {
    if (!current) return current;
    return {
      ...current,
      sensor_unaligned: current.sensor_unaligned.map((point) =>
        movePoint(point, current.initial_pose, pose),
      ),
      initial_pose: pose,
    };
  });
  placed.set(true);
}

export function resetPlacement() {
  generation.update((current) => ({ ...current, initial_guess: null }));
  placed.set(false);
  placing.set(false);
}

export function setAbMode(value: boolean) {
  abMode.set(value);
  if (value) {
    stepMode.set(false);
    importMode.set(false);
    imported.set(null);
    editingSide.set('A');
  }
}

export function setStepMode(value: boolean) {
  stepMode.set(value);
  if (value) {
    setAbMode(false);
    importMode.set(false);
    imported.set(null);
  }
}

/** Backwards-compatible name for callers that still refer to the old mode. */
export const sequenceMode = stepMode;
export const setSequenceMode = setStepMode;

/** Import an ordered polar/Cartesian scan pair. No ground truth is invented. */
export async function importPair(text: string) {
  error.set(null);
  try {
    const pair = JSON.parse(text) as Record<string, unknown>;
    // Imported inputs use the current matcher settings, like generated inputs.
    pair.config = get(matcher);
    const response = await importScanPair(pair);
    imported.set(response);
    importMode.set(true);
    placing.set(false);
    placed.set(false);
    result.set(null);
    compare.set(null);
    benchmark.set(null);
    resultKey.set(activeKey());
    stepTarget.set(0);
    traceIteration.set(0);
    outdated.set(false);
  } catch (cause) {
    error.set(cause instanceof Error ? cause.message : String(cause));
  }
}

export function clearImport() {
  importMode.set(false);
  imported.set(null);
}

export async function refreshExperiments() {
  try {
    experiments.set((await listExperiments()).names);
  } catch {
    // A missing data directory simply means no saved experiments yet.
  }
}

/** Save the current run as a named experiment on the local backend. */
export async function saveCurrentExperiment(name: string) {
  error.set(null);
  try {
    const run: RunRequest = {
      generation: get(generation),
      matcher: get(matcher),
      reference_mode: get(referenceMode),
      trace: get(trace),
      request_id: 0,
    };
    const document = await saveExperiment(name, run, get(abMode) ? get(matcherB) : null);
    loadedExperiment.set(document);
    experimentWarnings.set([]);
    rerunResult.set(null);
    await refreshExperiments();
  } catch (cause) {
    error.set(cause instanceof Error ? cause.message : String(cause));
  }
}

/** Load a saved experiment: apply its configuration and show its snapshot. */
export async function openExperiment(name: string) {
  error.set(null);
  try {
    const outcome = await loadExperiment(name);
    const document = outcome.document;
    loadedExperiment.set(document);
    experimentWarnings.set(outcome.warnings);
    rerunResult.set(null);
    generation.set({ ...document.session.generation });
    matcher.set({ ...document.session.matcher });
    referenceMode.set(document.session.reference_mode);
    importMode.set(false);
    imported.set(null);
    result.set(null);
    compare.set(null);
    resultKey.set(null);
    stepTarget.set(0);
    traceIteration.set(0);
    outdated.set(false);
  } catch (cause) {
    error.set(cause instanceof Error ? cause.message : String(cause));
  }
}

/** Import a portable experiment document, validating its version. */
export async function importPortable(text: string) {
  error.set(null);
  try {
    const document = JSON.parse(text) as ExperimentDocument;
    const outcome = await importExperiment(document);
    loadedExperiment.set(outcome.document);
    experimentWarnings.set(outcome.warnings);
    rerunResult.set(null);
  } catch (cause) {
    error.set(cause instanceof Error ? cause.message : String(cause));
  }
}

/** The loaded experiment as a portable document, for copy/export. */
export function exportLoadedExperiment(): string {
  const document = get(loadedExperiment);
  return document ? JSON.stringify(document, null, 2) : '';
}

/** Rerun a loaded experiment from its stored scans, distinct from its snapshot. */
export async function rerunLoadedExperiment() {
  const document = get(loadedExperiment);
  if (!document) return;
  error.set(null);
  try {
    rerunResult.set(await replaySession(document.session));
  } catch (cause) {
    error.set(cause instanceof Error ? cause.message : String(cause));
  }
}

export async function run() {
  if (get(stepMode)) return runStep();
  importMode.set(false);
  imported.set(null);
  if (get(issues).length > 0) {
    error.set('Fix the invalid matcher settings before running.');
    return;
  }
  const id = get(requestId) + 1;
  requestId.set(id);
  running.set(true);
  error.set(null);
  const key = activeKey();
  try {
    if (get(abMode)) {
      const response = await compareFrame({
        generation: get(generation),
        reference_mode: get(referenceMode),
        matcher_a: get(matcher),
        matcher_b: get(matcherB),
        request_id: id,
      });
      // Late-response guard: an older reply must never replace a newer result.
      if (response.request_id < get(requestId)) return;
      compare.set(response);
      result.set(null);
      resultKey.set(key);
      traceIteration.set(0);
      outdated.set(false);
    } else {
      const request: RunRequest = {
        generation: get(generation),
        matcher: get(matcher),
        reference_mode: get(referenceMode),
        trace: get(trace),
        request_id: id,
      };
      const response = await runFrame(request);
      if (response.request_id < get(requestId)) return;
      result.set(response);
      compare.set(null);
      resultKey.set(key);
      traceIteration.set(Math.max(0, (response.trace?.length ?? 1) - 1));
      outdated.set(false);
    }
  } catch (cause) {
    if (id === get(requestId)) error.set(cause instanceof Error ? cause.message : String(cause));
  } finally {
    if (id === get(requestId)) running.set(false);
  }
}

/** Run the same pair with one more allowed iteration in step-through mode. */
export async function runStep() {
  if (get(issues).length > 0) {
    error.set('Fix the invalid matcher settings before running.');
    return;
  }
  importMode.set(false);
  imported.set(null);
  const maxIterations = Math.max(1, Math.floor(get(matcher).max_iterations));
  const target = Math.min(maxIterations, Math.max(1, get(stepTarget) + 1));
  stepTarget.set(target);
  const id = get(requestId) + 1;
  requestId.set(id);
  running.set(true);
  error.set(null);
  const key = activeKey();
  try {
    const response = await runFrame({
      generation: get(generation),
      matcher: { ...get(matcher), max_iterations: target },
      reference_mode: get(referenceMode),
      trace: true,
      request_id: id,
    });
    if (response.request_id < get(requestId)) return;
    result.set(response);
    compare.set(null);
    resultKey.set(key);
    traceIteration.set(Math.max(0, (response.trace?.length ?? 1) - 1));
    outdated.set(false);
  } catch (cause) {
    if (id === get(requestId)) error.set(cause instanceof Error ? cause.message : String(cause));
  } finally {
    if (id === get(requestId)) running.set(false);
  }
}

export function resetStep() {
  stepTarget.set(0);
  traceIteration.set(0);
  result.set(null);
  compare.set(null);
  benchmark.set(null);
  resultKey.set(null);
  outdated.set(false);
}

export const canRunNextStep = derived(
  [stepMode, result, stepTarget, matcher, running],
  ([$stepMode, $result, $target, $matcher, $running]) => {
    if (!$stepMode || $running) return false;
    if (!$result) return true;
    const maxIterations = Math.max(1, Math.floor($matcher.max_iterations));
    return $result.termination === 'IterationLimit' && $target < maxIterations;
  },
);

// --- Repeated A/B benchmark ----------------------------------------------

/** Warm up and measure both sides on the current shared problem. */
export async function runBenchmark() {
  if (get(issues).length > 0) {
    error.set('Fix the invalid matcher settings before benchmarking.');
    return;
  }
  benchmarking.set(true);
  error.set(null);
  try {
    benchmark.set(
      await benchmarkFrame({
        generation: get(generation),
        reference_mode: get(referenceMode),
        matcher_a: get(matcher),
        matcher_b: get(matcherB),
        warmup: 5,
        samples: 30,
      }),
    );
  } catch (cause) {
    error.set(cause instanceof Error ? cause.message : String(cause));
  } finally {
    benchmarking.set(false);
  }
}

let previewTimer: ReturnType<typeof setTimeout> | undefined;
let previewRequestId = 0;
/** Regenerate the scene preview without running the matcher. */
export async function refreshPreview() {
  const id = ++previewRequestId;
  const request: PreviewRequest = {
    generation: get(generation),
    reference_mode: get(referenceMode),
  };
  const key = JSON.stringify(request);
  try {
    const response = await previewFrame(request);
    const currentKey = JSON.stringify({
      generation: get(generation),
      reference_mode: get(referenceMode),
    });
    if (id === previewRequestId && key === currentKey) preview.set(response);
  } catch {
    // Preview failures are non-fatal; Run surfaces configuration problems.
  }
}

function schedulePreview() {
  clearTimeout(previewTimer);
  previewTimer = setTimeout(refreshPreview, 120);
}

// Clear the displayed run as soon as its inputs change. Keep the last preview
// visible until the next generation response arrives so edits stay responsive.
generation.subscribe(() => {
  invalidateResults(false);
  schedulePreview();
});
referenceMode.subscribe(() => {
  invalidateResults(false);
  schedulePreview();
});
matcher.subscribe(() => invalidateResults(false));
matcherB.subscribe(() => {
  if (get(abMode)) invalidateResults(false);
});
trace.subscribe(() => invalidateResults(false));
abMode.subscribe(() => invalidateResults(false));
stepMode.subscribe((value) => {
  invalidateResults(false);
  if (!value) {
    stepTarget.set(0);
    traceIteration.set(0);
  }
});

export const theme = writable<Theme>('light');

export function initTheme() {
  const stored = localStorage.getItem('csm-theme');
  const preferred: Theme =
    stored === 'light' || stored === 'dark'
      ? stored
      : window.matchMedia('(prefers-color-scheme: light)').matches
        ? 'light'
        : 'dark';
  theme.set(preferred);
}

export function toggleTheme() {
  theme.update((current) => {
    const next: Theme = current === 'dark' ? 'light' : 'dark';
    localStorage.setItem('csm-theme', next);
    return next;
  });
}

theme.subscribe((value) => {
  if (typeof document !== 'undefined') document.documentElement.dataset.theme = value;
});
