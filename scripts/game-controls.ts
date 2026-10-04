import type { Locator, Page } from "playwright-core";
import { dockFor, navigation, type PanelName } from "../src/presentation";

/** Use the same visible navigation path as players, including native touch tests. */
export async function clickControl(
  page: Page,
  name: string,
  press: (button: Locator) => Promise<void> = (button) => button.click(),
) {
  if (name === "Operations") {
    const close = page.getByRole("button", {
      name: "Close panel",
      exact: true,
    });
    if (await close.isVisible()) await press(close);
    return;
  }
  if (
    Object.values(navigation)
      .flat()
      .includes(name as PanelName)
  ) {
    const panel = name as PanelName;
    const drawer = page.locator(".game-drawer");
    if (
      (await drawer.count()) &&
      (await drawer.getAttribute("aria-label")) === `${panel} controls`
    )
      return;
    const dock = page
      .locator(".game-dock")
      .getByRole("button", { name: dockFor(panel), exact: true });
    if ((await dock.getAttribute("aria-pressed")) !== "true") await press(dock);
    if ((await drawer.getAttribute("aria-label")) !== `${panel} controls`) {
      await press(
        page.locator(".drawer-tabs").getByRole("button", {
          name: panel === "Industry" ? "Recipes" : panel,
          exact: true,
        }),
      );
    }
    return;
  }
  const camera = [
    "Surface",
    "Surface ↑",
    "Active crew",
    "Fit workings",
    "Follow crew",
    "Survey / work plan",
    "Camp",
    "Shaft",
    "Plants",
    "Waste",
  ];
  if (camera.includes(name)) {
    if (!(await page.locator(".camera-menu").isVisible()))
      await press(
        page.getByRole("button", { name: "Camera controls", exact: true }),
      );
    await press(
      page.locator(".camera-menu").getByRole("button", {
        name: name === "Surface ↑" ? "Surface" : name,
        exact: true,
      }),
    );
    return;
  }
  await press(page.getByRole("button", { name, exact: true }));
}
