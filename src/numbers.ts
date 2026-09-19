import { RESOURCE_UNIT } from "./geometry";

const materialNumber = new Intl.NumberFormat("en", {
  maximumFractionDigits: 3,
});
export function resourceQuantity(quanta: number) {
  return quanta > 0 && quanta < RESOURCE_UNIT / 1000
    ? "<0.001"
    : materialNumber.format(quanta / RESOURCE_UNIT);
}

export function displayNumber(value: number | string, style = "compact") {
  const number =
    typeof value === "string" && /^-?\d+$/.test(value)
      ? BigInt(value)
      : Number(value);
  return new Intl.NumberFormat("en", {
    notation:
      style === "scientific"
        ? "scientific"
        : style === "full"
          ? "standard"
          : number >= 10000
            ? "compact"
            : "standard",
    maximumFractionDigits: style === "scientific" ? 3 : 1,
  }).format(number);
}
