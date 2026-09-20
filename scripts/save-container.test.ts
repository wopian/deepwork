import { expect, test } from "bun:test";
import { decodeSave } from "./save-container";

function container(value: unknown) {
  const payload = new TextEncoder().encode(JSON.stringify(value));
  const gzip = Bun.gzipSync(payload);
  const bytes = new Uint8Array(20 + gzip.length);
  bytes.set(new TextEncoder().encode("DWRKSAVE"));
  new DataView(bytes.buffer).setUint16(8, 1, true);
  bytes[10] = 1;
  new DataView(bytes.buffer).setBigUint64(12, BigInt(payload.length), true);
  bytes.set(gzip, 20);
  return bytes;
}

test("native harness reads binary Deepwork exports", () => {
  const encoded = container({ version: 11, credits: "123" });
  expect(decodeSave(encoded)).toEqual({ version: 11, credits: "123" });
  new DataView(encoded.buffer).setBigUint64(12, 999n, true);
  expect(() => decodeSave(encoded)).toThrow("payload length mismatch");
});
