import { expect, test } from "bun:test";
import { terrainPixels } from "./terrain-pixels";
test("terrain textures preserve RGB channels and hide unrevealed geology", () => {
  const hidden = Array(4096).fill(255);
  expect([...terrainPixels(undefined, hidden, 1).slice(0, 4)]).toEqual([
    128, 99, 79, 255,
  ]);
  expect([...terrainPixels(undefined, undefined, 0).slice(0, 4)]).toEqual([
    186, 153, 108, 255,
  ]);
  hidden[0] = 3;
  expect([...terrainPixels(undefined, hidden, 1).slice(0, 4)]).toEqual([
    199, 111, 90, 255,
  ]);
  const dug = Array(512).fill(0);
  dug[0] = 1;
  expect([...terrainPixels(dug, hidden, 1).slice(0, 4)]).toEqual([
    86, 97, 91, 255,
  ]);
  expect([...terrainPixels(dug, hidden, 1).slice(4, 8)]).toEqual([
    128, 99, 79, 255,
  ]);
});

test("priority outlines cannot reveal unknown or excavated minerals", () => {
  const selected = [1];
  const hidden = Array(4096).fill(255);
  expect([
    ...terrainPixels(undefined, hidden, 1, selected).slice(0, 4),
  ]).toEqual([128, 99, 79, 255]);
  hidden[0] = 3;
  expect([
    ...terrainPixels(undefined, hidden, 1, selected).slice(0, 4),
  ]).toEqual([232, 223, 200, 255]);
  expect([...terrainPixels([1], hidden, 1, selected).slice(0, 4)]).toEqual([
    86, 97, 91, 255,
  ]);
});

test("texture stays deterministic at negative chunk coordinates and chunk edges", () => {
  const publicOre = Array(4096).fill(255);
  publicOre[63] = 8;
  publicOre[64] = 6;
  const first = terrainPixels(undefined, publicOre, 2, undefined, -1);
  expect(terrainPixels(undefined, publicOre, 2, undefined, -1)).toEqual(first);
  expect(terrainPixels(undefined, Array(4096).fill(255), 2, [255], -1)).toEqual(
    terrainPixels(undefined, undefined, 2, undefined, -1),
  );
  // An open neighbor across a chunk boundary removes the false wall edge.
  const allOpen = Array(512).fill(255);
  const wall = terrainPixels(allOpen, undefined, 2, undefined, -1);
  const connected = terrainPixels(
    allOpen,
    undefined,
    2,
    undefined,
    -1,
    () => true,
  );
  expect([...connected.slice(0, 4)]).toEqual([17, 28, 36, 255]);
  expect([...wall.slice(0, 4)]).not.toEqual([...connected.slice(0, 4)]);
});
