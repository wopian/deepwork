import { chromium } from "playwright-core";
import {
  copyFile,
  mkdir,
  mkdtemp,
  readFile,
  writeFile,
} from "node:fs/promises";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";
import { decodeSave, installSaveDecoder } from "./save-container";

const output = resolve(process.argv[2] ?? "test-results/idle-recovery-native");
const fixture = JSON.parse(await readFile(resolve(process.argv[3]!), "utf8"));
await mkdir(output, { recursive: true });
const directory = await mkdtemp(join(tmpdir(), "deepwork-idle-"));
const executable = join(directory, "deepwork.exe");
await copyFile(resolve("target/release/deepwork.exe"), executable);
const reservation = Bun.serve({
  hostname: "127.0.0.1",
  port: 0,
  fetch: () => new Response(),
});
const port = reservation.port;
await reservation.stop(true);
const app = Bun.spawn([executable, "--portable"], {
  env: {
    ...process.env,
    PATH: "C:\\Windows\\System32;C:\\Windows",
    WEBVIEW2_USER_DATA_FOLDER: join(directory, "webview"),
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
  },
  stdout: "ignore",
  stderr: "pipe",
});
const stderr = new Response(app.stderr).text();
let browser;
try {
  for (let attempt = 0; attempt < 150; attempt++) {
    try {
      if ((await fetch(`http://127.0.0.1:${port}/json/version`)).ok) break;
    } catch {}
    if (app.exitCode !== null) throw new Error(await stderr);
    await Bun.sleep(200);
  }
  browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
  const context = browser.contexts()[0]!;
  const page = context.pages()[0] ?? (await context.waitForEvent("page"));
  await installSaveDecoder(context, page);
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.locator("canvas").waitFor({ timeout: 30000 });
  await page.waitForFunction(() => (window as any).__DEEPWORK_TEST_EXPORT__);
  fixture.last_saved = Math.floor(Date.now() / 1000) - 28800;
  const started = performance.now();
  await page.evaluate((data) => {
    (window as any).__IDLE_ACCEPTANCE__ = {
      done: false,
      result: null,
      error: null,
    };
    void (window as any).__TAURI_INTERNALS__
      .invoke("import_save", { data, encoding: "json" })
      .then((result: any) => {
        (window as any).__IDLE_ACCEPTANCE__ = { done: true, result };
      })
      .catch((error: unknown) => {
        (window as any).__IDLE_ACCEPTANCE__ = {
          done: true,
          error: String(error),
        };
      });
  }, JSON.stringify(fixture));
  await page.locator(".catchup-notice").waitFor({ timeout: 30000 });
  await page.waitForFunction(() => {
    const world = document.querySelector<HTMLElement>(".world");
    return world?.dataset.cargo === "0" && world.dataset.workers === "0";
  });
  const actorCounts = await page
    .locator(".world")
    .evaluate((element: HTMLElement) => ({
      cargo: element.dataset.cargo,
      workers: element.dataset.workers,
    }));
  const blocked = await page.evaluate(async (campaignId) => {
    try {
      await (window as any).__TAURI_INTERNALS__.invoke("command", {
        campaignId,
        action: { sequence: 999999, kind: "policy", target: "depth", value: 0 },
      });
      return false;
    } catch (error) {
      return /catch-up|resume the game/i.test(String(error));
    }
  }, fixture.campaign_id);
  if (!blocked) throw new Error("Idle computation accepted gameplay command");
  await page.screenshot({ path: join(output, "excavation-replay.png") });
  const skip = page.getByRole("button", {
    name: "Skip timelapse",
    exact: true,
  });
  const skipped = await skip.isVisible();
  if (skipped) await skip.click();
  await page.waitForFunction(
    () => (window as any).__IDLE_ACCEPTANCE__.done,
    undefined,
    { timeout: 120000 },
  );
  await page.waitForFunction(() => !document.querySelector(".catchup-notice"));
  const recoveryMs = performance.now() - started;
  const result = await page.evaluate(() => (window as any).__IDLE_ACCEPTANCE__);
  if (result.error) throw new Error(result.error);
  if (result.result.offline.effective !== 14400)
    throw new Error("Idle credit cap changed");
  const committed = decodeSave(
    await readFile(join(directory, "deepwork-data", "mine.deepwork")),
  );
  if (committed.ticks < result.result.ticks)
    throw new Error("Idle rewards not checkpointed");
  await page.reload();
  await page.locator("canvas").waitFor({ timeout: 30000 });
  await page.waitForFunction(() => !document.querySelector(".catchup-notice"));
  const loaded = JSON.parse(
    await page.evaluate(() => (window as any).__DEEPWORK_TEST_EXPORT__()),
  );
  if (
    loaded.ticks < result.result.ticks ||
    loaded.excavated < result.result.excavated
  )
    throw new Error("Reload lost idle progress");
  await page.setViewportSize({ width: 390, height: 844 });
  await page.waitForTimeout(250);
  await page.screenshot({ path: join(output, "portrait.png"), fullPage: true });
  const renderer = await page
    .locator(".world")
    .evaluate((element: HTMLElement) => ({
      resident_chunks: Number(element.dataset.residentChunks),
      workers: Number(element.dataset.workers),
      cargo: Number(element.dataset.cargo),
      frame_p95_ms: Number(element.dataset.frameP95),
    }));
  if (
    renderer.resident_chunks > 768 ||
    renderer.workers > 250 ||
    renderer.cargo > 2000
  )
    throw new Error("Idle recovery exceeds renderer budgets");
  if (errors.length) throw new Error(errors.join("\n"));
  const report = {
    recovery_ms: recoveryMs,
    effective: result.result.offline.effective,
    actorCounts,
    blocked,
    skipped,
    saved: true,
    reload: true,
    renderer,
    final_snapshot_bytes: new TextEncoder().encode(
      JSON.stringify(result.result),
    ).length,
    page_errors: errors,
  };
  await writeFile(
    join(output, "evidence.json"),
    JSON.stringify(report, null, 2),
  );
  console.log(JSON.stringify(report));
} finally {
  await browser?.close();
  app.kill();
  await app.exited;
}
