import { test, expect } from "bun:test";
import { WasteParticles } from "./waste";
test("waste particle population stays bounded and settles", () => {
  const p = new WasteParticles(100);
  for (let i = 0; i < 108000; i++) {
    p.emit(30);
    p.step(1, 190);
    expect(p.items.length).toBeLessThanOrEqual(100);
  }
  for (let i = 0; i < 300; i++) p.step(1, 190);
  expect(p.items.length).toBe(0);
});
