/** Isolated native visual/stream acceptance for dynamically generated workings. */
import { chromium } from "playwright-core";
import {
  mkdtemp,
  mkdir,
  copyFile,
  readFile,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
const out = resolve(process.argv[2] ?? "test-results/workings");
const duration = Number(process.argv[3] ?? 30);
await mkdir(out, { recursive: true });
const data = await mkdtemp(join(tmpdir(), "deepwork-workings-"));
const exe = join(data, "deepwork.exe");
await copyFile("target/release/deepwork.exe", exe);
const portServer = Bun.serve({
  hostname: "127.0.0.1",
  port: 0,
  fetch: () => new Response(),
});
const port = portServer.port;
await portServer.stop(true);
const app = Bun.spawn([exe, "--portable"], {
  env: {
    ...process.env,
    PATH: "C:\\Windows\\System32;C:\\Windows",
    WEBVIEW2_USER_DATA_FOLDER: join(data, "webview"),
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
  },
  stdout: "ignore",
  stderr: "ignore",
});
let browser;
try {
  for (let n = 0; n < 150; n++) {
    try {
      if ((await fetch(`http://127.0.0.1:${port}/json/version`)).ok) break;
    } catch {}
    await Bun.sleep(200);
  }
  browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
  const context = browser.contexts()[0]!;
  const page = context.pages()[0]!;
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.locator("canvas").waitFor();
  await page.setViewportSize({ width: 1440, height: 1000 });
  const fixture = await readFile("target/workings-fixture.json", "utf8");
  // Import through the same file UI used by players, preserving campaign stream identity.
  await page.getByRole("button", { name: "Records", exact: true }).click();
  await page
    .locator('input[type="file"]')
    .setInputFiles({
      name: "workings.json",
      mimeType: "application/json",
      buffer: Buffer.from(fixture),
    });
  await page.waitForTimeout(2000);
  await page.getByRole("button", { name: "Operations", exact: true }).click();
  await page.getByRole("button", { name: "Follow crew", exact: true }).click();
  await page
    .getByRole("button", { name: "Survey / work plan", exact: true })
    .click();
  await page.waitForTimeout(2000);
  const samples = [];
  for (let n = 0; n < duration; n++) {
    if (n % 5 === 0)
      samples.push(
        await page
          .locator("[data-fps]")
          .evaluate((el) => ({
            ...(el as HTMLElement).dataset,
            heap: (performance as any).memory?.usedJSHeapSize,
          })),
      );
    await page.waitForTimeout(1000);
  }
  await page.screenshot({
    path: join(out, "underground-desktop.png"),
    fullPage: true,
  });
  await page.setViewportSize({ width: 430, height: 932 });
  await page.waitForTimeout(1500);
  await page.screenshot({
    path: join(out, "underground-portrait.png"),
    fullPage: true,
  });
  const save = await page.evaluate(() =>
    (window as any).__TAURI_INTERNALS__.invoke("export_save"),
  );
  await writeFile(join(out, "checkpoint.json"), save);
  const g = JSON.parse(save);
  if (g.workings.passages.length < 20)
    throw new Error("Underground fixture import did not take effect");
  const before = g.ticks;
  await page.reload();
  await page.locator("canvas").waitFor();
  await page.waitForTimeout(1000);
  const resumed = JSON.parse(
    await page.evaluate(() =>
      (window as any).__TAURI_INTERNALS__.invoke("export_save"),
    ),
  );
  if (
    resumed.ticks < before ||
    resumed.workings.passages.length < g.workings.passages.length
  )
    throw new Error("Reload lost workings");
  if (errors.length) throw new Error(errors.join("\n"));
  await writeFile(
    join(out, "report.json"),
    JSON.stringify(
      {
        passed: true,
        duration,
        saveBytes: Buffer.byteLength(save),
        passages: g.workings.passages.length,
        samples,
        errors,
      },
      null,
      2,
    ),
  );
  console.log("Native workings acceptance passed", out);
} finally {
  await browser?.close();
  app.kill();
}
