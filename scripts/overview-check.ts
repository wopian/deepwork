/** Browser-only rendering check using Rust-generated passage geometry, not a native playthrough. */
import { chromium } from "playwright-core";
import { mkdir } from "node:fs/promises";
import { CELL_PIXEL, chunkOrigin } from "../src/geometry";
import catalogue from "../content/materials.json";
import { checkReadability } from "./readability-check";

const [input, output] = Bun.argv.slice(2);
const checkAccess = Bun.argv.includes("--access");
const includeTerrain = Bun.argv.includes("--terrain") || checkAccess;
if (!input || !output)
  throw new Error(
    "overview-check.ts RAW_DIAGNOSTIC.json OUTPUT_DIRECTORY [--terrain] [--access]",
  );
await mkdir(output, { recursive: true });
const game = await Bun.file(input).json();
const { active, revision, status, passages, chambers } = game.workings;
// Default empty textures exercise the overview fallback. --terrain includes
// public masks and revealed pixels; private deposit descriptors are always removed.
game.workings = {
  active,
  revision,
  status,
  passages,
  chambers,
  signals: [],
  section: null,
  target: null,
};
game.terrain = includeTerrain
  ? {
      revision: game.terrain.revision,
      chunks: game.terrain.chunks,
      visible: game.terrain.visible,
      revealed: game.terrain.revealed,
    }
  : { revision: 0, chunks: {}, visible: {}, revealed: {} };
Object.assign(game, {
  shipments: [],
  work_route: [],
  selected_vein: null,
  access_depth_limit: 700,
  access_upgrades: ["pump"],
  quotes: {},
  purchase_blockers: {},
  // Deliberately synthetic UI contract; Rust tests verify these fields' calculation.
  upgrade_previews: {
    shaft: {
      machine_percent: 12,
      line_percent: 0,
      lift_depth_after: 4500,
      access_depth_after: 1500,
    },
  },
  pinned_inputs: {},
  research_invested: 0,
  retirement_award: 0,
  raw_stock_capacity: 0,
  processing: [],
  requires_reset: false,
  offline: null,
});
const reservation = Bun.serve({
  hostname: "127.0.0.1",
  port: 0,
  fetch: () => new Response(),
});
const port = reservation.port;
await reservation.stop(true);
const url = `http://127.0.0.1:${port}`;
const server = Bun.spawn(
  [
    process.execPath,
    "--bun",
    "node_modules/vite/bin/vite.js",
    "--host",
    "127.0.0.1",
    "--port",
    String(port),
    "--strictPort",
  ],
  { stdout: "ignore", stderr: "pipe" },
);
let browser;
try {
  for (let i = 0; i < 100; i++) {
    try {
      if ((await fetch(url)).ok) break;
    } catch {}
    await Bun.sleep(100);
  }
  browser = await chromium.launch({ channel: "msedge", headless: true });
  const page = await browser.newPage({ hasTouch: true });
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  const observations = [];
  for (const [name, width, height] of [
    ["desktop", 1440, 1050],
    ["portrait", 430, 932],
  ] as const) {
    await page.setViewportSize({ width, height });
    await page.goto(url);
    await page.locator("canvas").waitFor();
    await page.evaluate(
      async ({ game, moduleUrl }) => {
        const module = await import(/* @vite-ignore */ moduleUrl);
        module.state.value = game;
      },
      { game, moduleUrl: `${url}/src/game.ts` },
    );
    await page.waitForTimeout(1000);
    await page
      .getByRole("button", { name: "Fit workings", exact: true })
      .click();
    await page.waitForTimeout(includeTerrain ? 5000 : 1000);
    const camera = await page
      .locator("[data-camera-zoom]")
      .evaluate((el) => ({ ...(el as HTMLElement).dataset }));
    if (
      Number(camera.residentChunks) > 768 ||
      (includeTerrain && !(Number(camera.residentChunks) > 0))
    )
      throw new Error(
        `${name}: terrain texture budget was not exercised or exceeded`,
      );
    const scale =
      (Number(camera.cameraWidth) / 1100) * Number(camera.cameraZoom);
    const x = Number(camera.cameraX),
      y = Number(camera.cameraY);
    for (const { feet } of passages) {
      const sx = x + (235 + feet[0] * CELL_PIXEL) * scale;
      const sy = y + (208 + feet[1] * CELL_PIXEL) * scale;
      if (
        sx < 31.99 ||
        sx > Number(camera.cameraWidth) - 31.99 ||
        sy < 127.99 ||
        sy > Number(camera.cameraHeight) - 95.99
      )
        throw new Error(`${name}: supported passage outside fitted viewport`);
    }
    await page.screenshot({ path: `${output}/${name}.png`, fullPage: true });
    let accessInspection = null;
    if (checkAccess) {
      await page
        .getByRole("button", { name: "Active crew", exact: true })
        .click();
      await page.waitForTimeout(1000);
      const view = await page
        .locator("[data-camera-zoom]")
        .evaluate((el) => ({ ...(el as HTMLElement).dataset }));
      const scale = (Number(view.cameraWidth) / 1100) * Number(view.cameraZoom);
      const candidates: {
        x: number;
        y: number;
        distance: number;
        mineral: number;
      }[] = [];
      for (const [key, visible] of Object.entries(game.terrain.visible) as [
        string,
        number[],
      ][]) {
        const [cx, cy] = chunkOrigin(Number(key));
        for (let index = 0; index < visible.length; index++) {
          const mineral = visible[index]!;
          if (
            mineral <= 1 ||
            mineral === 255 ||
            (game.terrain.chunks[key]?.[index >> 3] ?? 0) & (1 << index % 8)
          )
            continue;
          const row = cy + Math.floor(index / 64);
          if (row < 700 * 4) continue;
          const x =
            Number(view.cameraX) +
            (235 + (cx + (index % 64) + 0.5) * CELL_PIXEL) * scale;
          const y =
            Number(view.cameraY) + (208 + (row + 0.5) * CELL_PIXEL) * scale;
          if (
            x < 24 ||
            x > Number(view.cameraWidth) - 24 ||
            y < 128 ||
            y > Number(view.cameraHeight) - 170
          )
            continue;
          candidates.push({
            x,
            y,
            mineral,
            distance: Math.hypot(
              x - Number(view.cameraWidth) / 2,
              y - Number(view.cameraHeight) / 2,
            ),
          });
        }
      }
      const pick = candidates.sort((a, b) => a.distance - b.distance)[0];
      if (!pick)
        throw new Error(`${name}: no revealed ore visible near active crew`);
      const canvas = (await page.locator("canvas").boundingBox())!;
      if (name === "portrait")
        await page.touchscreen.tap(canvas.x + pick.x, canvas.y + pick.y);
      else await page.mouse.click(canvas.x + pick.x, canvas.y + pick.y);
      const inspector = page
        .locator(".ore-inspector")
        .filter({ hasText: "Access ends at" });
      await inspector.waitFor();
      const text = await inspector.innerText();
      if (!text.includes(catalogue[pick.mineral]!.name))
        throw new Error(
          `${name}: inspected mineral does not match revealed cell`,
        );
      if (!text.includes("Drainage pumps"))
        throw new Error(`${name}: missing equipment reason`);
      const accessReadability = await checkReadability(
        page,
        `${name} ore access inspector`,
      );
      await page.screenshot({
        path: `${output}/${name}-access.png`,
        fullPage: true,
      });
      await page.evaluate(async (moduleUrl) => {
        const module = await import(/* @vite-ignore */ moduleUrl);
        module.state.value = {
          ...module.state.value,
          access_depth_limit: 20000,
          access_upgrades: [],
        };
      }, `${url}/src/game.ts`);
      await inspector.waitFor({ state: "hidden" });
      await page.evaluate(async (moduleUrl) => {
        const module = await import(/* @vite-ignore */ moduleUrl);
        module.state.value = {
          ...module.state.value,
          access_depth_limit: 700,
          access_upgrades: ["pump"],
        };
      }, `${url}/src/game.ts`);
      await page
        .getByRole("button", { name: "View access equipment", exact: true })
        .click();
      await page.locator(".upgrade").first().waitFor();
      await page
        .getByRole("button", { name: "Equipment", exact: true })
        .click();
      await page
        .getByRole("button", { name: "Close mineral inspector", exact: true })
        .click();
      await page
        .getByRole("button", { name: "Fit workings", exact: true })
        .click();
      accessInspection = {
        mineral: pick.mineral,
        text,
        clearsWhenAccessImproves: true,
        equipmentButtonOpens: true,
        touchSelection: name === "portrait",
        readability: accessReadability,
      };
    }
    await page.getByRole("button", { name: "Equipment", exact: true }).click();
    const lift = page.locator(".upgrade").filter({
      has: page.getByText("Shaft & lift", { exact: true }),
    });
    await lift.scrollIntoViewIfNeeded();
    const label = (await lift.innerText()).replace(/\s+/g, " ");
    if (
      !label.includes("Lift reach after upgrade: 4,500 m.") ||
      !label.includes("equipment permits 1,500 m.")
    )
      throw new Error(`${name}: lift preview omitted reach or equipment gate`);
    const readability = await checkReadability(page, `${name} lift preview`);
    await page.screenshot({
      path: `${output}/${name}-lift.png`,
      fullPage: true,
    });
    observations.push({
      name,
      passages: passages.length,
      camera,
      allPassagesFit: true,
      syntheticLiftPreview: true,
      accessInspection,
      readability,
    });
  }
  if (errors.length) throw new Error(errors.join("\n"));
  await Bun.write(
    `${output}/report.json`,
    JSON.stringify(
      {
        browserOnly: true,
        simulationAdvanced: false,
        includeTerrain,
        checkAccess,
        observations,
      },
      null,
      2,
    ),
  );
  console.log(
    "Desktop and portrait fit every supported passage in the deep fixture.",
  );
} finally {
  await browser?.close();
  server.kill();
  await server.exited;
}
