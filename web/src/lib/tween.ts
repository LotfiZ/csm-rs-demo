/** A planar pose: x, y, and heading in radians. */
export type Pose = [number, number, number];

/**
 * Interpolate between two poses along the shortest angular path.
 *
 * `t` is clamped to [0, 1], so the endpoints are exact: `tweenPose(a, b, 0)`
 * is `a` and `tweenPose(a, b, 1)` is `b`.
 */
export function tweenPose(from: Pose, to: Pose, t: number): Pose {
  const clamped = Math.min(1, Math.max(0, t));
  let dtheta = to[2] - from[2];
  dtheta = Math.atan2(Math.sin(dtheta), Math.cos(dtheta));
  return [
    from[0] + (to[0] - from[0]) * clamped,
    from[1] + (to[1] - from[1]) * clamped,
    from[2] + dtheta * clamped,
  ];
}