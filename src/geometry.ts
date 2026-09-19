export const CHUNK = 64;
export const WORLD_WIDTH = 512;
export const CHUNKS_ACROSS = WORLD_WIDTH / CHUNK;
export const CELLS_PER_METRE = 4;
export const CELL_PIXEL = 7 / 8;
export const RESOURCE_UNIT = 64_000;
// Bias signed X in the low 32 bits. Multiplication retains exact JS integers.
export const cellKey = (x: number, y: number) =>
  y * 4294967296 + x + 2147483648;
export const cellPoint = (key: number): [number, number] => [
  (key % 4294967296) - 2147483648,
  Math.floor(key / 4294967296),
];
export const chunkId = (x: number, y: number) =>
  cellKey(Math.floor(x / CHUNK), Math.floor(y / CHUNK));
export const chunkOrigin = (key: number): [number, number] => {
  const [x, y] = cellPoint(key);
  return [x * CHUNK, y * CHUNK];
};
export const localCell = (x: number) => ((x % CHUNK) + CHUNK) % CHUNK;
