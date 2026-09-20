import type { Page } from "playwright-core";
import {
  CELL_PIXEL,
  MINE_ORIGIN_X,
  SURFACE_Y,
  chunkOrigin,
} from "../src/geometry";
import catalogue from "../content/materials.json";

/** Exercise the player's controls using only exported revealed cells. */
export async function checkOreInspection(page: Page, mode: "mouse" | "touch") {
  const touch =
    mode === "touch" ? await page.context().newCDPSession(page) : null;
  if (touch)
    await touch.send("Emulation.setTouchEmulationEnabled", { enabled: true });
  const tap = async (x: number, y: number) => {
    if (!touch) return page.mouse.click(x, y);
    await touch.send("Input.dispatchTouchEvent", {
      type: "touchStart",
      touchPoints: [{ x, y }],
    });
    await touch.send("Input.dispatchTouchEvent", {
      type: "touchEnd",
      touchPoints: [],
    });
  };
  const button = async (name: string) => {
    const box = await page
      .getByRole("button", { name, exact: true })
      .boundingBox();
    if (!box) throw new Error(`Missing inspection control: ${name}`);
    await tap(box.x + box.width / 2, box.y + box.height / 2);
    await page.waitForTimeout(100);
  };
  const camera = () =>
    page.locator("[data-fps]").evaluate((el) => {
      const d = (el as HTMLElement).dataset,
        b = el.getBoundingClientRect();
      return {
        x: Number(d.cameraX),
        y: Number(d.cameraY),
        scale: (Number(d.cameraWidth) / 1100) * Number(d.cameraZoom),
        left: b.left,
        top: b.top,
        width: b.width,
        height: b.height,
      };
    });
  const save = async () =>
    JSON.parse(
      await page.evaluate(() => (window as any).__DEEPWORK_TEST_EXPORT__()),
    );
  try {
    if (
      await page
        .getByRole("button", { name: "Close mineral inspector", exact: true })
        .isVisible()
    )
      await button("Close mineral inspector");
    const state = await save(),
      initial = await camera();
    const expand = (runs: number[]) => {
      if (!state.terrain_encoding) return runs;
      const bytes: number[] = [];
      for (let i = 0; i < runs.length; i += 2)
        for (let n = 0; n < runs[i]!; n++) bytes.push(runs[i + 1]!);
      return bytes;
    };
    let target:
      | { x: number; y: number; id: number; distance: number }
      | undefined;
    for (const [key, encoded] of Object.entries(state.terrain.visible) as [
      string,
      number[],
    ][]) {
      const [cx, cy] = chunkOrigin(Number(key));
      const visible = expand(encoded),
        dug = expand(state.terrain.chunks[key] ?? []);
      for (let i = 0; i < visible.length; i++) {
        const id = visible[i]!;
        if (id <= 1 || id === 255 || (dug[i >> 3] ?? 0) & (1 << i % 8))
          continue;
        const x = cx + (i % 64),
          y = cy + Math.floor(i / 64);
        const sx =
          (MINE_ORIGIN_X + (x + 0.5) * CELL_PIXEL) * initial.scale + initial.x;
        const sy =
          (SURFACE_Y + (y + 0.5) * CELL_PIXEL) * initial.scale + initial.y;
        const distance = Math.hypot(
          sx - initial.width / 2,
          sy - initial.height / 2,
        );
        if (!target || distance < target.distance)
          target = { x, y, id, distance };
      }
    }
    if (!target) throw new Error("Fixture has no revealed, unmined ore");
    // Repeated short real drags keep the gesture inside the mine viewport.
    for (let n = 0; n < 80; n++) {
      const c = await camera();
      const sx =
        (MINE_ORIGIN_X + (target.x + 0.5) * CELL_PIXEL) * c.scale + c.x;
      const sy = (SURFACE_Y + (target.y + 0.5) * CELL_PIXEL) * c.scale + c.y;
      const dx = c.width / 2 - sx,
        dy = c.height / 2 - sy;
      if (Math.hypot(dx, dy) < 2) break;
      const start = { x: c.left + c.width / 2, y: c.top + c.height / 2 };
      const end = {
        x: start.x + Math.max(-120, Math.min(120, dx)),
        y: start.y + Math.max(-120, Math.min(120, dy)),
      };
      if (touch) {
        await touch.send("Input.dispatchTouchEvent", {
          type: "touchStart",
          touchPoints: [start],
        });
        await touch.send("Input.dispatchTouchEvent", {
          type: "touchMove",
          touchPoints: [end],
        });
        await touch.send("Input.dispatchTouchEvent", {
          type: "touchEnd",
          touchPoints: [],
        });
      } else {
        await page.mouse.move(start.x, start.y);
        await page.mouse.down();
        await page.mouse.move(end.x, end.y, { steps: 4 });
        await page.mouse.up();
      }
      await page.waitForTimeout(40);
    }
    const c = await camera();
    const x =
      c.left + (MINE_ORIGIN_X + (target.x + 0.5) * CELL_PIXEL) * c.scale + c.x;
    const y =
      c.top + (SURFACE_Y + (target.y + 0.5) * CELL_PIXEL) * c.scale + c.y;
    await tap(x, y);
    const name = catalogue[target.id]!.name;
    await page
      .locator(".ore-inspector strong")
      .filter({ hasText: name })
      .waitFor();
    if (!touch) {
      await page.mouse.move(c.left + c.width - 20, c.top + 170);
      await page.waitForTimeout(100);
      if (
        !(await page
          .locator(".ore-inspector strong")
          .filter({ hasText: name })
          .isVisible())
      )
        throw new Error(
          "Moving toward inspector controls discarded clicked ore",
        );
    }
    await button("Prioritise whole vein");
    await page.waitForTimeout(300);
    const selected = await save();
    if (
      JSON.stringify(selected.workings.target) !==
        JSON.stringify([target.x, target.y]) ||
      !selected.workings.target_deposit
    )
      throw new Error(
        `${mode} selection did not reach authoritative vein order`,
      );
    return {
      mode,
      material: name,
      anchor: [target.x, target.y],
      deposit: selected.workings.target_deposit,
    };
  } finally {
    if (touch) {
      await touch.send("Emulation.setTouchEmulationEnabled", {
        enabled: false,
      });
      await touch.detach();
    }
  }
}
