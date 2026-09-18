import type { GenerationConfig } from './api';

export interface Example {
  id: string;
  name: string;
  /** What to observe when the example runs. */
  observe: string;
  generation: GenerationConfig;
}

function generation(overrides: Partial<GenerationConfig>): GenerationConfig {
  return {
    scenario: 'asymmetric_room',
    seed: 7,
    motion: 1.0,
    noise: 0.01,
    dropout: 0.0,
    initial_error: 0.05,
    ray_count: 181,
    half_span: 2.2,
    overlap: 0.0,
    step: 4,
    ...overrides,
  };
}

/** Bundled examples. Each is a generation recipe, not a stored dataset. */
export const EXAMPLE_LIST: Example[] = [
  {
    id: 'easy-alignment',
    name: 'Easy alignment',
    observe:
      'A cluttered room seen from two nearby poses. The sensor scan should settle onto the reference, with translation and rotation error near zero.',
    generation: generation({}),
  },
  {
    id: 'ambiguous-corridor',
    name: 'Ambiguous corridor',
    observe:
      'A long straight corridor weakly constrains motion along it, so yaw and travel are ambiguous. Watch a good fit still hide along-corridor error.',
    generation: generation({ scenario: 'ambiguous_corridor', step: 6 }),
  },
  {
    id: 'low-overlap',
    name: 'Low overlap',
    observe:
      'The sensor has moved far enough that only part of the scene is shared. Alignment depends on how much geometry the two scans still have in common.',
    generation: generation({ scenario: 'partial_overlap', step: 10, motion: 2.0, overlap: 1.5, initial_error: 0.1 }),
  },
  {
    id: 'noisy',
    name: 'Noisy scans',
    observe:
      'Every range reading carries substantial noise. The fit stays plausible but truth-based error grows; the residual is not accuracy.',
    generation: generation({ noise: 0.15 }),
  },
  {
    id: 'missing-readings',
    name: 'Missing readings',
    observe:
      'Many rays return nothing and are marked invalid. The matcher must work from the readings that remain, including through dropout.',
    generation: generation({ dropout: 0.4 }),
  },
  {
    id: 'poor-initialization',
    name: 'Poor initial guess',
    observe:
      'The initial guess is far from the truth. A local method can settle into the wrong basin or fail outright; check the termination, not just the pose.',
    generation: generation({ initial_error: 1.2 }),
  },
];

export function exampleById(id: string): Example {
  return EXAMPLE_LIST.find((example) => example.id === id) ?? EXAMPLE_LIST[0];
}
