//! Floor routes through the pit ramp and lift-connected underground portals.
use crate::{geometry::WIDTH, terrain::Terrain};
const PIT: u32 = 192;
const SHAFT: u32 = WIDTH / 2;
const CLEARANCE: u32 = 4;
fn clear(t: &Terrain, x: u32, feet: u32) -> bool {
    (feet.saturating_sub(CLEARANCE - 1)..=feet).all(|y| t.contains(x, y))
}
fn push(points: &mut Vec<[u32; 2]>, point: [u32; 2]) {
    if points.last() == Some(&point) {
        return;
    }
    if points.len() >= 2 {
        let a = points[points.len() - 2];
        let b = points[points.len() - 1];
        let ab = [b[0] as i64 - a[0] as i64, b[1] as i64 - a[1] as i64];
        let bc = [point[0] as i64 - b[0] as i64, point[1] as i64 - b[1] as i64];
        if ab[0] * bc[1] == ab[1] * bc[0] && ab[0] * bc[0] + ab[1] * bc[1] > 0 {
            points.pop();
        }
    }
    points.push(point);
}
/// Walking portals follow floors with <= 0.25m steps. Vertical movement is
/// restricted to the commissioned lift. A cutting crew stands behind its face.
pub fn route(t: &Terrain, heights: &[u32], face: [u32; 2], lift: bool) -> Vec<[u32; 2]> {
    let mut points = Vec::new();
    let mut x = face[0];
    if face[1] >= PIT {
        if !lift {
            return points;
        }
        if x.abs_diff(SHAFT) >= 4 {
            let top = (face[1] / 96 + u32::from(face[1] % 96 >= 80)) * 96;
            let direction = if x < SHAFT { 1i32 } else { -1 };
            // Portal at the last cleared column behind the active cutting area.
            let mut found = None;
            for distance in 0..=8 {
                let cx = (x as i32 + direction * distance) as u32;
                if cx >= WIDTH {
                    break;
                }
                if let Some(y) = (top + 7..top + 8).find(|&y| {
                    clear(t, cx, y) && (cx.abs_diff(SHAFT) < 4 || !t.contains(cx, y + 1))
                }) {
                    found = Some([cx, y]);
                    break;
                }
            }
            let Some(mut feet) = found else {
                return vec![];
            };
            push(&mut points, feet);
            while feet[0] != SHAFT {
                let nx = (feet[0] as i32 + direction) as u32;
                let Some(y) = (feet[1].saturating_sub(1)..=feet[1] + 1).find(|&y| {
                    clear(t, nx, y) && (nx.abs_diff(SHAFT) < 4 || !t.contains(nx, y + 1))
                }) else {
                    return vec![];
                };
                feet = [nx, y];
                push(&mut points, feet);
            }
        } else {
            // Lift cage stays in the fully excavated shaft, behind its cutting face.
            let y = face[1].saturating_sub(1).max(PIT - 1);
            push(&mut points, [SHAFT, y]);
        }
        let depth = points.last().unwrap()[1];
        if !(PIT - 1..=depth).all(|y| t.contains(SHAFT, y)) {
            return vec![];
        }
        push(&mut points, [SHAFT, PIT - 1]);
        x = SHAFT;
    }
    let mut previous = None;
    for cx in (16..=x).rev() {
        let h = heights
            .get(cx as usize)
            .copied()
            .unwrap_or(0)
            .min(PIT)
            .min(cx.saturating_sub(15));
        if h == 0 {
            return vec![];
        }
        let feet = h - 1;
        let platform = lift && feet == PIT - 1;
        if !clear(t, cx, feet)
            || (!platform && t.contains(cx, feet + 1))
            || previous.is_some_and(|y: u32| y.abs_diff(feet) > 1)
        {
            return vec![];
        }
        push(&mut points, [cx, feet]);
        previous = Some(feet);
    }
    if points.last().is_some_and(|p| p[1] != 0) {
        return vec![];
    }
    points
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ramp_reaches_daylight_but_vertical_walls_are_not_walkable() {
        let heights: Vec<_> = (0..WIDTH)
            .map(|x| if x < 16 { 0 } else { (x - 15).min(80) })
            .collect();
        let t = Terrain::from_columns(&heights);
        let path = route(&t, &heights, [200, 79], false);
        assert_eq!(path.first(), Some(&[200, 79]));
        assert_eq!(path.last(), Some(&[16, 0]));
        let wall: Vec<_> = (0..WIDTH).map(|x| if x >= 16 { 80 } else { 0 }).collect();
        let t = Terrain::from_columns(&wall);
        // A vertical lip cannot be used as an exit.
        assert!(route(&t, &wall, [200, 79], false).is_empty());
    }
    #[test]
    fn underground_requires_lift_and_clear_shaft() {
        let heights: Vec<_> = (0..WIDTH)
            .map(|x| if x < 16 { 0 } else { (x - 15).min(192) })
            .collect();
        let mut t = Terrain::from_columns(&heights);
        for y in 192..250 {
            t.excavate(SHAFT, y);
        }
        assert!(route(&t, &heights, [SHAFT, 249], false).is_empty());
        let p = route(&t, &heights, [SHAFT, 249], true);
        assert_eq!(p.last(), Some(&[16, 0]));
        assert!(p
            .windows(2)
            .any(|w| w[0][0] == SHAFT && w[1][0] == SHAFT && w[0][1] > 192));
    }
    #[test]
    fn tunnel_under_ramp_does_not_replace_surface_floor() {
        let mut heights: Vec<_> = (0..WIDTH)
            .map(|x| if x < 16 { 0 } else { (x - 15).min(192) })
            .collect();
        let mut t = Terrain::from_columns(&heights);
        for y in 192..200 {
            for x in 253..=259 {
                t.excavate(x, y);
                heights[x as usize] = y + 1;
            }
        }
        for x in (40..253).rev() {
            for y in 192..200 {
                t.excavate(x, y);
                heights[x as usize] = y + 1;
            }
        }
        let path = route(&t, &heights, [40, 199], true);
        assert!(!path.is_empty());
        assert_eq!(path.last(), Some(&[16, 0]));
        assert!(path.contains(&[256, 191]));
    }
    #[test]
    fn unfinished_drive_uses_completed_floor_behind_face() {
        let mut heights: Vec<_> = (0..WIDTH)
            .map(|x| if x < 16 { 0 } else { (x - 15).min(192) })
            .collect();
        let mut t = Terrain::from_columns(&heights);
        for y in 192..300 {
            for x in 253..=259 {
                t.excavate(x, y);
                heights[x as usize] = y + 1;
            }
        }
        for x in (251..253).rev() {
            for y in 288..296 {
                t.excavate(x, y);
                heights[x as usize] = y + 1;
            }
        }
        for y in 288..292 {
            t.excavate(250, y);
            heights[250] = y + 1;
        }
        let p = route(&t, &heights, [250, 291], true);
        assert_eq!(p.first(), Some(&[251, 295]));
        assert_eq!(p.last(), Some(&[16, 0]));
    }
}
