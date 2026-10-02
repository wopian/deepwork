import catalogue from "../content/materials.json";
import { CHUNK } from "./geometry";
const colours = catalogue.map((m) => Number.parseInt(m.color.slice(1), 16));
/** Unknown geology remains host rock; excavation always takes precedence. */
export function terrainPixels(
  mask: number[] | undefined,
  visible: number[] | undefined,
  chunkY: number,
  selected?: number[],
  chunkX = 0,
  isOpen?: (x: number, y: number) => boolean,
) {
  const pixels = new Uint8ClampedArray(CHUNK * CHUNK * 4);
  for (let i = 0; i < CHUNK * CHUNK; i++) {
    const open = !!(mask && mask[i >> 3]! & (1 << i % 8));
    const material = visible?.[i] ?? 255;
    const y = chunkY * CHUNK + Math.floor(i / CHUNK);
    const x = chunkX * CHUNK + (i % CHUNK);
    const neighbor = (dx: number, dy: number) => {
      if (isOpen) return isOpen(x + dx, y + dy);
      const nx = (i % CHUNK) + dx,
        ny = Math.floor(i / CHUNK) + dy;
      if (nx < 0 || nx >= CHUNK || ny < 0 || ny >= CHUNK) return false;
      const index = ny * CHUNK + nx;
      return !!(mask?.[index >> 3]! & (1 << index % 8));
    };
    let colour = open
      ? 0x101820
      : material !== 255 && material > 1
        ? colours[material]!
        : y < 32
          ? 0xd8bc7d
          : 0x806044;
    if (open) {
      colour = !neighbor(0, 1)
        ? 0x394044
        : !neighbor(0, -1)
          ? 0x28313b
          : !neighbor(-1, 0) || !neighbor(1, 0)
            ? 0x1b242e
            : 0x101820;
    } else {
      const noise =
        ((Math.imul(x, 374761393) ^ Math.imul(y, 668265263)) >>> 0) % 13;
      const mineral = material !== 255 && material > 1;
      const delta = mineral
        ? noise < 2
          ? 22
          : noise > 10
            ? -16
            : 0
        : noise === 0
          ? 8
          : noise > 10
            ? -8
            : y % 17 === 0
              ? -4
              : 0;
      const channel = (shift: number) =>
        Math.min(255, Math.max(0, ((colour >> shift) & 255) + delta));
      colour = (channel(16) << 16) | (channel(8) << 8) | channel(0);
    }
    if (
      !open &&
      material > 1 &&
      material !== 255 &&
      selected?.[i >> 3]! & (1 << i % 8)
    ) {
      const edge = [i - 1, i + 1, i - CHUNK, i + CHUNK].some(
        (j) =>
          j < 0 || j >= CHUNK * CHUNK || !(selected?.[j >> 3]! & (1 << j % 8)),
      );
      if (edge) colour = 0xe8dfc8;
    }
    pixels[i * 4] = (colour >> 16) & 255;
    pixels[i * 4 + 1] = (colour >> 8) & 255;
    pixels[i * 4 + 2] = colour & 255;
    pixels[i * 4 + 3] = 255;
  }
  return pixels;
}
