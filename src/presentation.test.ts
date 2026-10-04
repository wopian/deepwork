import { expect, test } from "bun:test";
import type { Game } from "./game";
import { dockFor, navigation, nextGoal, NoticeTracker } from "./presentation";
import { readPreferences, writePreferences } from "./preference-storage";

function mine(overrides: Partial<Game> = {}): Game {
  return {
    campaign_id: "one",
    levels: {},
    pinned: null,
    build_queue: [],
    credits: "120",
    quotes: { furnace: "35" },
    products: {},
    purchase_blockers: {},
    research_invested: 0,
    heights: {},
    stages: [],
    discoveries: [],
    milestones: [],
    ...overrides,
  } as Game;
}
test("navigation retains every management panel exactly once", () => {
  const panels = Object.values(navigation).flat();
  expect(new Set(panels).size).toBe(11);
  for (const panel of panels)
    expect(navigation[dockFor(panel)]).toContain(panel);
});
test("guidance follows manual goals and missing prerequisites without changing orders", () => {
  const game = mine({ pinned: "trace", build_queue: ["pump"] });
  const before = JSON.stringify(game);
  expect(nextGoal(game)?.id).toBe("upgrade:furnace");
  expect(JSON.stringify(game)).toBe(before);
  game.levels = { furnace: 1, steelworks: 1, chemical: 1, electrolytic: 1 };
  game.purchase_blockers.trace = "Requires 455 headquarters research invested";
  expect(nextGoal(game)?.panel).toBe("Headquarters");
  expect(nextGoal(game)?.progress).toBe(0);
});
test("guidance checks exact authored material quantities and covers later campaign", () => {
  const game = mine({
    levels: { furnace: 1 },
    pinned: "steelworks",
    quotes: { steelworks: "600" },
    credits: "600",
    products: { iron: 64000 },
  });
  expect(nextGoal(game)?.progress).toBe(0.5);
  game.products.iron = 128000;
  expect(nextGoal(game)?.progress).toBe(1);
  const mature = mine({
    levels: Object.fromEntries(
      [
        "furnace",
        "conveyor",
        "sorter",
        "steelworks",
        "shaft",
        "supports",
        "power",
        "manufacturing",
        "chemical",
        "pump",
        "electrolytic",
        "ventilation",
        "trace",
      ].map((id) => [id, 1]),
    ),
    steel_made: true,
    heights: { "0": 1200 },
    specialisation: "bulk",
    research_invested: 400,
  });
  expect(nextGoal(mature)?.id).toBe("headquarters");
  expect(nextGoal(mature)?.progress).toBe(0);
  for (const product of [
    "advanced_structures",
    "precision_controls",
    "magnets",
    "batteries",
  ])
    mature.products[product] = 640000;
  expect(nextGoal(mature)?.progress).toBe(0.5);
});
test("announcements never reveal catch-up discoveries early or repeat loaded milestones", () => {
  const notices = new NoticeTracker();
  const game = mine({ discoveries: [3], milestones: ["First mineral"] });
  expect(notices.update(game)).toEqual([]);
  game.discoveries.push(4);
  expect(notices.update(game).map((item) => item.title)).toEqual([
    "Malachite discovered",
  ]);
  expect(notices.update(game)).toEqual([]);
  game.discoveries.push(5);
  game.milestones.push("First steel");
  expect(notices.update(game, true)).toEqual([]);
  expect(notices.update(game)).toEqual([]);
  expect(
    notices.update({
      ...game,
      campaign_id: "imported",
      discoveries: [3, 4, 5, 7],
    }),
  ).toEqual([]);
});
test("blocked and malformed preference storage never prevents play", () => {
  const blocked = {
    getItem() {
      throw new DOMException("Blocked", "SecurityError");
    },
    setItem() {
      throw new DOMException("Full", "QuotaExceededError");
    },
  };
  expect(readPreferences(blocked)).toEqual({});
  expect(() => writePreferences(blocked, { guidance: true })).not.toThrow();
  expect(readPreferences({ getItem: () => "[1,2]" })).toEqual({});
  expect(readPreferences({ getItem: () => "not json" })).toEqual({});
});
