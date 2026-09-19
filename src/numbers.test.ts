import { test, expect } from "bun:test";
import { displayNumber, resourceQuantity } from "./numbers";
test("nonzero material work stays visible before the first whole unit", () => {
  expect(resourceQuantity(1)).toBe("<0.001");
  expect(resourceQuantity(1000)).toBe("0.016");
  expect(resourceQuantity(64000)).toBe("1");
  expect(resourceQuantity(0)).toBe("0");
});
test("large credit strings keep exact digits in full display", () => {
  expect(displayNumber("9007199254740993", "full")).toBe(
    "9,007,199,254,740,993",
  );
  expect(displayNumber(12000)).toBe("12K");
  expect(displayNumber(12000, "scientific")).toBe("1.2E4");
});
