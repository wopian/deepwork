import { test, expect } from "bun:test";
import { displayNumber } from "./numbers";
test("large credit strings keep exact digits in full display", () => {
  expect(displayNumber("9007199254740993", "full")).toBe(
    "9,007,199,254,740,993",
  );
  expect(displayNumber(12000)).toBe("12K");
  expect(displayNumber(12000, "scientific")).toBe("1.2E4");
});
