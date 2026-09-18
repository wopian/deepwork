import { expect, test } from "bun:test";
import { gesture, zoomAt } from "./camera";

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
  expect(gesture([{ x: 10, y: 20 }])).toEqual({ centre: { x: 10, y: 20 }, distance: 0 });
  expect(gesture([{ x: 0, y: 0 }, { x: 60, y: 80 }])).toEqual({ centre: { x: 30, y: 40 }, distance: 100 });
});
