import { chromium } from "playwright-core";
import { mkdir } from "node:fs/promises";
const output = process.argv[2] ?? "test-results";
await mkdir(output, { recursive: true });
const server = Bun.spawn([process.execPath, "run", "dev", "--port", "5174"], {
  stdout: "ignore",
  stderr: "pipe",
});
let browser;
try {
  for (let i = 0; i < 100; i++) {
    try {
      if ((await fetch("http://localhost:5174")).ok) break;
    } catch {}
    await Bun.sleep(100);
  }
  browser = await chromium.launch({ channel: "msedge", headless: true });
  const page = await browser.newPage({
    viewport: { width: 1440, height: 1050 },
  });
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto("http://localhost:5174");
  await page.locator("canvas").waitFor();
  await page.waitForTimeout(1000);
  await page.screenshot({
    path: output + "/desktop-preview.png",
    fullPage: true,
  });
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
