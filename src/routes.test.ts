import { test, expect } from "bun:test";
import { routePosition } from "./routes";
test("cargo follows bends at constant distance per progress", () => {
  const path: [number, number][] = [[0,10],[0,0],[30,0]];
  expect(routePosition(path,0.125)).toEqual([0,5]);
  expect(routePosition(path,0.5)).toEqual([10,0]);
  expect(routePosition(path,1)).toEqual([30,0]);
  expect(routePosition([[2,3],[2,3]],0.5)).toEqual([2,3]);
});
