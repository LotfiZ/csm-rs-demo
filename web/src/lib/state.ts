import { derived, get, writable } from 'svelte/store';
import {
  DEFAULT_MATCHER,
  compareFrame,
  previewFrame,
  runFrame,
  type CompareResponse,
  type FrameResponse,
  type GenerationConfig,
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
export const preview = writable<PreviewResponse | null>(null);
export const running = writable(false);
export const error = writable<string | null>(null);

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
});

export const diagnosticsOpen = writable(true);

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

export const hasResult = derived([result, compare], ([$result, $compare]) =>
  $result !== null || $compare !== null,
);

/** What the plot draws: the live result when current, otherwise a preview. */
export interface ViewData {
  reference: [number, number][];
  sensor_unaligned: [number, number][];
  sensor_true: [number, number][];
  sensor_aligned?: [number, number][];
  estimated_pose?: [number, number, number];
  sensor_aligned_b?: [number, number][];
  estimated_pose_b?: [number, number, number];
  truth_pose: [number, number, number];
  initial_pose: [number, number, number];
  extent: number;
  segments: [[number, number], [number, number]][];
}

export const view = derived(
  [result, compare, preview, outdated, abMode],
  ([$result, $compare, $preview, $outdated, $ab]): ViewData | null => {
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
      };
    }
    if (!$ab && $result && !$outdated) return $result as ViewData;
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
  if (value) editingSide.set('A');
}

export async function run() {
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