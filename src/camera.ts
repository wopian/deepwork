export type Camera = { zoom: number; x: number; y: number };
export type ScreenPoint = { x: number; y: number };
export type Viewport = { width: number; height: number };
const MIN_ZOOM = 0.0001;
const MAX_ZOOM = 24;
const clampZoom = (zoom: number) =>
  Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, zoom));

/** Fit signed world bounds with room for the camera and district controls. */
export function fitBounds(
  bounds: { left: number; right: number; top: number; bottom: number },
  viewport: Viewport,
): Camera {
  const scale = Math.min(
    Math.max(1, viewport.width - 64) /
      Math.max(100, bounds.right - bounds.left),
    Math.max(1, viewport.height - 224) /
      Math.max(100, bounds.bottom - bounds.top),
  );
  const zoom = clampZoom((scale * 1100) / viewport.width);
  const actual = (viewport.width / 1100) * zoom;
  return {
    zoom,
    x: viewport.width / 2 - ((bounds.left + bounds.right) / 2) * actual,
    y: 128 - bounds.top * actual,
  };
}

/** Rotation keeps the inspected world point centred and its pixel scale stable. */
export function resizeViewport(
  camera: Camera,
  before: Viewport,
  after: Viewport,
): Camera {
  if (before.width <= 0 || after.width <= 0) return camera;
  const zoom = clampZoom((camera.zoom * before.width) / after.width);
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
  const zoom = clampZoom(camera.zoom * factor);
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
