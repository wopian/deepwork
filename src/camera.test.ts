import { expect, test } from "bun:test";
import { gesture, zoomAt, resizeViewport, fitBounds } from "./camera";

test("five-kilometre workings fit both screens and keep low-zoom gestures anchored", () => {
  const bounds = { left: -1515, right: 2860, top: 208, bottom: 17708 };
  for (const viewport of [
    { width: 1440, height: 818 },
    { width: 412, height: 740 },
  ]) {
    const camera = fitBounds(bounds, viewport);
    const scale = (viewport.width / 1100) * camera.zoom;
    expect(camera.zoom).toBeLessThan(0.15);
    expect(camera.x + bounds.left * scale).toBeGreaterThanOrEqual(32 - 1e-9);
    expect(camera.x + bounds.right * scale).toBeLessThanOrEqual(
      viewport.width - 32 + 1e-9,
    );
    expect(camera.y + bounds.top * scale).toBeCloseTo(128);
    expect(camera.y + bounds.bottom * scale).toBeLessThanOrEqual(
      viewport.height - 96 + 1e-9,
    );
    const next = zoomAt(camera, 0.5, { x: 100, y: 300 });
    expect(next.zoom).toBeCloseTo(camera.zoom / 2);
    expect((100 - next.x) / next.zoom).toBeCloseTo(
      (100 - camera.x) / camera.zoom,
    );
    expect((300 - next.y) / next.zoom).toBeCloseTo(
      (300 - camera.y) / camera.zoom,
    );
  }
});

test("portrait rotation preserves the inspected world centre and scale", () => {
  const camera = { zoom: 3, x: -966, y: -1855 };
  const before = { width: 1408, height: 818 };
  const after = { width: 412, height: 760 };
  const next = resizeViewport(camera, before, after);
  expect(after.width * next.zoom).toBeCloseTo(before.width * camera.zoom);
  expect((after.width / 2 - next.x) / (after.width * next.zoom)).toBeCloseTo(
    (before.width / 2 - camera.x) / (before.width * camera.zoom),
  );
  expect((after.height / 2 - next.y) / (after.width * next.zoom)).toBeCloseTo(
    (before.height / 2 - camera.y) / (before.width * camera.zoom),
  );
  const restored = resizeViewport(next, after, before);
  expect(restored.zoom).toBeCloseTo(camera.zoom);
  expect(restored.x).toBeCloseTo(camera.x);
  expect(restored.y).toBeCloseTo(camera.y);
});

test("rotation keeps the world centre even when the zoom limit applies", () => {
  const camera = { zoom: 20, x: -1200, y: -400 };
  const before = { width: 1400, height: 800 },
    after = { width: 300, height: 600 };
  const next = resizeViewport(camera, before, after);
  expect(next.zoom).toBe(24);
  expect((after.width / 2 - next.x) / (after.width * next.zoom)).toBeCloseTo(
    (before.width / 2 - camera.x) / (before.width * camera.zoom),
  );
});

test("pinch preserves anchor while its centre moves", () => {
  const camera = { zoom: 2, x: -100, y: 40 };
  const next = zoomAt(camera, 3, { x: 50, y: 80 }, { x: 70, y: 110 });
  expect((70 - next.x) / next.zoom).toBe((50 - camera.x) / camera.zoom);
  expect((110 - next.y) / next.zoom).toBe((80 - camera.y) / camera.zoom);
});

test("zoom clamps without moving its anchor", () => {
  const next = zoomAt({ zoom: 12, x: 0, y: 0 }, 100, { x: 100, y: 200 });
  expect(next).toEqual({ zoom: 24, x: -100, y: -200 });
});

test("one pointer pans and two pointers define a pinch", () => {
  expect(gesture([{ x: 10, y: 20 }])).toEqual({
    centre: { x: 10, y: 20 },
    distance: 0,
  });
  expect(
    gesture([
      { x: 0, y: 0 },
      { x: 60, y: 80 },
    ]),
  ).toEqual({ centre: { x: 30, y: 40 }, distance: 100 });
});
