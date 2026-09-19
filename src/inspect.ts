import { CHUNK, chunkId, localCell } from "./geometry";
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
    y < 0
  )
    return null;
  const id = chunkId(x, y),
    bit = (y % CHUNK) * CHUNK + localCell(x);
  if ((terrain.chunks[id]?.[bit >> 3] ?? 0) & (1 << bit % 8)) return null;
  const material = terrain.visible[id]?.[bit] ?? 255;
  return material > 1 && material !== 255 ? material : null;
}
