import { clickControl } from "./game-controls";
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
  await page.waitForTimeout(400);
  const canvas = page.locator("canvas");
  await canvas.evaluate((element) => {
    element.dataset.persistent = "yes";
  });
  await page.screenshot({ path: output + "/desktop-preview.png" });
  const panels = [
    "Equipment",
    "Processing",
    "Crew",
    "Logistics",
    "Production",
    "Industry",
    "Minerals",
    "Contracts",
    "Headquarters",
    "Records",
    "Settings",
  ];
  for (const width of [1440, 390]) {
    await page.setViewportSize({ width, height: width === 1440 ? 940 : 844 });
    for (const name of panels) {
      await clickControl(page, name);
      readability.push(await checkReadability(page, `${width}px ${name}`));
      if ((await canvas.getAttribute("data-persistent")) !== "yes")
        throw new Error("Navigation recreated mine canvas");
      if (name === "Equipment" || name === "Industry" || name === "Settings")
        await page.screenshot({
          path: `${output}/${width}-${name.toLowerCase()}.png`,
        });
      const tabs = page.locator(".drawer-tabs");
      if (await tabs.count()) {
        const tabBox = (await tabs.boundingBox())!;
        const contentBox = (await page
          .locator(".drawer-content")
          .boundingBox())!;
        if (tabBox.y + tabBox.height > contentBox.y + 1)
          throw new Error("Panel tabs moved below content");
      }
      await clickControl(page, "Operations");
    }
  }
  await clickControl(page, "Settings");
  await page.getByLabel("Audio", { exact: true }).check();
  await page.getByLabel("Machinery ambience", { exact: true }).uncheck();
  await page.getByLabel("Audio", { exact: true }).uncheck();
  await page.getByLabel("Visual quality").selectOption("low");
  await page.getByLabel("Next-action guidance").uncheck();
  await page.getByLabel("Next-action guidance").check();
  await clickControl(page, "Operations");
  await clickControl(page, "Waste");
  await clickControl(page, "Surface");
  for (const [width, height] of [
    [640, 360],
    [836, 470],
    [1031, 580],
    [390, 844],
    [844, 390],
    [320, 568],
  ]) {
    await page.setViewportSize({ width: width!, height: height! });
    readability.push(await checkReadability(page, `${width}×${height} mine`));
    await page.screenshot({ path: `${output}/mine-${width}x${height}.png` });
    const overflow = await page.evaluate(
      () =>
        document.documentElement.scrollWidth > innerWidth + 1 ||
        document.documentElement.scrollHeight > innerHeight + 1,
    );
    if (overflow) throw new Error(`${width}×${height} viewport overflows`);
    const dock = await page.locator(".game-dock").boundingBox();
    for (const button of await page.locator(".game-dock button").all()) {
      const box = (await button.boundingBox())!;
      if (box.y + box.height > height! + 1)
        throw new Error("Dock action clipped vertically");
      if (box.x < dock!.x || box.x + box.width > dock!.x + dock!.width + 1)
        throw new Error("Dock action clipped");
    }
    await clickControl(page, "Equipment");
    readability.push(
      await checkReadability(page, `${width}×${height} equipment`),
    );
    await page.keyboard.press("Escape");
    if (await page.locator(".game-drawer").count())
      throw new Error("Escape did not dismiss panel");
  }
  await page.setViewportSize({ width: 390, height: 844 });
  await page
    .locator(".game-shell")
    .evaluate((element: HTMLElement) =>
      element.style.setProperty("--host-overlay-left", "46px"),
    );
  readability.push(await checkReadability(page, "Host overlay inset"));
  await page.screenshot({ path: output + "/host-overlay.png" });
  await page
    .locator(".game-shell")
    .evaluate((element: HTMLElement) =>
      element.style.removeProperty("--host-overlay-left"),
    );
  await page
    .getByRole("button", { name: "Dismiss camera tutorial", exact: true })
    .click();
  await page.reload();
  await page.locator("canvas").waitFor();
  if (await page.locator(".camera-hint").count())
    throw new Error("Tutorial dismissal lost on reload");
  await page.emulateMedia({ reducedMotion: "reduce" });
  await clickControl(page, "Settings");
  await page.getByLabel("Reduced motion", { exact: true }).check();
  await clickControl(page, "Operations");
  await page.screenshot({ path: output + "/reduced-motion.png" });
  const closed = await page
    .getByRole("button", { name: "Build", exact: true })
    .getAttribute("aria-pressed");
  await page.getByRole("button", { name: "Build", exact: true }).click();
  await page.keyboard.press("Escape");
  if (
    (await page.evaluate(() => document.activeElement?.textContent?.trim())) !==
    "Build"
  )
    throw new Error("Panel dismissal did not restore focus");
  const restricted = await browser.newContext();
  await restricted.addInitScript(() => {
    Object.defineProperty(window, "localStorage", {
      get() {
        throw new DOMException("Blocked", "SecurityError");
      },
    });
  });
  const privatePage = await restricted.newPage();
  const privateErrors: string[] = [];
  privatePage.on("pageerror", (error) => privateErrors.push(error.message));
  await privatePage.goto(url);
  await privatePage.locator("canvas").waitFor();
  await clickControl(privatePage, "Settings");
  await privatePage.getByLabel("Next-action guidance").uncheck();
  if (privateErrors.length)
    throw new Error(
      "Restricted storage blocked play: " + privateErrors.join(", "),
    );
  await restricted.close();
  await Bun.write(
    output + "/readability.json",
    JSON.stringify(readability, null, 2),
  );
  if (errors.length) throw new Error(errors.join("\n"));
  console.log(
    "Browser visual acceptance: persistent mine, 11 panels, 6 viewports, host overlay, focus, reduced motion, and blocked storage passed.",
  );
} catch (error) {
  const page = browser?.contexts()[0]?.pages()[0];
  await page?.screenshot({ path: output + "/failure.png" }).catch(() => {});
  throw error;
} finally {
  await browser?.close();
  server.kill();
}
