export const CHUNK = 64;
export const WORLD_WIDTH = 512;
export const CHUNKS_ACROSS = WORLD_WIDTH / CHUNK;
export const CELLS_PER_METRE = 4;
export const CELL_PIXEL = 7 / 8;
export const RESOURCE_UNIT = 64_000;
export const chunkId = (x: number, y: number) =>
  Math.floor(y / CHUNK) * CHUNKS_ACROSS + Math.floor(x / CHUNK);
