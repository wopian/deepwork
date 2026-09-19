export type Camera = { zoom: number; x: number; y: number };
export type ScreenPoint = { x: number; y: number };
export type Viewport = { width: number; height: number };

/** Rotation keeps the inspected world point centred and its pixel scale stable. */
export function resizeViewport(
  camera: Camera,
  before: Viewport,
  after: Viewport,
): Camera {
  if (before.width <= 0 || after.width <= 0) return camera;
  const zoom = Math.max(
    0.15,
    Math.min(24, (camera.zoom * before.width) / after.width),
  );
  const ratio = (after.width * zoom) / (before.width * camera.zoom);
  return {
    zoom,
    x: after.width / 2 - (before.width / 2 - camera.x) * ratio,
    y: after.height / 2 - (before.height / 2 - camera.y) * ratio,
  };
}

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
