import { derived, get, writable } from 'svelte/store';
import {
  DEFAULT_MATCHER,
  runFrame,
  type FrameResponse,
  type GenerationConfig,
  type MatcherConfig,
  type RunRequest,
} from './api';
import { exampleById, EXAMPLE_LIST } from './examples';
import { matcherIssues } from './matcherFields';

export type Theme = 'light' | 'dark';

export const exampleId = writable(EXAMPLE_LIST[0].id);
export const generation = writable<GenerationConfig>({ ...EXAMPLE_LIST[0].generation });
export const matcher = writable<MatcherConfig>({ ...DEFAULT_MATCHER });
export const matcherPreset = writable('');
export const referenceMode = writable('fixed');
export const trace = writable(false);

/** Plain-language validation mirroring the library. */
export const issues = derived(matcher, (value) => matcherIssues(value));

export const result = writable<FrameResponse | null>(null);
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
  truth: true,
});

export const diagnosticsOpen = writable(true);

function keyOf(request: Omit<RunRequest, 'request_id'>): string {
  return JSON.stringify([
    request.generation,
    request.matcher,
    request.reference_mode,
    request.trace,
  ]);
}

function currentRequest(): Omit<RunRequest, 'request_id'> {
  return {
    generation: get(generation),
    matcher: get(matcher),
    reference_mode: get(referenceMode),
    trace: get(trace),
  };
}

/** True when the displayed result no longer matches the current inputs. */
export const outdated = derived([generation, matcher, referenceMode, trace, resultKey], () => {
  const key = get(resultKey);
  return key !== null && key !== keyOf(currentRequest());
});

export const hasResult = derived(result, (value) => value !== null);

export function loadExample(id: string) {
  const example = exampleById(id);
  exampleId.set(example.id);
  generation.set({ ...example.generation });
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
  const request = currentRequest();
  const key = keyOf(request);
  try {
    const response = await runFrame({ ...request, request_id: id });
    // Late-response guard: an older reply must never replace a newer result.
    if (response.request_id < get(requestId)) return;
    result.set(response);
    resultKey.set(key);
  } catch (cause) {
    if (id === get(requestId)) error.set(cause instanceof Error ? cause.message : String(cause));
  } finally {
    if (id === get(requestId)) running.set(false);
  }
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