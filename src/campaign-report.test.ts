import { expect, test } from "bun:test";
import { assessCampaign } from "../scripts/check-campaign";
import pacing from "../content/pacing.json";
const validRuns = () =>
  Array.from({ length: 30 }, (_, index) => ({
    seed: 42 + index,
    strategy: ["bulk", "precision", "reclamation"][index % 3]!,
    mode: "scheduled",
    complete: true,
    events: Object.fromEntries(
      Object.entries(pacing.windows).map(([name, range]) => [
        name,
        (range[0]! + range[1]!) / 2,
      ]),
    ),
  }));
test("campaign acceptance covers all seeds and all comparable windows", () => {
  const runs = validRuns();
  // Early minute targets belong to continuous first-site tests.
  for (const run of runs) run.events.furnace = 43200;
  expect(assessCampaign(runs).accepted).toBe(true);
});
test("successful survivors cannot hide missing campaigns or milestones", () => {
  const runs = validRuns();
  runs[0]!.complete = false;
  delete runs[0]!.events.headquarters;
  const result = assessCampaign(runs);
  expect(result.accepted).toBe(false);
  expect(result.completed).toBe(29);
  expect(result.milestones.headquarters!.reached).toBe(29);
});
test("duplicate seeds and mixed timing bases cannot pass campaign acceptance", () => {
  const runs = validRuns();
  runs[0]!.seed = runs[1]!.seed;
  runs[1]!.mode = "continuous";
  expect(assessCampaign(runs).accepted).toBe(false);
  expect(assessCampaign(runs.slice(0, 29)).accepted).toBe(false);
});
test("campaign completion alone does not pass missed pacing windows", () => {
  const runs = validRuns();
  for (const run of runs)
    run.events.precision = pacing.windows.precision[1]! + 86400;
  expect(assessCampaign(runs).accepted).toBe(false);
  runs[0]!.events.precision = Number.NaN;
  expect(assessCampaign(runs).milestones.precision!.reached).toBe(29);
});
test("even campaign samples average both central milestone observations", () => {
  const runs = validRuns();
  runs.forEach(
    (run, index) =>
      (run.events.headquarters = index < 15 ? 30 * 86400 : 32 * 86400),
  );
  expect(assessCampaign(runs).milestones.headquarters!.median_seconds).toBe(
    31 * 86400,
  );
});
