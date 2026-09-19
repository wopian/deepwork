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
const inputLocked = process.argv.includes("--locked-input");
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
  await page.evaluate(() => {
    const marker = document.createElement("div");
    marker.id = "automation-marker";
    marker.textContent = "AUTOMATED TEST · TEMPORARY SAVE";
    marker.style.cssText =
      "position:fixed;top:0;right:0;z-index:99999;padding:6px 12px;background:#E5A34D;color:#101820;font:12px monospace;pointer-events:none";
    document.body.append(marker);
  });
  await page.setViewportSize({ width: 1440, height: 1000 });
  const fixtureState = JSON.parse(
    await readFile("target/workings-fixture.json", "utf8"),
  );
  if (duration >= 1800) {
    for (const upgrade of await Bun.file("content/upgrades.json").json())
      fixtureState.levels[upgrade.id] = 50;
    fixtureState.workers = 1000;
    fixtureState.housing = 1004;
    fixtureState.credits = "1000000000";
  }
  const fixture = JSON.stringify(fixtureState);
  // Import through the same file UI used by players, preserving campaign stream identity.
  await page.getByRole("button", { name: "Records", exact: true }).click();
  await page.locator('input[type="file"]').setInputFiles({
    name: "workings.json",
    mimeType: "application/json",
    buffer: Buffer.from(fixture),
  });
  await page.waitForTimeout(2000);
  const imported = JSON.parse(
    await page.evaluate(() =>
      (window as any).__TAURI_INTERNALS__.invoke("export_save"),
    ),
  );
  if (
    imported.workers !== fixtureState.workers ||
    imported.workings.passages.length < fixtureState.workings.passages.length
  )
    throw new Error(
      `Stress fixture rejected: ${await page.locator("body").innerText()}`,
    );
  await page.getByRole("button", { name: "Operations", exact: true }).click();
  await page.getByRole("button", { name: "Follow crew", exact: true }).click();
  await page
    .getByRole("button", { name: "Survey / work plan", exact: true })
    .click();
  await page.waitForTimeout(2000);
  if (inputLocked) {
    await page.evaluate(() => {
      const marker = document.getElementById("automation-marker");
      if (marker)
        marker.textContent =
          "AUTOMATED PERFORMANCE TEST · TEMPORARY SAVE · CONTROLS LOCKED DURING MEASUREMENT";
      const shield = document.createElement("div");
      shield.style.cssText = "position:fixed;inset:0;z-index:99998;cursor:wait";
      document.body.append(shield);
      const events = [
        "pointerdown",
        "pointerup",
        "click",
        "keydown",
        "keyup",
        "wheel",
        "touchstart",
        "touchend",
      ];
      const block = (event: Event) => {
        event.preventDefault();
        event.stopImmediatePropagation();
      };
      for (const event of events)
        window.addEventListener(event, block, {
          capture: true,
          passive: false,
        });
      (window as any).__deepworkUnlockPerformanceInput = () => {
        for (const event of events)
          window.removeEventListener(event, block, true);
        shield.remove();
      };
    });
  }
  const samples = [];
  for (let n = 0; n < duration; n++) {
    if (n % 5 === 0)
      samples.push(
        await page.locator("[data-fps]").evaluate((el) => ({
          ...(el as HTMLElement).dataset,
          heap: (performance as any).memory?.usedJSHeapSize,
          visibility: document.visibilityState,
          focused: document.hasFocus(),
          width: innerWidth,
          sampledAt: Date.now(),
        })),
      );
    if (n % 60 === 0) {
      await writeFile(
        join(out, "progress.json"),
        JSON.stringify({ seconds: n, samples, errors }),
      );
      console.log(`Native workings stress: ${n}/${duration} seconds`);
    }
    await page.waitForTimeout(1000);
  }
  await page.evaluate(() => {
    (window as any).__deepworkUnlockPerformanceInput?.();
    document.getElementById("automation-marker")?.remove();
  });
  await page.screenshot({
    path: join(out, "underground-desktop.png"),
    fullPage: true,
  });
  await page.setViewportSize({ width: 430, height: 932 });
  await page.waitForTimeout(1500);
  const touch = await context.newCDPSession(page);
  await touch.send("Emulation.setTouchEmulationEnabled", { enabled: true });
  const overlay = page.getByRole("button", {
    name: "Survey / work plan",
    exact: true,
  });
  const pressed = await overlay.getAttribute("aria-pressed");
  await overlay.scrollIntoViewIfNeeded();
  const box = (await overlay.boundingBox())!;
  await touch.send("Input.dispatchTouchEvent", {
    type: "touchStart",
    touchPoints: [{ x: box.x + box.width / 2, y: box.y + box.height / 2 }],
  });
  await touch.send("Input.dispatchTouchEvent", {
    type: "touchEnd",
    touchPoints: [],
  });
  await page.waitForTimeout(200);
  if ((await overlay.getAttribute("aria-pressed")) === pressed)
    throw new Error("Touch did not toggle survey overlay");
  await touch.send("Emulation.setTouchEmulationEnabled", { enabled: false });
  await touch.detach();
  if (
    await page.evaluate(
      () => document.documentElement.scrollWidth > innerWidth + 2,
    )
  )
    throw new Error("Portrait overflow");
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
        inputLocked,
        saveBytes: Buffer.byteLength(save),
        fixtureWorkers: fixtureState.workers,
        gameplayCommands: g.last_sequence - fixtureState.last_sequence,
        touchSurvey: true,
        passages: g.workings.passages.length,
        scenarioChanges: [
          "policy",
          "specialisation",
          "workers",
          "housing",
          "levels",
          "priorities",
          "reserve",
          "pinned",
          "crew_priority",
          "cargo_policy",
          "enabled_recipes",
          "paused_recipes",
        ].filter(
          (key) => JSON.stringify(g[key]) !== JSON.stringify(fixtureState[key]),
        ),
        transportControlsChanged:
          g.transport.express !== fixtureState.transport.express ||
          g.transport.stations.some((station: any, index: number) => {
            const initial = fixtureState.transport.stations[index];
            return (
              station.level !== initial.level ||
              station.preferred !== initial.preferred
            );
          }),
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
