import { chromium, type Page } from "playwright-core";
import {
  copyFile,
  mkdir,
  mkdtemp,
  readFile,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { installSaveDecoder } from "./save-container";
import { clickControl } from "./game-controls";
import { checkReadability } from "./readability-check";

const baseline = process.argv.includes("--baseline");
const measureOnly = process.argv.includes("--measure-only");
const fixtureArgument = process.argv.indexOf("--fixture");
const fixturePath =
  fixtureArgument >= 0
    ? process.argv[fixtureArgument + 1]!
    : "test-results/natural-veins-final/native-fixture.json";
const output = resolve(
  "test-results/presentation",
  baseline ? "baseline" : "redesign",
);
await mkdir(output, { recursive: true });
const directory = await mkdtemp(join(tmpdir(), "deepwork-presentation-"));
const executable = join(directory, "deepwork.exe");
await copyFile(
  baseline
    ? join(output, "deepwork.exe")
    : resolve("target/release/deepwork.exe"),
  executable,
);
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
  await page.locator("canvas").waitFor();
  await page.evaluate(() =>
    localStorage.setItem(
      "deepwork-preferences",
      JSON.stringify({ quality: "high", tutorialDismissed: true }),
    ),
  );
  await page.reload();
  await page.locator("canvas").waitFor();
  const cdp = await context.newCDPSession(page);
  await cdp.send("Performance.enable");
  const viewport = async (width: number, height: number, mobile = false) => {
    await cdp.send("Emulation.setDeviceMetricsOverride", {
      width,
      height,
      deviceScaleFactor: 1,
      mobile,
    });
    await cdp.send("Emulation.setTouchEmulationEnabled", { enabled: mobile });
  };
  const invoke = (command: string, args: Record<string, unknown> = {}) =>
    page.evaluate(
      async ({ command, args }) =>
        (window as any).__TAURI_INTERNALS__.invoke(command, args),
      { command, args },
    );
  const control = async (name: string) => {
    if (baseline)
      await page
        .getByRole("button", {
          name: name === "Surface" ? "Surface ↑" : name,
          exact: true,
        })
        .click();
    else await clickControl(page, name);
  };
  await viewport(1440, 940);
  await page.screenshot({ path: join(output, "starter.png") });
  if (!baseline) {
    await page.getByRole("button", { name: /^View goal:/ }).click();
    const furnace = page.locator('[data-upgrade="furnace"]');
    const card = (await furnace.boundingBox())!;
    const content = (await page.locator(".drawer-content").boundingBox())!;
    if (card.y < content.y || card.y > content.y + content.height)
      throw new Error("Goal did not reveal matching upgrade");
    await clickControl(page, "Operations");
  }
  const fixture = JSON.parse(await readFile(resolve(fixturePath), "utf8"));
  fixture.last_saved = Math.floor(Date.now() / 1000);
  await invoke("import_save", {
    data: JSON.stringify(fixture),
    encoding: "json",
  });
  await page.waitForFunction(() => !document.querySelector(".catchup-notice"));
  await page.reload();
  await page.locator("canvas").waitFor();
  await page.waitForFunction(
    () =>
      Number(document.querySelector<HTMLElement>(".world")?.dataset.workers) ===
      16,
  );
  await control("Active crew");
  await page.waitForTimeout(1500);
  const samples = [];
  for (let n = 0; n < 3; n++) {
    await page.waitForTimeout(2000);
    const { metrics } = await cdp.send("Performance.getMetrics");
    const telemetry = await page
      .locator(".world")
      .evaluate((node: HTMLElement) => ({ ...node.dataset }));
    samples.push({
      telemetry,
      heap_bytes: metrics.find((item: any) => item.name === "JSHeapUsedSize")
        ?.value,
    });
    if (
      Number(telemetry.residentChunks) > 768 ||
      Number(telemetry.workers) > 250 ||
      Number(telemetry.moving) > 2000
    )
      throw new Error("Renderer budget exceeded");
  }
  await cdp.send("HeapProfiler.collectGarbage");
  const retainedMetrics = (await cdp.send("Performance.getMetrics")).metrics;
  const retainedHeapBytes = retainedMetrics.find(
    (item: any) => item.name === "JSHeapUsedSize",
  )?.value;
  if (measureOnly) {
    await writeFile(
      join(output, "retained-memory.json"),
      JSON.stringify(
        {
          fixture: fixturePath,
          retained_heap_bytes: retainedHeapBytes,
          samples,
        },
        null,
        2,
      ),
    );
    console.log(
      `${baseline ? "Baseline" : "Redesign"} retained heap: ${retainedHeapBytes}`,
    );
  } else {
    await page.screenshot({ path: join(output, "mature-mine.png") });
    await control("Surface");
    await page.waitForTimeout(250);
    await page.screenshot({ path: join(output, "surface.png") });
    const readability: unknown[] = [];
    let stress: unknown = null;
    let skipped = false;
    if (!baseline) {
      for (const name of [
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
      ]) {
        await clickControl(page, name);
        readability.push(await checkReadability(page, `Native ${name}`));
        if (!(await page.locator("canvas").isVisible()))
          throw new Error("Management unmounted mine");
      }
      await clickControl(page, "Operations");
      await page.screenshot({ path: join(output, "surface-closed.png") });
      for (const [width, height, mobile] of [
        [640, 360, false],
        [836, 470, false],
        [1031, 580, false],
        [390, 844, true],
        [844, 390, true],
      ] as const) {
        await viewport(width, height, mobile);
        await control("Active crew");
        await page.waitForTimeout(250);
        readability.push(
          await checkReadability(page, `${width}×${height} mine`),
        );
        await page.screenshot({
          path: join(output, `mine-${width}x${height}.png`),
        });
      }
      await viewport(390, 844, true);
      await clickControl(page, "Crew");
      await page.screenshot({ path: join(output, "portrait-crew.png") });
      await clickControl(page, "Settings");
      await page.getByLabel("Reduced motion", { exact: true }).check();
      await clickControl(page, "Operations");
      const saved = JSON.parse(
        await page.evaluate(() => (window as any).__DEEPWORK_TEST_EXPORT__()),
      );
      saved.last_saved = Math.floor(Date.now() / 1000) - 10;
      await invoke("import_save", {
        data: JSON.stringify(saved),
        encoding: "json",
      });
      await page.waitForFunction(
        () => !document.querySelector(".catchup-notice"),
      );
      if (await page.locator(".milestone-notice").count())
        throw new Error("Replay celebrated old discoveries");
      await clickControl(page, "Settings");
      await page.getByLabel("Reduced motion", { exact: true }).uncheck();
      await clickControl(page, "Operations");
      // Short catch-up validates new HUD locking and Skip without rerunning travel progression.
      saved.last_saved = Math.floor(Date.now() / 1000) - 120;
      await page.evaluate((data) => {
        (window as any).__PRESENTATION_REPLAY__ = null;
        void (window as any).__TAURI_INTERNALS__
          .invoke("import_save", { data, encoding: "json" })
          .then((result: any) => {
            (window as any).__PRESENTATION_REPLAY__ = result;
          });
      }, JSON.stringify(saved));
      await page.locator(".catchup-notice").waitFor();
      await clickControl(page, "Records");
      for (const name of ["Reset campaign", "Export save"])
        if (
          !(await page.getByRole("button", { name, exact: true }).isDisabled())
        )
          throw new Error(`${name} enabled in catch-up`);
      if (!(await page.locator('input[type="file"]').isDisabled()))
        throw new Error("Import enabled in catch-up");
      await clickControl(page, "Operations");
      await page.screenshot({ path: join(output, "timelapse.png") });
      const skip = page.getByRole("button", {
        name: "Skip timelapse",
        exact: true,
      });
      skipped = await skip.isVisible();
      if (!skipped) throw new Error("Replay completed before Skip coverage");
      await skip.click();
      await page.waitForFunction(() => (window as any).__PRESENTATION_REPLAY__);
      await page.waitForFunction(
        () => !document.querySelector(".catchup-notice"),
      );
    }
    if (!baseline) {
      await viewport(1440, 940);
      const stressFixture = {
        ...fixture,
        workers: 1000,
        housing: 1004,
        credits: "1000000000",
        last_saved: Math.floor(Date.now() / 1000),
      };
      const requirements = JSON.parse(
        await readFile("content/upgrades.json", "utf8"),
      );
      for (const item of requirements) stressFixture.levels[item.id] = 50;
      await invoke("import_save", {
        data: JSON.stringify(stressFixture),
        encoding: "json",
      });
      await page.reload();
      await page.locator("canvas").waitFor();
      await control("Active crew");
      await page.waitForTimeout(2500);
      const telemetry = await page
        .locator(".world")
        .evaluate((node: HTMLElement) => ({ ...node.dataset }));
      const { metrics } = await cdp.send("Performance.getMetrics");
      stress = {
        simulated_workers: 1000,
        telemetry,
        heap_bytes: metrics.find((item: any) => item.name === "JSHeapUsedSize")
          ?.value,
      };
      if (
        Number(telemetry.workers) > 250 ||
        Number(telemetry.residentChunks) > 768 ||
        Number(telemetry.moving) > 2000
      )
        throw new Error("Stress renderer budget exceeded");
      await page.screenshot({ path: join(output, "stress.png") });
    }
    if (errors.length) throw new Error(errors.join("\n"));
    await writeFile(
      join(output, "evidence.json"),
      JSON.stringify(
        {
          baseline,
          fixture: fixturePath,
          samples,
          stress,
          skipped,
          readability,
          page_errors: errors,
        },
        null,
        2,
      ),
    );
    console.log(
      `${baseline ? "Baseline" : "Redesign"} native evidence: ${output}`,
    );
  }
} catch (error) {
  const page = browser?.contexts()[0]?.pages()[0];
  await page?.screenshot({ path: join(output, "failure.png") }).catch(() => {});
  throw error;
} finally {
  await browser?.close().catch(() => {});
  app.kill();
  await app.exited;
}
