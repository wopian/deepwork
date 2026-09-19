import { test, expect } from "bun:test";
import {
  cellKey,
  cellPoint,
  chunkId,
  chunkOrigin,
  localCell,
} from "./geometry";
test("signed coordinates round-trip across chunks without surface-width aliases", () => {
  for (const [x, y] of [
    [-8000000, 0],
    [-65, 64],
    [-1, 799999],
    [0, 0],
    [511, 64],
    [4096, 128],
  ] as [number, number][]) {
    expect(cellPoint(cellKey(x, y))).toEqual([x, y]);
    const [cx, cy] = chunkOrigin(chunkId(x, y));
    expect([cx + localCell(x), cy + localCell(y)]).toEqual([x, y]);
    expect(Number.isSafeInteger(cellKey(x, y))).toBe(true);
  }
});
