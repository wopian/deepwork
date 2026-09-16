import { expect, test } from "bun:test";
import { terrainPixels } from "./terrain-pixels";
test("terrain textures preserve RGB channels and hide unrevealed geology", () => {
  const hidden = Array(4096).fill(255);
  expect([...terrainPixels(undefined, hidden, 1).slice(0, 4)]).toEqual([
    128, 96, 68, 255,
  ]);
  expect([...terrainPixels(undefined, undefined, 0).slice(0, 4)]).toEqual([
    216, 188, 125, 255,
  ]);
  hidden[0] = 3;
  expect([...terrainPixels(undefined, hidden, 1).slice(0, 4)]).toEqual([
    189, 101, 80, 255,
  ]);
  const dug = Array(512).fill(0);
  dug[0] = 1;
  expect([...terrainPixels(dug, hidden, 1).slice(0, 4)]).toEqual([
    16, 24, 32, 255,
  ]);
  expect([...terrainPixels(dug, hidden, 1).slice(4, 8)]).toEqual([
    128, 96, 68, 255,
  ]);
});
