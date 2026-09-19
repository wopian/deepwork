import catalogue from "../content/materials.json";
import { CHUNK } from "./geometry";
const colours = catalogue.map((m) => Number.parseInt(m.color.slice(1), 16));
/** Unknown geology remains host rock; excavation always takes precedence. */
export function terrainPixels(
  mask: number[] | undefined,
  visible: number[] | undefined,
  chunkY: number,
  selected?: number[],
) {
  const pixels = new Uint8ClampedArray(CHUNK * CHUNK * 4);
  for (let i = 0; i < CHUNK * CHUNK; i++) {
    const open = !!(mask && mask[i >> 3]! & (1 << i % 8));
    const material = visible?.[i] ?? 255;
    const y = chunkY * CHUNK + Math.floor(i / CHUNK);
    let colour = open
      ? 0x101820
      : material !== 255 && material > 1
        ? colours[material]!
        : y < 32
          ? 0xd8bc7d
          : 0x806044;
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
