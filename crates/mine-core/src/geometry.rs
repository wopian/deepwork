//! Physical scale and width-independent sparse coordinate keys.
pub const CHUNK: i64 = 64;
/// Surface district width only. Underground has no district wall.
pub const WIDTH: i64 = 512;
pub const CELLS_PER_METRE: i64 = 4;
pub const PIT_ROWS: i64 = 1;
pub const BENCH_ROWS: i64 = 24;
pub const PIT_MARGIN: i64 = 16;
pub const PIT_LAST_X: i64 = WIDTH - PIT_MARGIN - 1;
pub const UNITS: u64 = 64_000;
pub const CELL_MASS: u64 = 1_000;
pub const MAX_ROWS: i64 = 800_000;
pub const MAX_X: i64 = 8_000_000;
pub const GENERATOR_VERSION: u32 = 5;
pub fn valid_cell(x: i64, y: i64) -> bool {
    (-MAX_X..MAX_X).contains(&x) && (0..MAX_ROWS).contains(&y)
}
pub fn protects_ramp(_x: i64, _y: i64) -> bool {
    false
}
pub fn depth(row: i64) -> u32 {
    (row.max(0) / CELLS_PER_METRE) as u32
}
/// Row-major keys stay below JavaScript's exact integer limit at supported depths.
pub fn cell_key(x: i64, y: i64) -> i64 {
    (y << 32) | ((x as i32 as u32 ^ 0x80000000) as i64)
}
pub fn cell_point(key: i64) -> [i64; 2] {
    [((key as u32 ^ 0x80000000) as i32) as i64, key >> 32]
}
pub fn chunk_id(x: i64, y: i64) -> i64 {
    cell_key(x.div_euclid(CHUNK), y.div_euclid(CHUNK))
}
pub fn chunk_origin(id: i64) -> (i64, i64) {
    let [x, y] = cell_point(id);
    (x * CHUNK, y * CHUNK)
}
pub fn valid_chunk(id: i64) -> bool {
    let (x, y) = chunk_origin(id);
    valid_cell(x, y) && chunk_id(x, y) == id
}
pub fn bit_index(x: i64, y: i64) -> usize {
    (y.rem_euclid(CHUNK) * CHUNK + x.rem_euclid(CHUNK)) as usize
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signed_keys_are_exact_and_independent_of_surface_width() {
        for (x, y) in [
            (-8000000, 0),
            (-65, 64),
            (-1, 799999),
            (0, 0),
            (511, 64),
            (4096, 128),
        ] {
            assert_eq!(cell_point(cell_key(x, y)), [x, y]);
            let (cx, cy) = chunk_origin(chunk_id(x, y));
            let bit = bit_index(x, y) as i64;
            assert_eq!((cx + bit % 64, cy + bit / 64), (x, y));
            assert!(cell_key(x, y) < (1_i64 << 53));
        }
        assert_eq!(64 * CELL_MASS, UNITS);
    }
}
