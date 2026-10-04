import catalogue from "../content/materials.json";
import { CHUNK } from "./geometry";
const colours = catalogue.map((m) => Number.parseInt(m.color.slice(1), 16));
export function hostRockColour(y: number) {
  if (y < 32) return 0xc6a578;
  if (y < 512) return 0x80634f;
  if (y < 2048) return 0x736454;
  return 0x586263;
}
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
        : hostRockColour(y);
    if (open) {
      colour = !neighbor(0, 1)
        ? 0x56615b
        : !neighbor(0, -1)
          ? 0x344851
          : !neighbor(-1, 0) || !neighbor(1, 0)
            ? 0x23333c
            : 0x111c24;
    } else {
      const hash = (Math.imul(x, 374761393) ^ Math.imul(y, 668265263)) >>> 0;
      const noise = hash % 17;
      const mineral = material !== 255 && material > 1;
      const family = mineral ? catalogue[material]?.family : "";
      // Different mineral fabrics remain inside public ore cells. They never alter vein boundaries.
      const fabric =
        family === "sulfide" || family === "furnace"
          ? (x + y * 2) % 7 === 0
          : family === "industrial"
            ? y % 5 === 0
            : family === "physical"
              ? (x - y) % 6 === 0
              : (Math.floor(x / 2) + Math.floor(y / 3)) % 5 === 0;
      const seam = (y + Math.floor(x / 24)) % 23 === 0;
      const delta = mineral
        ? fabric
          ? 24
          : noise < 3
            ? 10
            : noise > 13
              ? -22
              : -5
        : seam
          ? -12
          : noise === 0
            ? 9
            : noise > 13
              ? -7
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
