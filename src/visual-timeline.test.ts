import { expect, test } from "bun:test";
import {
  motionPosition,
  mergeWorldUpdate,
  latestWorldSnapshot,
} from "./visual-timeline";

test("live delta arriving before replay save keeps complete geometry and newer movement", () => {
  const baseline = {
    campaign_id: "imported",
    ticks: 20,
    worker_x: 0,
    workings_offset: 0,
    workings: { passages: [1] },
    terrain: {
      chunks: { old: [1] } as Record<string, number[]>,
      visible: {},
      revealed: {},
    },
  };
  const live = mergeWorldUpdate(
    baseline,
    {
      ...baseline,
      ticks: 44,
      worker_x: 1.5,
      workings_offset: 1,
      workings: { passages: [2] },
      terrain: { chunks: { new: [2] }, visible: {}, revealed: {} },
    },
    false,
  );
  const saved = { ...baseline, ticks: 40, worker_x: 1 };
  const reconciled = latestWorldSnapshot(saved, live);
  expect(reconciled.worker_x).toBe(1.5);
  expect(reconciled.workings.passages).toEqual([1, 2]);
  expect(reconciled.terrain.chunks).toEqual({ old: [1], new: [2] });
  expect(latestWorldSnapshot(saved, baseline)).toBe(saved);
});

test("replay reconciliation rejects newer snapshots from another campaign", () => {
  const saved = { campaign_id: "imported", ticks: 40 };
  expect(
    latestWorldSnapshot(saved, { campaign_id: "previous", ticks: 100 }),
  ).toBe(saved);
  expect(latestWorldSnapshot(saved, null)).toBe(saved);
});

test("movement interpolates between snapshots and crosses supplied corners", () => {
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
      mode: "walking",
      milliseconds: 1000,
    },
  ];
  expect(motionPosition([0, 10], legs, 950, 150).point[0]).toBeCloseTo(1);
  expect(motionPosition([0, 10], legs, 500, 10000).point).toEqual([0, 3]);
  expect(motionPosition([0, 10], legs, 500, 150, 0).point).toEqual([0, 5]);
  expect(motionPosition([0, 10], legs, -500, 150).point).toEqual([0, 10]);
});
test("delta frames reveal only their chronological chunks and construction", () => {
  type World = {
    workings_offset: number;
    workings: { passages: number[] };
    terrain: {
      chunks: Record<string, number[]>;
      visible: Record<string, number[]>;
      revealed: Record<string, number[]>;
    };
  };
  const previous: World = {
    workings_offset: 0,
    workings: { passages: [1, 2] },
    terrain: { chunks: { "1": [1] }, visible: { "1": [255] }, revealed: {} },
  };
  const next: World = {
    workings_offset: 2,
    workings: { passages: [3] },
    terrain: { chunks: { "2": [2] }, visible: { "2": [3] }, revealed: {} },
  };
  const merged = mergeWorldUpdate(previous, next, false);
  expect(merged.workings.passages).toEqual([1, 2, 3]);
  expect(merged.terrain.visible).toEqual({ "1": [255], "2": [3] });
  expect(
    mergeWorldUpdate(
      previous,
      { ...next, terrain: { chunks: {}, visible: {}, revealed: {} } },
      true,
    ).terrain.chunks,
  ).toEqual({});
});

test("prediction holds at unpowered lift after walking handoff", () => {
  const legs = [
    {
      from: [0, 0] as [number, number],
      to: [10, 0] as [number, number],
      mode: "walking",
      milliseconds: 1000,
      speed: 1,
    },
    {
      from: [10, 0] as [number, number],
      to: [10, 10] as [number, number],
      mode: "lift",
      milliseconds: 1000,
      speed: 0,
    },
  ];
  expect(motionPosition([0, 0], legs, 950, 150).point).toEqual([10, 0]);
});
