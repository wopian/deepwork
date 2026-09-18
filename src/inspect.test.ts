import { expect, test } from "bun:test";
import { inspectOre } from "./inspect";
test("inspection never names hidden or mined ore", () => {
  const terrain = {
    chunks: {} as Record<string, number[]>,
    visible: { "0": Array(4096).fill(255) },
  };
  expect(inspectOre(terrain, 2, 0)).toBeNull();
  terrain.visible["0"]![2] = 3;
  expect(inspectOre(terrain, 2, 0)).toBe(3);
  terrain.chunks["0"] = [4];
  expect(inspectOre(terrain, 2, 0)).toBeNull();
  expect(inspectOre(terrain, -1, 0)).toBeNull();
});
