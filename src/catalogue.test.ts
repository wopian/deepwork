import { test, expect } from "bun:test";
import materials from "../content/materials.json";
test("catalogue identifiers and routes remain usable", () => {
  const ids = new Set();
  for (const m of materials) {
    expect(ids.has(m.id)).toBe(false);
    ids.add(m.id);
    expect(m.id).toBeLessThan(materials.length);
    expect(m.color).toMatch(/^#[0-9a-f]{6}$/);
    expect(m.product.length).toBeGreaterThan(0);
    expect(m.price).toBeGreaterThan(0);
  }
});
test("distinct copper routes and aluminium electrolysis", () => {
  expect(materials.find((m) => m.name === "Malachite")?.family).toBe(
    "chemical",
  );
  expect(materials.find((m) => m.name === "Chalcopyrite")?.family).toBe(
    "sulfide",
  );
  expect(materials.find((m) => m.name === "Bauxite")?.family).toBe(
    "electrolytic",
  );
});
