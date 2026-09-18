export type Camera = { zoom: number; x: number; y: number };
export type ScreenPoint = { x: number; y: number };

/** Preserve the world point under the gesture while scaling and translating. */
export function zoomAt(
  camera: Camera,
  factor: number,
  from: ScreenPoint,
  to = from,
): Camera {
  const zoom = Math.max(0.15, Math.min(24, camera.zoom * factor));
  const ratio = zoom / camera.zoom;
  return {
    zoom,
    x: to.x - (from.x - camera.x) * ratio,
    y: to.y - (from.y - camera.y) * ratio,
  };
}

export function gesture(points: ScreenPoint[]) {
  const a = points[0];
  const b = points[1] ?? a;
  return {
    centre: { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 },
    distance: Math.hypot(a.x - b.x, a.y - b.y),
  };
}
