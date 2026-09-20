import type { BrowserContext, Page } from "playwright-core";

const MAGIC = "DWRKSAVE";

function bytes(raw: unknown) {
  if (raw instanceof Uint8Array) return raw;
  if (raw instanceof ArrayBuffer) return new Uint8Array(raw);
  if (Array.isArray(raw)) return Uint8Array.from(raw);
  throw new Error("Tauri export did not return binary save bytes");
}

export function decodeSave(raw: unknown) {
  const container = bytes(raw);
  const magic = new TextDecoder().decode(container.subarray(0, 8));
  if (magic !== MAGIC || container.length < 20)
    throw new Error("Invalid Deepwork save container");
  const expected = Number(
    new DataView(container.buffer, container.byteOffset + 12, 8).getBigUint64(
      0,
      true,
    ),
  );
  const payload = Bun.gunzipSync(Uint8Array.from(container.subarray(20)));
  if (payload.length !== expected)
    throw new Error("Deepwork save payload length mismatch");
  return JSON.parse(new TextDecoder().decode(payload));
}

function installBrowserDecoder() {
  (window as any).__DEEPWORK_TEST_EXPORT__ = async () => {
    const raw = await (window as any).__TAURI_INTERNALS__.invoke("export_save");
    const container =
      raw instanceof ArrayBuffer ? new Uint8Array(raw) : new Uint8Array(raw);
    if (new TextDecoder().decode(container.subarray(0, 8)) !== "DWRKSAVE")
      throw new Error("Invalid Deepwork save container");
    const expected = Number(
      new DataView(container.buffer, container.byteOffset + 12, 8).getBigUint64(
        0,
        true,
      ),
    );
    const stream = new Blob([container.subarray(20)])
      .stream()
      .pipeThrough(new DecompressionStream("gzip"));
    const payload = new Uint8Array(await new Response(stream).arrayBuffer());
    if (payload.length !== expected)
      throw new Error("Deepwork save payload length mismatch");
    return new TextDecoder().decode(payload);
  };
}

export async function installSaveDecoder(context: BrowserContext, page: Page) {
  await context.addInitScript(installBrowserDecoder);
  await page.evaluate(installBrowserDecoder);
}
