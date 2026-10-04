export function readPreferences(
  storage: Pick<Storage, "getItem">,
): Record<string, unknown> {
  try {
    const saved: unknown = JSON.parse(
      storage.getItem("deepwork-preferences") ?? "{}",
    );
    return saved && typeof saved === "object" && !Array.isArray(saved)
      ? (saved as Record<string, unknown>)
      : {};
  } catch {
    return {};
  }
}
export function writePreferences(
  storage: Pick<Storage, "setItem">,
  value: unknown,
) {
  try {
    storage.setItem("deepwork-preferences", JSON.stringify(value));
  } catch {
    /* Restricted storage must not interrupt play. */
  }
}
