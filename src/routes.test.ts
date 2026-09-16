import { test, expect } from "bun:test";
import { routePosition, cargoPosition } from "./routes";
test("cargo follows bends at constant distance per progress", () => {
  const path: [number, number][] = [
    [0, 10],
    [0, 0],
    [30, 0],
  ];
  expect(routePosition(path, 0.125)).toEqual([0, 5]);
  expect(routePosition(path, 0.5)).toEqual([10, 0]);
  expect(routePosition(path, 1)).toEqual([30, 0]);
  expect(
    routePosition(
      [
        [2, 3],
        [2, 3],
      ],
      0.5,
    ),
  ).toEqual([2, 3]);
});

test("cargo changes vehicle at transport handoffs after loading", () => {
  const legs = [
    {
      from: [0, 10] as [number, number],
      to: [0, 0] as [number, number],
      mode: "lift",
      milliseconds: 1000,
    },
    {
      from: [0, 0] as [number, number],
      to: [10, 0] as [number, number],
      mode: "conveyor",
      milliseconds: 2000,
    },
  ];
  expect(cargoPosition(legs, 1)?.point).toEqual([0, 10]);
  expect(cargoPosition(legs, 2.5)).toEqual({ point: [0, 5], mode: "lift" });
  expect(cargoPosition(legs, 4)).toEqual({ point: [5, 0], mode: "conveyor" });
});
