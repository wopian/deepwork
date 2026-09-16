import { chromium, type Browser } from "playwright-core";
import {
  mkdtemp,
  mkdir,
  readFile,
  writeFile,
  copyFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
const stressSeconds = Math.max(0, Number(process.argv[3] ?? 0));
const output = resolve(process.argv[2] ?? "test-results");
await mkdir(output, { recursive: true });
const data = await mkdtemp(join(tmpdir(), "deepwork-native-"));
const release = process.argv[4] === "release";
const executable = release
  ? join(data, "deepwork.exe")
  : resolve("target/debug/deepwork.exe");
if (release) await copyFile(resolve("target/release/deepwork.exe"), executable);
const saves = release ? join(data, "deepwork-data") : data;
const reservation = Bun.serve({
  hostname: "127.0.0.1",
  port: 0,
  fetch: () => new Response("reserved"),
});
const port = reservation.port;
await reservation.stop(true);
const app = Bun.spawn([executable, ...(release ? ["--portable"] : [])], {
  env: {
    ...process.env,
    ...(release ? { PATH: "C:\\Windows\\System32;C:\\Windows" } : {}),
    DEEPWORK_TEST_DATA_DIR: data,
    WEBVIEW2_USER_DATA_FOLDER: join(data, "webview"),
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
  },
  stdout: "pipe",
  stderr: "pipe",
});
void app.exited.then((code) => console.log("Native process exited", code));
const stderrLog = new Response(app.stderr).text();
const stdoutLog = new Response(app.stdout).text();
let browser: Browser | undefined;
try {
  for (let i = 0; i < 150; i++) {
    try {
      if ((await fetch(`http://127.0.0.1:${port}/json/version`)).ok) break;
    } catch {}
    if (app.exitCode !== null)
      throw new Error(`App exited ${app.exitCode}: ${await stderrLog}`);
    await Bun.sleep(200);
  }
  browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
  const context = browser.contexts()[0]!;
  const page = context.pages()[0] ?? (await context.waitForEvent("page"));
  page.on("console", (msg) => console.log("WEBVIEW", msg.type(), msg.text()));
  page.on("pageerror", (e) => console.log("WEBVIEW ERROR", e.message));
  console.log("Native URL", page.url());
  console.log("Isolated automated acceptance window; temporary save", saves);
  await page.waitForLoadState("domcontentloaded");
  await page
    .locator("canvas")
    .waitFor()
    .catch(async (e) => {
      await page.screenshot({ path: join(output, "native-failure.png") });
      console.log(await page.locator("body").innerText());
      throw e;
    });
  page.on("crash", () => console.log("Native renderer crashed"));
  page.on("close", () => console.log("Native page closed"));
  browser.on("disconnected", () => console.log("Native browser disconnected"));
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  const invoke = async (command: string, args: Record<string, unknown> = {}) =>
    page.evaluate(
      async ({ command, args }) => {
        return (window as any).__TAURI_INTERNALS__.invoke(command, args);
      },
      { command, args },
    );
  const initial = JSON.parse(await invoke("export_save"));
  await page.waitForTimeout(2000);
  const advanced = JSON.parse(await invoke("export_save"));
  if (
    advanced.ticks <= initial.ticks ||
    advanced.excavated <= initial.excavated
  )
    throw new Error("Native simulation did not advance");
  const purchased = await invoke("command", {
    campaignId: advanced.campaign_id,
    action: {
      sequence: advanced.last_sequence + 1,
      kind: "buy",
      target: "worker",
      value: 0,
    },
  });
  if (purchased.workers !== advanced.workers + 1)
    throw new Error("Worker purchase failed");
  const disk = JSON.parse(await readFile(join(saves, "mine.json"), "utf8"));
  if (disk.workers !== purchased.workers)
    throw new Error("Purchase was not checkpointed");
  let rejected = false;
  try {
    await invoke("import_save", { data: '{"version":999}' });
  } catch {
    rejected = true;
  }
  if (!rejected) throw new Error("Corrupt import accepted");
  await page.reload();
  await page.locator("canvas").waitFor();
  const restored = JSON.parse(await invoke("export_save"));
  if (restored.workers !== purchased.workers)
    throw new Error("Reload lost purchase");
  for (const name of [
    "Minerals",
    "Industry",
    "Headquarters",
    "Records",
    "Settings",
    "Operations",
  ]) {
    await page.getByRole("button", { name, exact: true }).click();
  }
  await page.getByLabel("Cargo scheduling").selectOption("preferred");
  await page.waitForFunction(async () => {
    const data = await (window as any).__TAURI_INTERNALS__.invoke(
      "export_save",
    );
    return JSON.parse(data).cargo_policy === "preferred";
  });
  // Exercise the actual file input/download path, not only IPC commands.
  await page.getByRole("button", { name: "Records", exact: true }).click();
  const downloadEvent = page.waitForEvent("download", { timeout: 15000 });
  await page.getByRole("button", { name: "Export save", exact: true }).click();
  const download = await downloadEvent;
  const exportedPath = join(data, "exported-save.json");
  await download.saveAs(exportedPath);
  const exported = JSON.parse(await readFile(exportedPath, "utf8"));
  if (
    exported.workers !== purchased.workers ||
    exported.cargo_policy !== "preferred"
  )
    throw new Error("UI export lost authoritative state");
  await page.locator('input[type="file"]').setInputFiles(exportedPath);
  await page.waitForTimeout(500);
  const imported = JSON.parse(await invoke("export_save"));
  if (
    imported.workers !== exported.workers ||
    imported.cargo_policy !== "preferred"
  )
    throw new Error("UI import did not restore exported state");
  await page.getByRole("button", { name: "Operations", exact: true }).click();
  const paused = await invoke("set_background", { background: true });
  await page.waitForTimeout(2200);
  const sleeping = JSON.parse(await invoke("export_save"));
  if (sleeping.ticks !== paused.ticks)
    throw new Error("Background simulation advanced at foreground rate");
  const resumed = await invoke("set_background", { background: false });
  if (
    !resumed.offline ||
    resumed.offline.effective < 1 ||
    resumed.ticks !== paused.ticks + resumed.offline.effective * 20
  )
    throw new Error("Resume did not apply half-rate offline interval");
  if (stressSeconds) {
    const fixture = JSON.parse(await invoke("export_save"));
    const requirements = await Bun.file("content/upgrades.json").json();
    for (const u of requirements) fixture.levels[u.id] = 50;
    fixture.workers = 1000;
    fixture.housing = 1004;
    fixture.credits = "1000000000";
    fixture.policy = "depth";
    const fixturePath = join(data, "stress-save.json");
    await writeFile(fixturePath, JSON.stringify(fixture));
    await page.getByRole("button", { name: "Records", exact: true }).click();
    await page.locator('input[type="file"]').setInputFiles(fixturePath);
    await page.waitForFunction(
      async () =>
        JSON.parse(
          await (window as any).__TAURI_INTERNALS__.invoke("export_save"),
        ).workers === 1000,
    );
    await page.getByRole("button", { name: "Operations", exact: true }).click();
    await page
      .getByRole("button", { name: "Follow depth", exact: true })
      .click();
    const cdp = await context.newCDPSession(page);
    await cdp.send("Performance.enable");
    const samples: unknown[] = [];
    const started = Date.now();
    while (Date.now() - started < stressSeconds * 1000) {
      await page.waitForTimeout(10000);
      if (!(await page.locator(".world").count())) {
        console.log("Restoring Operations for renderer stress coverage");
        await page.getByRole("button", { name: "Operations", exact: true }).click();
        await page.locator(".world").waitFor();
        await page.waitForTimeout(1200);
      }
      const { metrics } = await cdp.send("Performance.getMetrics");
      const telemetry: Record<string, string | undefined> = await page.evaluate(
        () => {
          const world = document.querySelector<HTMLElement>(".world");
          return world ? { ...world.dataset } : { renderHidden: "true" };
        },
      );
      const status = JSON.parse(await invoke("export_save"));
      const processInfo = Bun.spawnSync([
        "tasklist.exe",
        "/FI",
        `PID eq ${app.pid}`,
        "/FO",
        "CSV",
        "/NH",
      ]);
      const memory = processInfo.stdout.toString().match(/"([\d,]+) K"/);
      const sample = {
        seconds: Math.round((Date.now() - started) / 1000),
        heapBytes: metrics.find((m: any) => m.name === "JSHeapUsedSize")?.value,
        nativeKb: memory ? Number(memory[1]!.replaceAll(",", "")) : null,
        telemetry,
        depthMetres: Math.max(...status.heights) / 4,
        excavated: status.excavated,
        chunks: Object.keys(status.terrain.chunks).length,
        shipments: status.transport.segments.reduce(
          (n: number, segment: any) => n + segment.batches.length,
          0,
        ),
        saveBytes: new TextEncoder().encode(JSON.stringify(status)).length,
        ticks: status.ticks,
      };
      samples.push(sample);
      await writeFile(
        join(output, "native-stress.json"),
        JSON.stringify(samples, null, 2),
      );
      console.log(JSON.stringify(sample));
      if (
        Number(telemetry.workers) > 250 ||
        Number(telemetry.moving ?? telemetry.particles) > 2000
      )
        throw new Error("Visual entity budget exceeded");
    }
    await page.getByRole("button", { name: "Operations", exact: true }).click();
    await page.screenshot({
      path: join(output, "native-stress.png"),
      fullPage: true,
    });
  }
  await page.screenshot({
    path: join(output, "native-game.png"),
    fullPage: true,
  });
  await page
    .locator(".world")
    .screenshot({ path: join(output, "native-mine-detail.png") });
  await page.getByRole("button", { name: "Waste", exact: true }).click();
  await page.waitForTimeout(250);
  await page
    .locator(".world")
    .screenshot({ path: join(output, "native-waste-detail.png") });
  const portrait = await context.newCDPSession(page);
  await portrait.send("Emulation.setDeviceMetricsOverride", {
    width: 390,
    height: 844,
    deviceScaleFactor: 1,
    mobile: true,
  });
  await portrait.send("Emulation.setTouchEmulationEnabled", { enabled: true });
  const tap = async (name: string) => {
    const button = page.getByRole("button", { name, exact: true });
    await button.scrollIntoViewIfNeeded();
    const box = await button.boundingBox();
    if (!box) throw new Error(`Touch target missing: ${name}`);
    const point = { x: box.x + box.width / 2, y: box.y + box.height / 2 };
    await portrait.send("Input.dispatchTouchEvent", {
      type: "touchStart",
      touchPoints: [point],
    });
    await portrait.send("Input.dispatchTouchEvent", {
      type: "touchEnd",
      touchPoints: [],
    });
  };
  await tap("Settings");
  await page.locator(".settings").waitFor();
  await page.getByLabel("Reduced motion", { exact: true }).check();
  await tap("Operations");
  await page.waitForFunction(() => document.querySelector<HTMLElement>(".world")?.dataset.particles === "0");
  await tap("Surface ↑");
  await page.screenshot({
    path: join(output, "native-portrait.png"),
    fullPage: true,
  });
  if (
    await page.evaluate(
      () => document.documentElement.scrollWidth > innerWidth + 2,
    )
  )
    throw new Error("Portrait page overflows viewport");
  await portrait.send("Emulation.clearDeviceMetricsOverride");
  await portrait.send("Emulation.setTouchEmulationEnabled", { enabled: false });
  await portrait.detach();
  const beforeReset = JSON.parse(await invoke("export_save"));
  await page.getByRole("button", { name: "Records", exact: true }).click();
  await page
    .getByRole("button", { name: "Reset campaign", exact: true })
    .click();
  await page.getByRole("button", { name: "Cancel", exact: true }).click();
  if (
    JSON.parse(await invoke("export_save")).campaign_id !==
    beforeReset.campaign_id
  )
    throw new Error("Cancelled reset changed campaign");
  await page
    .getByRole("button", { name: "Reset campaign", exact: true })
    .click();
  await page.getByLabel("Type RESET to confirm").fill("RESET");
  await page
    .getByRole("button", { name: "Start fresh campaign", exact: true })
    .click();
  await page.getByRole("dialog").waitFor({ state: "hidden" });
  const fresh = JSON.parse(await invoke("export_save"));
  if (
    fresh.campaign_id === beforeReset.campaign_id ||
    fresh.workers !== 3 ||
    fresh.research > 2 ||
    fresh.records.length !== 0 ||
    Object.keys(fresh.ranks).length !== 0
  )
    throw new Error("Fresh campaign retained progress");
  let staleRejected = false;
  try {
    await invoke("command", {
      campaignId: beforeReset.campaign_id,
      action: { sequence: 999999, kind: "buy", target: "worker", value: 0 },
    });
  } catch {
    staleRejected = true;
  }
  if (!staleRejected) throw new Error("Stale campaign command accepted");
  if (errors.length) throw new Error(errors.join("\n"));
  console.log(
    JSON.stringify(
      {
        result: "passed",
        ticks: restored.ticks,
        workers: restored.workers,
        data,
      },
      null,
      2,
    ),
  );
} catch (error) {
  const page = browser?.contexts()[0]?.pages()[0];
  if (page) {
    await page
      .screenshot({ path: join(output, "native-failure.png"), timeout: 5000 })
      .catch(() => {});
    await writeFile(
      join(output, "native-failure.txt"),
      `${page.url()}\n${await page
        .locator("body")
        .innerText({ timeout: 5000 })
        .catch(() => "Body unavailable")}`,
    ).catch(() => {});
  }
  console.log(
    "Failure process state",
    app.exitCode,
    "browser connected",
    browser?.isConnected(),
  );
  throw error;
} finally {
  await browser?.close();
  app.kill();
  await app.exited;
  await writeFile(join(output, "native-stderr.log"), await stderrLog);
  await writeFile(join(output, "native-stdout.log"), await stdoutLog);
}
