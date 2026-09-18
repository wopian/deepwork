//! Floor routes through the pit ramp and lift-connected underground portals.
use crate::{geometry::WIDTH, terrain::Terrain};
const PIT: u32 = crate::geometry::PIT_ROWS;
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
    let x = face[0];
    // Underground callers must use the persisted passage graph below.
    if face[1] >= PIT {
        return points;
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
    fn commissioned_root_reaches_surface_without_a_pit_ramp() {
        let t = Terrain::default();
        let mut w = crate::workings::Workings::default();
        w.initialise();
        assert_eq!(
            underground(&t, &vec![0; WIDTH as usize], &w),
            vec![[SHAFT, 0]]
        );
    }
    #[test]
    fn arbitrary_vertical_rock_is_not_a_lift() {
        let t = Terrain::default();
        assert!(route(&t, &vec![0; WIDTH as usize], [SHAFT, 300], true).is_empty());
    }
}
/// Only constructed, supported edges enter navigation; no inferred periodic levels.
pub fn underground(
    t: &Terrain,
    _heights: &[u32],
    workings: &crate::workings::Workings,
) -> Vec<[u32; 2]> {
    let path = workings.working_route();
    if path.is_empty() {
        return vec![];
    }
    for pair in path.windows(2) {
        let lift = workings.is_lift_edge(pair[0], pair[1]);
        if !crate::workings::cut_cells(pair[0], pair[1], lift)
            .iter()
            .all(|p| t.contains(p[0], p[1]))
        {
            return vec![];
        }
    }
    let mut result = vec![];
    for p in path {
        push(&mut result, p);
    }
    result.dedup();
    if result.last() != Some(&[SHAFT, 0]) {
        return vec![];
    }
    result
}
