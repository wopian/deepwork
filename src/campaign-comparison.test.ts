import { expect, test } from "bun:test";
import { compareCampaigns } from "../scripts/compare-campaigns";

const report = (mode: string, times: Array<number | undefined>) => ({
  save_version: 7,
  generator_version: 3,
  days: 56,
  runs: times.map((time, index) => ({
    seed: 42 + index,
    strategy: ["bulk", "precision", "reclamation"][index % 3]!,
    mode,
    complete: time !== undefined,
    events: time === undefined ? {} : { headquarters: time },
  })),
});
test("calendar comparison pairs seed and strategy, ignoring extra baseline seeds", () => {
  const result = compareCampaigns(
    report("scheduled", [100, 200, 400]),
    report("attentive", [80, 180]),
  );
  expect(result.paired_seeds).toBe(2);
  expect(result.unmatched_baseline_seeds).toBe(1);
  expect(
    result.milestones.headquarters!.median_calendar_time_reduction_percent,
  ).toBeCloseTo(15);
});
test("unfinished runs cannot produce a survivor-only comparison median", () => {
  const result = compareCampaigns(
    report("scheduled", [100, 200]),
    report("attentive", [80, undefined]),
  );
  expect(result.comparison_completed).toBe(1);
  expect(result.milestones.headquarters!.paired_observations).toBe(1);
  expect(
    result.milestones.headquarters!.median_calendar_time_reduction_percent,
  ).toBeNull();
});
test("invalid and zero timestamps do not become percentage gains", () => {
  const result = compareCampaigns(
    report("scheduled", [0, 200, 300]),
    report("continuous", [0, NaN, -1]),
  );
  expect(result.milestones.headquarters!.paired_observations).toBe(0);
  expect(
    result.milestones.headquarters!.median_calendar_time_reduction_percent,
  ).toBeNull();
});
test("comparison rejects mismatched versions, seeds, strategies and timing modes", () => {
  const base = report("scheduled", [100, 200]);
  const other = report("attentive", [80, 180]);
  expect(() =>
    compareCampaigns(base, { ...other, generator_version: 2 }),
  ).toThrow();
  const duplicate = structuredClone(other);
  duplicate.runs[1]!.seed = 42;
  expect(() => compareCampaigns(base, duplicate)).toThrow();
  const mismatch = structuredClone(other);
  mismatch.runs[0]!.strategy = "precision";
  expect(() => compareCampaigns(base, mismatch)).toThrow();
  expect(() => compareCampaigns(base, report("scheduled", [80]))).toThrow();
  expect(() =>
    compareCampaigns(base, report("attentive", [80, 90, 100])),
  ).toThrow();
});
