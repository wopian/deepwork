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
