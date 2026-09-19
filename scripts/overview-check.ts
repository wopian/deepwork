/** Browser-only rendering check using Rust-generated passage geometry, not a native playthrough. */
import { chromium } from "playwright-core";
import { mkdir } from "node:fs/promises";
import { CELL_PIXEL } from "../src/geometry";
import { checkReadability } from "./readability-check";

const [input, output] = Bun.argv.slice(2);
if (!input || !output)
  throw new Error("overview-check.ts RAW_DIAGNOSTIC.json OUTPUT_DIRECTORY");
await mkdir(output, { recursive: true });
const game = await Bun.file(input).json();
const { active, revision, status, passages, chambers } = game.workings;
// Only public passage geometry is relevant. Empty texture data exercises the
// overview fallback without exposing private deposit descriptors to the browser.
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
game.terrain = { revision: 0, chunks: {}, visible: {}, revealed: {} };
Object.assign(game, {
  shipments: [],
  work_route: [],
  selected_vein: null,
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
  const page = await browser.newPage();
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
    await page.waitForTimeout(1000);
    const camera = await page
      .locator("[data-camera-zoom]")
      .evaluate((el) => ({ ...(el as HTMLElement).dataset }));
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
    await page.screenshot({ path: `${output}/${name}-lift.png`, fullPage: true });
    observations.push({
      name,
      passages: passages.length,
      camera,
      allPassagesFit: true,
      syntheticLiftPreview: true,
      readability,
    });
  }
  if (errors.length) throw new Error(errors.join("\n"));
  await Bun.write(
    `${output}/report.json`,
    JSON.stringify(
      { browserOnly: true, simulationAdvanced: false, observations },
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
