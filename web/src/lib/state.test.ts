/**
 * Browser-level behaviours the HTTP boundary cannot establish: outdated
 * marking, the late-response guard, A/B input sharing, and frame diagnostics.
 *
 * Run with `npm test`.
 */
import { get } from 'svelte/store';
import { beforeEach, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  runFrame: vi.fn(),
  compareFrame: vi.fn(),
  previewFrame: vi.fn(),
  importScanPair: vi.fn(),
}));

vi.mock('./api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('./api')>();
  return { ...actual, ...mocks };
});

import {
  DEFAULT_MATCHER,
  type FrameResponse,
  type ImportResponse,
  type PreviewResponse,
  type RunRequest,
  type TraceIteration,
} from './api';
import {
  abMode,
  activeFrame,
  generation,
  importMode,
  importPair,
  imported,
  matcher,
  outdated,
  placed,
  placeScan,
  placing,
  preview,
  referenceMode,
  resetPlacement,
  result,
  run,
  setStepMode,
  stepMode,
  stepTarget,
  traceIteration,
  setAbMode,
  view,
} from './state';

function frame(overrides: Partial<FrameResponse> = {}): FrameResponse {
  return {
    request_id: 1,
    scenario: 'asymmetric_room',
    reference_mode: 'fixed',
    step: 4,
    truth_pose: [1, 0, 0.1],
    initial_pose: [1, 0, 0.1],
    estimated_pose: [1.01, 0, 0.1],
    relative_truth_pose: [1, 0, 0.1],
    relative_estimated_pose: [1.01, 0, 0.1],
    drift_pose: [0, 0, 0],
    valid: true,
    accepted: true,
    termination: 'Converged',
    iterations: 3,
    nvalid: 100,
    error: 0.01,
    covariance_status: 'Disabled',
    covariance: null,
    trace: null,
    normal_ms: 1,
    instrumented_ms: 0,
    reference: [[0, 0]],
    sensor_unaligned: [[0, 0]],
    sensor_aligned: [[0, 0]],
    sensor_true: [[0, 0]],
    extent: 7,
    segments: [],
    ...overrides,
  };
}

function previewData(): PreviewResponse {
  return {
    reference: [[0, 0]],
    sensor_unaligned: [[1, 0]],
    sensor_true: [[1, 0]],
    truth_pose: [1, 0, 0],
    initial_pose: [1, 0, 0],
    extent: 7,
    segments: [],
  };
}

function compareResponse(requestId: number) {
  const shared = {
    truth_pose: [1, 0, 0.1] as [number, number, number],
    initial_pose: [1, 0, 0.1] as [number, number, number],
    reference: [[0, 0]] as [number, number][],
    sensor_unaligned: [[0, 0]] as [number, number][],
    sensor_true: [[0, 0]] as [number, number][],
    extent: 7,
    segments: [] as [[number, number], [number, number]][],
  };
  const side = {
    relative_truth_pose: [1, 0, 0.1] as [number, number, number],
    estimated_pose: [1, 0, 0.1] as [number, number, number],
    sensor_aligned: [[0, 0]] as [number, number][],
    valid: true,
    accepted: true,
    termination: 'Converged',
    iterations: 1,
    nvalid: 10,
    error: 0.01,
    covariance_status: 'Disabled',
    normal_ms: 1,
  };
  return {
    request_id: requestId,
    scenario: 'asymmetric_room',
    reference_mode: 'fixed',
    step: 4,
    shared,
    a: side,
    b: { ...side },
  };
}

beforeEach(() => {
  mocks.runFrame.mockReset();
  mocks.runFrame.mockImplementation(async (request: RunRequest) =>
    frame({ request_id: request.request_id }),
  );
  mocks.compareFrame.mockReset();
  mocks.compareFrame.mockImplementation(async (request: { request_id: number }) =>
    compareResponse(request.request_id),
  );
  mocks.previewFrame.mockReset();
  mocks.previewFrame.mockRejectedValue(new Error('no preview in tests'));
  mocks.importScanPair.mockReset();
  mocks.importScanPair.mockResolvedValue({
    reference: [[10, 0]],
    sensor_unaligned: [[1, 0]],
    sensor_aligned: [[10, 0]],
    initial_pose: [0, 0, 0],
    estimated_pose: [9, 0, 0],
    valid: true,
    accepted: true,
    termination: 'Converged',
    iterations: 2,
    nvalid: 1,
    error: 0.01,
    covariance_status: 'Disabled',
    covariance: null,
    extent: 12,
  } satisfies ImportResponse);

  result.set(null);
  preview.set(null);
  importMode.set(false);
  imported.set(null);
  stepMode.set(false);
  stepTarget.set(0);
  traceIteration.set(0);
  outdated.set(false);
  placed.set(false);
  placing.set(false);
  abMode.set(false);
  referenceMode.set('fixed');
  matcher.set({ ...DEFAULT_MATCHER });
  generation.set({
    scenario: 'asymmetric_room',
    seed: 7,
    motion: 1,
    noise: 0.01,
    dropout: 0,
    initial_error: 0.05,
    ray_count: 181,
    half_span: 2.2,
    overlap: 0,
    step: 4,
  });
});

it('marks results outdated when the inputs change', async () => {
  await run();
  expect(get(outdated)).toBe(false);

  preview.set(previewData());
  generation.update((current) => ({ ...current, noise: current.noise + 0.05 }));
  expect(get(outdated)).toBe(true);
  expect(get(result)).toBe(null);
  expect(get(preview)).not.toBe(null);
});

it('does not let a late response overwrite a newer result', async () => {
  let firstId = 0;
  let resolveFirst: (value: FrameResponse) => void = () => {};
  const first = new Promise<FrameResponse>((resolve) => {
    resolveFirst = resolve;
  });
  mocks.runFrame
    .mockImplementationOnce((request: RunRequest) => {
      firstId = request.request_id;
      return first;
    })
    .mockImplementationOnce(async (request: RunRequest) => frame({ request_id: request.request_id }));

  const earlier = run();
  const later = run();

  // The first request's response arrives after the second has already applied.
  resolveFirst(frame({ request_id: firstId, normal_ms: 1 }));
  await Promise.all([earlier, later]);

  expect(get(result)?.request_id).toBe(firstId + 1);
});

it('keeps hand placement separate from matching', () => {
  preview.set(previewData());
  placing.set(true);

  placeScan([0.4, -0.3, 0.05]);

  expect(get(placed)).toBe(true);
  expect(get(placing)).toBe(true);
  expect(get(preview)?.initial_pose).toEqual([0.4, -0.3, 0.05]);
  expect(get(preview)?.sensor_unaligned).toEqual([[0.4, -0.3]]);
  expect(mocks.runFrame).not.toHaveBeenCalled();

  resetPlacement();

  expect(get(placed)).toBe(false);
  expect(mocks.runFrame).not.toHaveBeenCalled();
});

it('keeps imported scans active while placing them', async () => {
  await importPair(JSON.stringify({ format: 'csm-rs-scan-pair', version: 1 }));
  expect(get(importMode)).toBe(true);
  expect(get(imported)).not.toBe(null);

  placing.set(true);
  placeScan([0.4, -0.3, 0.05]);

  expect(get(importMode)).toBe(true);
  expect(get(imported)).not.toBe(null);
});

it('shares one set of inputs across A and B', async () => {
  setAbMode(true);
  await run();

  const current = get(view);
  expect(current?.sensor_aligned).toHaveLength(1);
  expect(current?.sensor_aligned_b).toHaveLength(1);
  expect(current?.reference).toEqual([[0, 0]]);
});

it('runs step mode one matcher iteration at a time', async () => {
  const trace: TraceIteration[] = [
    {
      iteration: 1,
      pose: [0.2, 0, 0],
      error: 0.2,
      valid_correspondences: 4,
      restart: false,
      correspondences: [],
    },
    {
      iteration: 2,
      pose: [0.4, 0, 0],
      error: 0.1,
      valid_correspondences: 5,
      restart: false,
      correspondences: [],
    },
  ];
  mocks.runFrame.mockImplementation(async (request: RunRequest) =>
    frame({
      request_id: request.request_id,
      iterations: request.matcher.max_iterations,
      termination: 'IterationLimit',
      trace: trace.slice(0, request.matcher.max_iterations),
    }),
  );

  setStepMode(true);
  await run();
  expect(mocks.runFrame.mock.calls[0][0].matcher.max_iterations).toBe(1);
  expect(mocks.runFrame.mock.calls[0][0].trace).toBe(true);
  expect(get(stepTarget)).toBe(1);

  await run();
  expect(mocks.runFrame.mock.calls[1][0].matcher.max_iterations).toBe(2);
  expect(get(stepTarget)).toBe(2);

  traceIteration.set(0);
  expect(get(activeFrame)?.trace?.length).toBe(2);
  expect(get(view)?.estimated_pose).toEqual([0.2, 0, 0]);
  traceIteration.set(1);
  expect(get(view)?.estimated_pose).toEqual([0.4, 0, 0]);
});
