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
}));

vi.mock('./api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('./api')>();
  return { ...actual, ...mocks };
});

import { DEFAULT_MATCHER, type FrameResponse, type RunRequest } from './api';
import {
  abMode,
  activeFrame,
  generation,
  matcher,
  outdated,
  referenceMode,
  result,
  run,
  sequence,
  sequenceIndex,
  sequenceMode,
  setAbMode,
  setSequenceMode,
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

  result.set(null);
  sequence.set(null);
  sequenceIndex.set(0);
  sequenceMode.set(false);
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

  generation.update((current) => ({ ...current, noise: current.noise + 0.05 }));
  expect(get(outdated)).toBe(true);
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

it('shares one set of inputs across A and B', async () => {
  setAbMode(true);
  await run();

  const current = get(view);
  expect(current?.sensor_aligned).toHaveLength(1);
  expect(current?.sensor_aligned_b).toHaveLength(1);
  expect(current?.reference).toEqual([[0, 0]]);
});

it('reveals the selected sequence frame diagnostics', async () => {
  // Establish a current result so the sequence view is not flagged outdated.
  await run();
  setSequenceMode(true);
  sequence.set({
    referenceMode: 'fixed',
    truth: [
      [0, 0],
      [1, 1],
    ],
    estimate: [
      [0, 0],
      [1, 1],
    ],
    frames: [
      frame({ termination: 'Converged', accepted: true }),
      frame({ termination: 'NoCorrespondences', accepted: false }),
    ],
  });

  sequenceIndex.set(0);
  expect(get(activeFrame)?.termination).toBe('Converged');
  sequenceIndex.set(1);
  expect(get(activeFrame)?.termination).toBe('NoCorrespondences');
  expect(get(view)?.rejected).toBe(true);
});