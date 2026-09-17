import { expect, it } from 'vitest';
import { tweenPose } from './tween';

it('is exact at both endpoints', () => {
  const from: [number, number, number] = [1, -2, 0.2];
  const to: [number, number, number] = [4, 3, 1.1];
  expect(tweenPose(from, to, 0)).toEqual(from);
  expect(tweenPose(from, to, 1)).toEqual(to);
});

it('moves monotonically through the midpoint', () => {
  const [x, y, theta] = tweenPose([0, 0, 0], [10, 4, 2], 0.5);
  expect(x).toBeCloseTo(5);
  expect(y).toBeCloseTo(2);
  expect(theta).toBeCloseTo(1);
});

it('takes the shortest way around when the heading wraps', () => {
  // 0.1 rad before a full turn is a short negative step, not a long positive one.
  const [, , theta] = tweenPose([0, 0, 0], [0, 0, 2 * Math.PI - 0.1], 0.5);
  expect(Math.abs(theta)).toBeLessThan(0.1);
});

it('clamps out-of-range progress', () => {
  expect(tweenPose([0, 0, 0], [1, 1, 1], -1)).toEqual([0, 0, 0]);
  expect(tweenPose([0, 0, 0], [1, 1, 1], 2)).toEqual([1, 1, 1]);
});