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
export const sequenceMode = writable(false);
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

export interface SequenceState {
  frames: FrameResponse[];
  referenceMode: string;
  truth: [number, number][];
  estimate: [number, number][];
}

export const sequence = writable<SequenceState | null>(null);
export const sequenceIndex = writable(0);
export const sequenceProgress = writable<{ done: number; total: number } | null>(null);
export const playing = writable(false);

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

export const layers = writable({
  walls: true,
  reference: true,
  unaligned: true,
  aligned: true,
  alignedB: true,
  truth: true,
  correspondences: true,
});

export const diagnosticsOpen = writable(true);
/** Selected iteration in the trace, for stepping and overlay. */
export const traceIteration = writable(0);

function activeKey(): string {
  return JSON.stringify([
    get(generation),
    get(matcher),
    get(referenceMode),
    get(trace),
    get(abMode),
    get(abMode) ? get(matcherB) : null,
  ]);
}

/** The frame whose pair diagnostics are shown: sequence, or the single result. */
export const activeFrame = derived(
  [result, sequence, sequenceIndex, sequenceMode],
  ([$result, $sequence, $index, $sequenceMode]) => {
    if ($sequenceMode && $sequence) return $sequence.frames[$index] ?? null;
    return $result;
  },
);

/** True when the displayed result no longer matches the current inputs. */
export const outdated = derived(
  [generation, matcher, matcherB, referenceMode, trace, abMode, resultKey],
  ([$generation, $matcher, $matcherB, $mode, $trace, $ab, $key]) => {
    const current = JSON.stringify([
      $generation,
      $matcher,
      $mode,
      $trace,
      $ab,
      $ab ? $matcherB : null,
    ]);
    return $key !== null && $key !== current;
  },
);

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
    sequenceMode,
    sequence,
    sequenceIndex,
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
      $sequenceMode,
      $sequence,
      $index,
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
    if ($sequenceMode && $sequence && !$outdated) {
      const frame = $sequence.frames[$index];
      if (frame) {
        return {
          reference: frame.reference,
          sensor_unaligned: frame.sensor_unaligned,
          sensor_true: frame.sensor_true,
          sensor_aligned: frame.sensor_aligned,
          estimated_pose: frame.estimated_pose,
          truth_pose: frame.truth_pose,
          initial_pose: frame.initial_pose,
          extent: frame.extent,
          segments: frame.segments,
          trajectory_true: $sequence.truth,
          trajectory_estimate: $sequence.estimate,
          rejected: !frame.accepted,
          hasTruth: true,
        };
      }
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
    if (!$ab && !$sequenceMode && $result && !$outdated) return $result as ViewData;
    return ($preview as ViewData | null) ?? ($result as ViewData | null);
  },
);

export function loadExample(id: string) {
  const example = exampleById(id);
  exampleId.set(example.id);
  generation.set({ ...example.generation });
}

export function setAbMode(value: boolean) {
  abMode.set(value);
  if (value) {
    sequenceMode.set(false);
    importMode.set(false);
    imported.set(null);
    editingSide.set('A');
  }
}

export function setSequenceMode(value: boolean) {
  sequenceMode.set(value);
  pauseSequence();
  if (value) {
    setAbMode(false);
    importMode.set(false);
    imported.set(null);
  }
}

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
    result.set(null);
    compare.set(null);
    sequence.set(null);
    benchmark.set(null);
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
  if (get(sequenceMode)) return runSequence();
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
    }
  } catch (cause) {
    if (id === get(requestId)) error.set(cause instanceof Error ? cause.message : String(cause));
  } finally {
    if (id === get(requestId)) running.set(false);
  }
}

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
/** Regenerate the scene preview without running the matcher. */
export async function refreshPreview() {
  try {
    const request: PreviewRequest = {
      generation: get(generation),
      reference_mode: get(referenceMode),
    };
    preview.set(await previewFrame(request));
  } catch {
    // Preview failures are non-fatal; Run surfaces configuration problems.
  }
}

function schedulePreview() {
  clearTimeout(previewTimer);
  previewTimer = setTimeout(refreshPreview, 120);
}

// Any change to the generated problem refreshes the preview immediately.
generation.subscribe(schedulePreview);
referenceMode.subscribe(schedulePreview);

// --- Sequence execution and playback -------------------------------------

let sequenceCancelled = false;
let playbackTimer: ReturnType<typeof setTimeout> | undefined;

/**
 * Run frames 1..step one at a time. Each previous-frame step carries its own
 * accumulated estimate forward, so no prefix is ever recomputed, and stopping
 * the loop is instant cancellation.
 */
export async function runSequence() {
  if (get(issues).length > 0) {
    error.set('Fix the invalid matcher settings before running.');
    return;
  }
  importMode.set(false);
  imported.set(null);
  pauseSequence();
  const total = Math.max(1, get(generation).step);
  const mode = get(referenceMode);
  sequenceCancelled = false;
  running.set(true);
  error.set(null);
  sequenceProgress.set({ done: 0, total });
  resultKey.set(activeKey());

  const frames: FrameResponse[] = [];
  const truth: [number, number][] = [];
  const estimate: [number, number][] = [];
  let prior: [number, number, number] | null = mode === 'previous_frame' ? [0, 0, 0] : null;

  try {
    for (let step = 1; step <= total; step += 1) {
      if (sequenceCancelled) break;
      const response = await runFrame({
        generation: { ...get(generation), step },
        matcher: get(matcher),
        reference_mode: mode,
        prior_estimate: prior,
        trace: false,
        request_id: step,
      });
      frames.push(response);
      truth.push([response.truth_pose[0], response.truth_pose[1]]);
      estimate.push([response.estimated_pose[0], response.estimated_pose[1]]);
      if (mode === 'previous_frame') prior = response.estimated_pose;
      sequenceProgress.set({ done: step, total });
    }
    if (!sequenceCancelled) {
      sequence.set({ frames, referenceMode: mode, truth, estimate });
      sequenceIndex.set(0);
      result.set(null);
      compare.set(null);
    }
  } catch (cause) {
    if (!sequenceCancelled) error.set(cause instanceof Error ? cause.message : String(cause));
  } finally {
    running.set(false);
    sequenceProgress.set(null);
  }
}

export function cancelSequence() {
  sequenceCancelled = true;
}

export function pauseSequence() {
  if (playbackTimer) {
    clearTimeout(playbackTimer);
    playbackTimer = undefined;
  }
  playing.set(false);
}

export function playSequence() {
  if (playbackTimer) return;
  playing.set(true);
  const tick = () => {
    const current = get(sequence);
    if (!current) {
      pauseSequence();
      return;
    }
    const next = get(sequenceIndex) + 1;
    if (next >= current.frames.length) {
      pauseSequence();
      return;
    }
    sequenceIndex.set(next);
    playbackTimer = setTimeout(tick, 400);
  };
  playbackTimer = setTimeout(tick, 250);
}

export function stepSequence(delta: number) {
  const current = get(sequence);
  if (!current) return;
  const next = Math.min(current.frames.length - 1, Math.max(0, get(sequenceIndex) + delta));
  sequenceIndex.set(next);
}

export function resetSequence() {
  pauseSequence();
  sequenceIndex.set(0);
}

export const theme = writable<Theme>('dark');

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