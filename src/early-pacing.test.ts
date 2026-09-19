import { expect, test } from "bun:test";
import { assessEarlyPacing } from "../scripts/check-early-pacing";
const runs = () =>
  [0, 1, 2].map((profile) => ({
    profile,
    policy: "depth",
    events: [
      { seconds: 0, upgrade: "worker" },
      { seconds: 240, upgrade: "conveyor" },
      { seconds: 1, upgrade: "furnace" },
      { seconds: 150, product: "iron" },
      { seconds: 1, unlock: "tactics" },
      { seconds: 1800, upgrade: "shaft" },
      { seconds: 3000, specialisation: "bulk" },
    ],
  }));
test("early baseline covers every geological profile and minute window", () => {
  expect(assessEarlyPacing(runs()).accepted).toBe(true);
  const missing = runs();
  missing[0]!.profile = 1;
  expect(assessEarlyPacing(missing).accepted).toBe(false);
  expect(assessEarlyPacing(runs().slice(1)).accepted).toBe(false);
});
test("buying a furnace cannot stand in for actual refined production", () => {
  const missing = runs();
  missing[0]!.events = missing[0]!.events.filter((e) => e.product !== "iron");
  expect(assessEarlyPacing(missing).accepted).toBe(false);
  const late = runs();
  for (const run of late)
    run.events.find((e) => e.product === "iron")!.seconds = 1000;
  expect(assessEarlyPacing(late).accepted).toBe(false);
});
test("bad observations and incomparable policies cannot pass early acceptance", () => {
  const invalid = runs();
  invalid[0]!.events[0]!.seconds = Number.NaN;
  expect(assessEarlyPacing(invalid).accepted).toBe(false);
  const mixed = runs();
  mixed[0]!.policy = "vein";
  expect(assessEarlyPacing(mixed).accepted).toBe(false);
});
