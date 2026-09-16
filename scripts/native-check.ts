import { chromium, type Browser } from "playwright-core";
import { mkdtemp, mkdir, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
const output = resolve(process.argv[2] ?? "test-results");
await mkdir(output, { recursive: true });
const data = await mkdtemp(join(tmpdir(), "deepwork-native-"));
const app = Bun.spawn([resolve("target/debug/deepwork.exe")], {
  env: { ...process.env, DEEPWORK_TEST_DATA_DIR: data,
    WEBVIEW2_USER_DATA_FOLDER: join(data, "webview"),
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: "--remote-debugging-port=9224" },
  stdout: "pipe", stderr: "pipe",
});
let browser: Browser | undefined;
try {
  for (let i=0; i<150; i++) {
    try { if ((await fetch("http://127.0.0.1:9224/json/version")).ok) break; } catch {}
    if (app.exitCode !== null) throw new Error(`App exited ${app.exitCode}: ${await new Response(app.stderr).text()}`);
    await Bun.sleep(200);
  }
  browser = await chromium.connectOverCDP("http://127.0.0.1:9224");
  const context = browser.contexts()[0]!;
  const page = context.pages()[0] ?? await context.waitForEvent("page");
  page.on("console", msg => console.log("WEBVIEW", msg.type(), msg.text()));
  page.on("pageerror", e => console.log("WEBVIEW ERROR", e.message));
  console.log("Native URL", page.url());
  await page.waitForLoadState("domcontentloaded");
  await page.locator("canvas").waitFor().catch(async e => { await page.screenshot({path:join(output,"native-failure.png")}); console.log(await page.locator("body").innerText()); throw e; });
  const errors: string[] = [];
  page.on("pageerror", e => errors.push(e.message));
  const invoke = async (command: string, args: Record<string, unknown> = {}) => page.evaluate(async ({command,args}) => {
    return (window as any).__TAURI_INTERNALS__.invoke(command,args);
  }, {command,args});
  const initial = JSON.parse(await invoke("export_save"));
  await page.waitForTimeout(2000);
  const advanced = JSON.parse(await invoke("export_save"));
  if (advanced.ticks <= initial.ticks || advanced.excavated <= initial.excavated) throw new Error("Native simulation did not advance");
  const purchased = await invoke("command", { action: { sequence: advanced.last_sequence+1, kind:"buy", target:"worker", value:0 } });
  if (purchased.workers !== advanced.workers+1) throw new Error("Worker purchase failed");
  const disk = JSON.parse(await readFile(join(data,"mine.json"),"utf8"));
  if (disk.workers !== purchased.workers) throw new Error("Purchase was not checkpointed");
  let rejected = false;
  try { await invoke("import_save", {data:'{"version":999}'}); } catch { rejected=true; }
  if (!rejected) throw new Error("Corrupt import accepted");
  await page.reload();
  await page.locator("canvas").waitFor();
  const restored = JSON.parse(await invoke("export_save"));
  if (restored.workers !== purchased.workers) throw new Error("Reload lost purchase");
  for (const name of ["Minerals","Industry","Headquarters","Records","Settings","Operations"]) {
    await page.getByRole("button", {name,exact:true}).click();
  }
  await page.screenshot({path:join(output,"native-game.png"),fullPage:true});
  if (errors.length) throw new Error(errors.join("\n"));
  console.log(JSON.stringify({result:"passed", ticks:restored.ticks, workers:restored.workers, data},null,2));
} finally {
  await browser?.close();
  app.kill();
  await app.exited;
}
