/** Interpolate by distance so corners do not change cargo speed. */
export function routePosition(
  points: [number, number][],
  progress: number,
): [number, number] {
  if (!points.length) return [0, 0];
  const lengths = points
    .slice(1)
    .map((p, i) => Math.hypot(p[0] - points[i]![0], p[1] - points[i]![1]));
  let distance =
    Math.max(0, Math.min(1, progress)) * lengths.reduce((a, b) => a + b, 0);
  for (let i = 0; i < lengths.length; i++) {
    const length = lengths[i]!;
    if (length > 0 && distance <= length) {
      const a = points[i]!,
        b = points[i + 1]!;
      return [
        a[0] + ((b[0] - a[0]) * distance) / length,
        a[1] + ((b[1] - a[1]) * distance) / length,
      ];
    }
    distance -= length;
  }
  return points[points.length - 1]!;
}

export interface CargoLeg {
  from: [number, number];
  to: [number, number];
  mode: string;
  milliseconds: number;
}
export function cargoPosition(
  legs: CargoLeg[],
  elapsed: number,
): { point: [number, number]; mode: string } | null {
  if (!legs.length) return null;
  let left = Math.max(0, elapsed * 1000 - 2000);
  for (const leg of legs) {
    if (left <= leg.milliseconds)
      return {
        point: routePosition([leg.from, leg.to], left / leg.milliseconds),
        mode: leg.mode,
      };
    left -= leg.milliseconds;
  }
  const end = legs[legs.length - 1]!;
  return { point: end.to, mode: end.mode };
}
