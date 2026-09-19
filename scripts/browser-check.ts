import { checkReadability } from "./readability-check";
import { chromium } from "playwright-core";
import { mkdir } from "node:fs/promises";
const readability: unknown[] = [];
const output = process.argv[2] ?? "test-results";
await mkdir(output, { recursive: true });
const reservation = Bun.serve({
  hostname: "127.0.0.1",
  port: 0,
  fetch: () => new Response("reserved"),
});
const port = reservation.port;
await reservation.stop(true);
const url = `http://127.0.0.1:${port}`;
const server = Bun.spawn(
  [
    process.execPath,
    "--bun",
    "node_modules/vite/bin/vite.js",
    ...(process.argv.includes("--built") ? ["preview"] : []),
    "--host",
    "127.0.0.1",
    "--port",
    String(port),
    "--strictPort",
  ],
  {
    stdout: "ignore",
    stderr: "pipe",
  },
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
  const page = await browser.newPage({
    viewport: { width: 1440, height: 1050 },
  });
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto(url);
  await page.locator("canvas").waitFor();
  await page.waitForTimeout(1000);
  readability.push(await checkReadability(page, "Desktop mine"));
  await page.screenshot({
    path: output + "/desktop-preview.png",
    fullPage: true,
  });
  for (const name of [
    "Equipment",
    "Processing",
    "Crew",
    "Logistics",
    "Production",
    "Contracts",
  ]) {
    await page.getByRole("button", { name, exact: true }).click();
    await page.locator(".context-panel").waitFor();
    readability.push(await checkReadability(page, `Desktop ${name}`));
    if (!(await page.locator("canvas").isVisible()))
      throw new Error("Workshop hid the mine");
    await page
      .getByRole("button", { name: "Close panel", exact: true })
      .click();
  }
  for (const name of [
    "Minerals",
    "Industry",
    "Headquarters",
    "Records",
    "Settings",
    "Operations",
  ]) {
    await page.getByRole("button", { name, exact: true }).click();
    await page.waitForTimeout(50);
    readability.push(await checkReadability(page, name));
  }
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByLabel("Audio", { exact: true }).check();
  await page.getByLabel("Machinery ambience", { exact: true }).uncheck();
  await page.getByLabel("Audio", { exact: true }).uncheck();
  await page.getByLabel("Number display").selectOption("full");
  await page.getByLabel("Visual quality").selectOption("low");
  await page.getByRole("button", { name: "Operations", exact: true }).click();
  await page.getByRole("button", { name: "Waste", exact: true }).click();
  await page.getByRole("button", { name: "Surface ↑", exact: true }).click();
  await page.setViewportSize({ width: 390, height: 844 });
  await page.screenshot({
    path: output + "/mobile-preview.png",
    fullPage: true,
  });
  await page.getByRole("button", { name: "Equipment", exact: true }).click();
  await page.screenshot({
    path: output + "/mobile-equipment.png",
    fullPage: true,
  });
  readability.push(await checkReadability(page, "Portrait equipment"));
  await page.getByRole("button", { name: "Close panel", exact: true }).click();
  for (const name of [
    "Processing",
    "Crew",
    "Logistics",
    "Production",
    "Contracts",
  ]) {
    await page.getByRole("button", { name, exact: true }).click();
    readability.push(await checkReadability(page, `Portrait ${name}`));
    await page
      .getByRole("button", { name: "Close panel", exact: true })
      .click();
  }
  for (const name of [
    "Minerals",
    "Industry",
    "Headquarters",
    "Records",
    "Settings",
    "Operations",
  ]) {
    await page.getByRole("button", { name, exact: true }).click();
    readability.push(await checkReadability(page, `Portrait ${name}`));
  }
  await Bun.write(
    output + "/readability.json",
    JSON.stringify(readability, null, 2),
  );
  const overflow = await page.evaluate(
    () => document.documentElement.scrollWidth > innerWidth + 1,
  );
  if (overflow) throw new Error("Portrait layout overflows horizontally");
  if (errors.length) throw new Error(errors.join("\n"));
  console.log(
    "Browser preview: all tabs, canvas, portrait width and page errors checked.",
  );
} finally {
  await browser?.close();
  server.kill();
}
