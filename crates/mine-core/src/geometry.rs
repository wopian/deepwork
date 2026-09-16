//! Shared physical scale. Storage chunks are never geological boundaries.
pub const CHUNK: u32 = 64;
pub const WIDTH: u32 = 512;
pub const CHUNKS_ACROSS: u32 = WIDTH / CHUNK;
pub const CELLS_PER_METRE: u32 = 4;
pub const PIT_ROWS: u32 = 48 * CELLS_PER_METRE;
pub const BENCH_ROWS: u32 = 6 * CELLS_PER_METRE;
pub const PIT_MARGIN: u32 = 4 * CELLS_PER_METRE;
pub const PIT_LAST_X: u32 = WIDTH - (PIT_MARGIN + (PIT_ROWS - 1) / BENCH_ROWS * BENCH_ROWS) - 1;
pub const UNITS: u64 = 64_000;
pub const CELL_MASS: u64 = 1_000;
pub const MAX_ROWS: u32 = 800_000;
pub const GENERATOR_VERSION: u32 = 2;
pub fn depth(row: u32) -> u32 {
    row / CELLS_PER_METRE
}
pub fn chunk_id(x: u32, y: u32) -> u32 {
    (y / CHUNK) * CHUNKS_ACROSS + x / CHUNK
}
pub fn chunk_origin(id: u32) -> (u32, u32) {
    ((id % CHUNKS_ACROSS) * CHUNK, (id / CHUNKS_ACROSS) * CHUNK)
}
pub fn bit_index(x: u32, y: u32) -> usize {
    ((y % CHUNK) * CHUNK + x % CHUNK) as usize
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fine_cells_preserve_mass_and_chunk_coordinates() {
        assert_eq!(64 * CELL_MASS, UNITS);
        for (x, y) in [(0, 0), (63, 63), (64, 64), (511, 799999)] {
            let (cx, cy) = chunk_origin(chunk_id(x, y));
            assert_eq!(cx + bit_index(x, y) as u32 % CHUNK, x);
            assert_eq!(cy + bit_index(x, y) as u32 / CHUNK, y);
        }
    }
}
