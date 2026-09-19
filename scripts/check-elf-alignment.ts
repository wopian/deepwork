/** Validate the ARM64 library before packaging; executed with Windows Bun. */
const path = Bun.argv[2];
if (!path)
  throw new Error("Usage: bun.exe scripts/check-elf-alignment.ts LIBRARY.so");
const bytes = new Uint8Array(await Bun.file(path).arrayBuffer());
const view = new DataView(bytes.buffer);
if (
  bytes.length < 64 ||
  bytes[0] !== 0x7f ||
  bytes[1] !== 0x45 ||
  bytes[2] !== 0x4c ||
  bytes[3] !== 0x46 ||
  bytes[4] !== 2 ||
  bytes[5] !== 1 ||
  view.getUint16(16, true) !== 3 ||
  view.getUint16(18, true) !== 183
)
  throw new Error("Expected a little-endian ARM64 ELF library.");
const offset = view.getBigUint64(32, true);
const stride = view.getUint16(54, true);
const count = view.getUint16(56, true);
if (stride < 56 || offset + BigInt(stride * count) > BigInt(bytes.length))
  throw new Error("ELF program-header table is invalid.");
let segments = 0;
for (let index = 0; index < count; index++) {
  const header = Number(offset) + stride * index;
  if (view.getUint32(header, true) !== 1) continue;
  const fileOffset = view.getBigUint64(header + 8, true);
  const address = view.getBigUint64(header + 16, true);
  const alignment = view.getBigUint64(header + 48, true);
  if (
    alignment < 16384n ||
    (alignment & (alignment - 1n)) !== 0n ||
    fileOffset % 16384n !== address % 16384n
  )
    throw new Error(`ELF load segment ${index} is not 16 KB aligned.`);
  segments++;
}
if (!segments) throw new Error("ELF library contains no load segments.");
console.log(
  `ARM64 ELF: ${segments} load segments support 16 KB page alignment.`,
);
