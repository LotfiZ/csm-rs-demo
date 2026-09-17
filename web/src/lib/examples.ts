import type { GenerationConfig } from './api';

export interface Example {
  id: string;
  name: string;
  /** What to observe when the example runs. */
  observe: string;
  generation: GenerationConfig;
}

/** Bundled examples. Each is a generation recipe, not a stored dataset. */
export const EXAMPLE_LIST: Example[] = [
  {
    id: 'easy-alignment',
    name: 'Easy alignment',
    observe:
      'A cluttered room seen from two nearby poses. The aligned scan should settle onto the reference, with translation and rotation error near zero.',
    generation: {
      scenario: 'asymmetric_room',
      seed: 7,
      motion: 1.0,
      noise: 0.01,
      dropout: 0.0,
      initial_error: 0.05,
      step: 4,
    },
  },
];

export function exampleById(id: string): Example {
  return EXAMPLE_LIST.find((example) => example.id === id) ?? EXAMPLE_LIST[0];
}