import { expect, test } from "bun:test";
import { inspectOre } from "./inspect";
test("inspection never names hidden or mined ore", () => {
  const terrain = {
    chunks: {} as Record<string, number[]>,
    visible: { "2147483648": Array(4096).fill(255) },
  };
  expect(inspectOre(terrain, 2, 0)).toBeNull();
  terrain.visible["2147483648"]![2] = 3;
  expect(inspectOre(terrain, 2, 0)).toBe(3);
  terrain.chunks["2147483648"] = [4];
  expect(inspectOre(terrain, 2, 0)).toBeNull();
  expect(inspectOre(terrain, -1, 0)).toBeNull();
});
