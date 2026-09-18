import { CHUNK, WORLD_WIDTH, chunkId } from "./geometry";
export function inspectOre(
  terrain: {
    chunks: Record<string, number[]>;
    visible: Record<string, number[]>;
  },
  x: number,
  y: number,
): number | null {
  if (
    !Number.isInteger(x) ||
    !Number.isInteger(y) ||
    x < 0 ||
    x >= WORLD_WIDTH ||
    y < 0
  )
    return null;
  const id = chunkId(x, y),
    bit = (y % CHUNK) * CHUNK + (x % CHUNK);
  if ((terrain.chunks[id]?.[bit >> 3] ?? 0) & (1 << bit % 8)) return null;
  const material = terrain.visible[id]?.[bit] ?? 255;
  return material > 1 && material !== 255 ? material : null;
}
