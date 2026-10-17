// SPDX-License-Identifier: GPL-3.0-or-later
//! What a tool leaves in a pocket, for rest machining (docs/specs/rest-machining.md): the
//! region minus the area a disk of the tool's radius reaches in it, worked out with
//! cavalier_contours' booleans, checked by area and nudged when they misjudge a near
//! coincidence. Until 0.14 this also drew the material left on the canvas
//! (docs/specs/material-left.md); the simulation does that now
//! (docs/specs/material-left-merge.md).

use cavalier_contours::polyline::{
    BooleanOp, BooleanResult, PlineContainsResult, PlineSource, Polyline,
};
use chipload_core::order::Ordered;
use chipload_core::tolerances::{
    ARC_APPROX_TOL_MM, CHAIN_TOL_MM, OFFSET_CHECK_SLACK_MM, REST_TOL_MM,
};

use crate::ToolpathError;
use crate::cavalier::{
    Area, boolean, ccw, cw, encloses, first_point, geometry, guarded, offset_loops, too_small,
};
use crate::offset::flatten;

/// The gap left between the opening and the walls, so booleans never work on coincident
/// edges. Slivers thinner than this are not shown (spec section 3).
const WALL_GAP_MM: f64 = ARC_APPROX_TOL_MM;
/// How far a point of material left may lie outside the area it was computed for: the
/// flattening of arcs, plus float slack (a point exactly on the tolerance is not outside).
const OUTSIDE_LIMIT_MM: f64 = ARC_APPROX_TOL_MM + OFFSET_CHECK_SLACK_MM;
/// How much area a boolean may gain or lose against what it must give, mm²: a square of
/// the wall gap. Pieces this small are dropped anyway (`too_small`, `keep_visible`).
const AREA_SLACK_MM2: f64 = WALL_GAP_MM * WALL_GAP_MM;

/// Where a tool of `radius` reaches, given where its centre can go (`level0`, the area
/// shrunk by the radius), grown by a small gap so it never shares an edge with a wall.
pub fn reach(level0: &[Area], radius: f64) -> Result<Vec<Area>, ToolpathError> {
    let mut areas = Vec::new();
    for area in level0 {
        areas.extend(area.grown(radius + WALL_GAP_MM)?);
    }
    Ok(areas)
}

/// Rest machining (docs/specs/rest-machining.md §2): the material inside `region` a tool
/// whose centre can go anywhere in `level0` (the region shrunk by its offset) leaves, as
/// areas. The opening keeps the wall gap, as for the display.
pub fn left_by(region: &Area, level0: &[Area], radius: f64) -> Result<Vec<Area>, ToolpathError> {
    let reached = reach(level0, radius)?;
    let mut pieces = vec![region.clone()];
    for area in &reached {
        pieces = minus_area(pieces, area)?;
    }
    keep_visible(pieces)
}

/// The areas a rest pocket clears (docs/specs/rest-machining.md §3): `region` within
/// `grow` (a tool diameter) of the material `left` in it, plus the wall gap. Grown together, so pieces that come near each
/// other become one; a hole the offset library drops only adds air to cut. Every area is
/// checked to lie inside `region`.
pub fn rest_areas(region: &Area, left: &[Area], grow: f64) -> Result<Vec<Area>, ToolpathError> {
    if left.is_empty() {
        return Ok(Vec::new());
    }
    let loops = left
        .iter()
        .flat_map(|a| std::iter::once(ccw(a.outer.clone())).chain(a.holes.iter().cloned().map(cw)));
    let grown = offset_loops(loops, -(grow + WALL_GAP_MM))?;
    let mut areas = Vec::new();
    for piece in &grown {
        areas.extend(subtract_holes(
            intersection(piece, &ccw(region.outer.clone()))?,
            &region.holes,
        )?);
    }
    for area in &areas {
        let outside = flatten(&area.outer)
            .into_iter()
            .any(|p| region.distance(p) > OUTSIDE_LIMIT_MM);
        if outside {
            return Err(geometry(
                "the rest machining area lies outside the pocket".into(),
            ));
        }
    }
    Ok(areas)
}

/// The area meant to be cleared once the clearance is applied: grown by a positive
/// clearance, shrunk by a negative one (docs/specs/depths-and-clearance.md). Material left
/// is judged against this, so stock left on purpose isn't shown as unreachable.
pub fn cleared(region: &Area, clearance: f64) -> Result<Vec<Area>, ToolpathError> {
    if clearance == 0.0 {
        return Ok(vec![region.clone()]);
    }
    region.grown(clearance)
}

/// Drop pieces that hold no disk of diameter REST_TOL_MM (chord and float noise).
fn keep_visible(pieces: Vec<Area>) -> Result<Vec<Area>, ToolpathError> {
    let mut kept = Vec::new();
    for piece in pieces {
        if !piece.shrunk(REST_TOL_MM / 2.0)?.is_empty() {
            kept.push(piece);
        }
    }
    Ok(kept)
}

/// Every piece minus `area` (its outer loop minus its holes).
pub(crate) fn minus_area(pieces: Vec<Area>, area: &Area) -> Result<Vec<Area>, ToolpathError> {
    let mut out = Vec::new();
    for piece in &pieces {
        out.extend(difference(piece, &area.outer)?);
        for hole in &area.holes {
            out.extend(intersection(piece, &ccw(hole.clone()))?);
        }
    }
    Ok(out)
}

/// Boolean result as areas: positive loops are outlines, negative ones their holes.
fn areas_of(result: BooleanResult<Polyline<f64>>) -> Result<Vec<Area>, ToolpathError> {
    let mut areas: Vec<Area> = result
        .pos_plines
        .into_iter()
        .map(|p| p.pline)
        .filter(|p| !too_small(p))
        .map(|p| Area::of(ccw(p)))
        .collect();
    for hole in result.neg_plines.into_iter().map(|p| p.pline) {
        if too_small(&hole) {
            continue;
        }
        let inside = first_point(&hole);
        let parent = areas
            .iter_mut()
            .filter(|a| encloses(&a.outer, inside))
            .min_by_key(|a| Ordered(a.outer.area().abs()));
        match parent {
            Some(area) => area.holes.push(cw(hole)),
            None => return Err(geometry("material left: a hole outside every piece".into())),
        }
    }
    Ok(areas)
}

/// `piece` minus the inside of the counter-clockwise loop `cutter`.
fn difference(piece: &Area, cutter: &Polyline<f64>) -> Result<Vec<Area>, ToolpathError> {
    match piece.outer.contains(cutter) {
        PlineContainsResult::Disjoint => Ok(vec![piece.clone()]),
        PlineContainsResult::Pline1InsidePline2 => Ok(Vec::new()),
        PlineContainsResult::Pline2InsidePline1 => {
            let mut hole = cutter.clone();
            let mut holes = Vec::new();
            for existing in &piece.holes {
                let existing = ccw(existing.clone());
                match hole.contains(&existing) {
                    PlineContainsResult::Disjoint => holes.push(cw(existing)),
                    PlineContainsResult::Pline2InsidePline1 => {}
                    PlineContainsResult::Pline1InsidePline2 => hole = existing,
                    PlineContainsResult::Intersected => {
                        let mut union =
                            boolean("Material left union", &hole, &existing, BooleanOp::Or)?;
                        if union.pos_plines.len() != 1 || !union.neg_plines.is_empty() {
                            return Err(geometry("material left: holes enclose an area".into()));
                        }
                        hole = ccw(union.pos_plines.swap_remove(0).pline);
                    }
                    PlineContainsResult::InvalidInput => return Err(invalid()),
                }
            }
            holes.push(cw(hole));
            Ok(vec![Area {
                outer: piece.outer.clone(),
                holes,
            }])
        }
        PlineContainsResult::Intersected => subtract_holes(
            checked_boolean(&piece.outer, cutter, BooleanOp::Not)?,
            &piece.holes,
        ),
        PlineContainsResult::InvalidInput => Err(invalid()),
    }
}

/// `piece` inside the counter-clockwise loop `window`.
fn intersection(piece: &Area, window: &Polyline<f64>) -> Result<Vec<Area>, ToolpathError> {
    match piece.outer.contains(window) {
        PlineContainsResult::Disjoint => Ok(Vec::new()),
        PlineContainsResult::Pline1InsidePline2 => Ok(vec![piece.clone()]),
        PlineContainsResult::Pline2InsidePline1 => {
            subtract_holes(vec![Area::of(window.clone())], &piece.holes)
        }
        PlineContainsResult::Intersected => subtract_holes(
            checked_boolean(&piece.outer, window, BooleanOp::And)?,
            &piece.holes,
        ),
        PlineContainsResult::InvalidInput => Err(invalid()),
    }
}

/// `outer` minus (Not) or inside (And) the counter-clockwise loop `other`, checked by area:
/// a vertex lying almost on the other loop's edge can put a piece on the wrong side. Then
/// `other` is moved out by `CHAIN_TOL_MM` (the result shrinks by that, invisibly) and it is
/// tried once more; a second failure is an error, which the caller turns into a note.
fn checked_boolean(
    outer: &Polyline<f64>,
    other: &Polyline<f64>,
    op: BooleanOp,
) -> Result<Vec<Area>, ToolpathError> {
    let first = boolean_areas(outer, other, op)?;
    if area_is_plausible(outer, other, op, &first) {
        return Ok(first);
    }
    // Not: a larger cutter; And: a smaller window. Either way the result only shrinks.
    let nudge = match op {
        BooleanOp::Not => -CHAIN_TOL_MM,
        _ => CHAIN_TOL_MM,
    };
    let moved = guarded("Material left offset", || {
        ccw(other.clone()).parallel_offset(nudge)
    })?
    .into_iter()
    .filter(|p| p.is_closed())
    .max_by_key(|p| Ordered(p.area().abs()))
    .map(ccw)
    .ok_or_else(invalid)?;
    let second = boolean_areas(outer, &moved, op)?;
    if area_is_plausible(outer, &moved, op, &second) {
        Ok(second)
    } else {
        Err(geometry(
            "material left: a boolean gave the wrong area".into(),
        ))
    }
}

fn boolean_areas(
    outer: &Polyline<f64>,
    other: &Polyline<f64>,
    op: BooleanOp,
) -> Result<Vec<Area>, ToolpathError> {
    let result = boolean("Material left boolean", outer, other, op)?;
    areas_of(result)
}

/// A − B keeps at least |A| − |B|; A ∩ B holds at most the smaller of the two.
fn area_is_plausible(
    outer: &Polyline<f64>,
    other: &Polyline<f64>,
    op: BooleanOp,
    result: &[Area],
) -> bool {
    let size = |p: &Polyline<f64>| p.area().abs();
    let total: f64 = result
        .iter()
        .map(|a| size(&a.outer) - a.holes.iter().map(size).sum::<f64>())
        .sum();
    match op {
        BooleanOp::Not => total + size(other) + AREA_SLACK_MM2 >= size(outer),
        _ => total <= size(outer).min(size(other)) + AREA_SLACK_MM2,
    }
}

fn subtract_holes(
    mut areas: Vec<Area>,
    holes: &[Polyline<f64>],
) -> Result<Vec<Area>, ToolpathError> {
    for hole in holes {
        let cutter = ccw(hole.clone());
        let mut next = Vec::new();
        for area in &areas {
            next.extend(difference(area, &cutter)?);
        }
        areas = next;
    }
    Ok(areas)
}

fn invalid() -> ToolpathError {
    geometry("material left: invalid geometry from the offset library".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cavalier_contours::polyline::PlineSourceMut;
    use chipload_core::Point;
    use std::f64::consts::PI;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Polyline<f64> {
        let mut p = Polyline::new_closed();
        for (x, y) in [(x0, y0), (x1, y0), (x1, y1), (x0, y1)] {
            p.add(x, y, 0.0);
        }
        p
    }

    /// A leftover as rings (its outline, then its holes), flattened, for the checks below.
    struct Piece {
        rings: Vec<Vec<Point>>,
    }

    impl Piece {
        fn bounds_check(&self) -> (Point, Point) {
            self.rings.iter().flatten().fold(
                (
                    (f64::INFINITY, f64::INFINITY),
                    (f64::NEG_INFINITY, f64::NEG_INFINITY),
                ),
                |((x0, y0), (x1, y1)), p| ((x0.min(p.0), y0.min(p.1)), (x1.max(p.0), y1.max(p.1))),
            )
        }
    }

    fn area_of(piece: &Piece) -> f64 {
        piece
            .rings
            .iter()
            .map(|r| chipload_core::primitives::signed_area(r))
            .sum::<f64>()
            .abs()
    }

    /// A square corner left by a tool of radius r: r²(1 − π/4), less the gap along the arc.
    fn is_corner(piece: &Piece, r: f64) -> bool {
        let full = r * r * (1.0 - PI / 4.0);
        let a = area_of(piece);
        a <= full && a >= full - WALL_GAP_MM * PI / 2.0 * r - 0.005
    }

    /// What a tool of `radius` leaves in `region`, by (min y, min x).
    fn inside(region: &Area, radius: f64) -> Vec<Piece> {
        let level0 = region.shrunk(radius).unwrap();
        let mut pieces: Vec<Piece> = left_by(region, &level0, radius)
            .unwrap()
            .iter()
            .map(|a| Piece {
                rings: std::iter::once(&a.outer)
                    .chain(&a.holes)
                    .map(flatten)
                    .collect(),
            })
            .collect();
        pieces.sort_by(|a, b| {
            let ((ax, ay), _) = a.bounds_check();
            let ((bx, by), _) = b.bounds_check();
            ay.total_cmp(&by).then(ax.total_cmp(&bx))
        });
        pieces
    }

    #[test]
    fn growing_keeps_the_hole_round_two_close_islands() {
        // From the material-left property test: islands closer than the tool diameter share
        // one hole in the tool centre's area. cavalier's shape offset lost that hole when
        // growing it back by r + 0.01 mm, so the notches of the islands didn't show.
        let island = |pts: &[(f64, f64)]| {
            let mut p = Polyline::new_closed();
            for &(x, y) in pts {
                p.add(x, y, 0.0);
            }
            cw(p)
        };
        let i2 = island(&[
            (-3.111864, -7.41021),
            (-3.192688, -4.992429),
            (-5.475167, -6.646518),
            (-8.258502, -6.993821),
            (-8.810572, -7.999693),
            (-7.274977, -7.848187),
            (-7.121847, -8.04851),
            (-7.896631, -11.134579),
            (-6.424539, -10.236057),
            (-4.821013, -9.548028),
            (-4.504654, -9.040614),
            (-4.078219, -8.945432),
            (-3.30825, -8.50243),
            (-4.059942, -7.929354),
        ]);
        let i3 = island(&[
            (3.114433, -9.925338),
            (5.602391, -8.681575),
            (2.942974, -6.451198),
            (2.438971, -6.096676),
            (1.189276, -7.65624),
            (0.478617, -8.078924),
            (-0.41388, -7.902225),
            (-1.156058, -8.495761),
            (1.117258, -10.853473),
            (-0.031204, -12.162694),
            (0.808376, -12.739233),
            (1.478249, -12.46676),
            (2.936997, -12.135961),
            (3.276493, -11.719524),
        ]);
        let region = Area {
            outer: ccw(rect(-30.0, -30.0, 30.0, 30.0)),
            holes: vec![i2, i3],
        };
        let r = 2.7918711862602326;
        let level0 = region.shrunk(r).unwrap();
        assert_eq!(level0.len(), 1);
        assert_eq!(level0[0].holes.len(), 1, "one hole round both islands");
        for extra in [0.0, 0.005, WALL_GAP_MM, 0.05, 0.1] {
            let grown = level0[0].grown(r + extra).unwrap();
            assert_eq!(grown.len(), 1);
            assert_eq!(grown[0].holes.len(), 1, "grown by r + {extra}");
        }
        // The notch of island 2 at (-8.5, -8) is out of reach and shown.
        let pieces = inside(&region, r);
        assert!(
            pieces
                .iter()
                .any(|piece| Area::of(ring_pline(&piece.rings[0])).contains((-8.5, -8.0))),
            "{} pieces",
            pieces.len()
        );
    }

    fn ring_pline(ring: &[Point]) -> Polyline<f64> {
        let mut p = Polyline::new_closed();
        for &(x, y) in ring {
            p.add(x, y, 0.0);
        }
        ccw(p)
    }

    #[test]
    fn square_pocket_leaves_four_corners() {
        let r = 3.0;
        let pieces = inside(&Area::of(rect(0.0, 0.0, 50.0, 30.0)), r);
        assert_eq!(pieces.len(), 4);
        for piece in &pieces {
            assert!(is_corner(piece, r), "{}", area_of(piece));
        }
        let ((x0, y0), _) = pieces[0].bounds_check();
        assert!(x0.abs() < 1e-6 && y0.abs() < 1e-6);
    }

    #[test]
    fn rounded_corners_at_least_the_tool_radius_leave_nothing() {
        // A stadium: radius 5 ends, tool radius 5 and 4.
        let mut p = Polyline::new_closed();
        p.add(5.0, 0.0, 0.0);
        p.add(25.0, 0.0, 1.0);
        p.add(25.0, 10.0, 0.0);
        p.add(5.0, 10.0, 1.0);
        let region = Area::of(p);
        assert!(inside(&region, 4.0).is_empty());
        assert!(inside(&region, 5.0 - 2.0 * CHAIN_TOL_MM).is_empty());
    }

    #[test]
    fn a_slot_narrower_than_the_tool_is_unreachable() {
        // A 40 x 40 pocket with a 4 mm wide slot sticking out to the right; tool D 6.
        let mut p = Polyline::new_closed();
        for (x, y) in [
            (0.0, 0.0),
            (40.0, 0.0),
            (40.0, 18.0),
            (60.0, 18.0),
            (60.0, 22.0),
            (40.0, 22.0),
            (40.0, 40.0),
            (0.0, 40.0),
        ] {
            p.add(x, y, 0.0);
        }
        let pieces = inside(&Area::of(p), 3.0);
        let slot = pieces
            .iter()
            .find(|piece| piece.bounds_check().1.0 > 59.0)
            .unwrap();
        // The whole 20 x 4 slot, less the bit a tool disk bulges into its mouth.
        assert!(area_of(slot) > 80.0 - 3.0, "{}", area_of(slot));
        assert_eq!(pieces.len(), 5); // the slot and the pocket's four corners
    }

    #[test]
    fn convex_island_corners_leave_nothing() {
        let region = Area {
            outer: ccw(rect(0.0, 0.0, 60.0, 60.0)),
            holes: vec![cw(rect(20.0, 20.0, 40.0, 40.0))],
        };
        // Only the four outer corners.
        assert_eq!(inside(&region, 3.0).len(), 4);
    }

    #[test]
    fn inside_corners_of_an_island_are_unreachable() {
        // An L-shaped island has one inside (concave) corner, seen from the pocket.
        let mut island = Polyline::new_closed();
        for (x, y) in [
            (20.0, 20.0),
            (40.0, 20.0),
            (40.0, 25.0),
            (25.0, 25.0),
            (25.0, 40.0),
            (20.0, 40.0),
        ] {
            island.add(x, y, 0.0);
        }
        let region = Area {
            outer: ccw(rect(0.0, 0.0, 60.0, 60.0)),
            holes: vec![cw(island)],
        };
        let pieces = inside(&region, 2.0);
        assert_eq!(pieces.len(), 5);
        assert!(pieces.iter().any(|p| {
            let ((x0, y0), _) = p.bounds_check();
            (x0 - 25.0).abs() < 1e-6 && (y0 - 25.0).abs() < 1e-6
        }));
    }

    /// A pocket whose island has a shallow notch: the reach's hole has a vertex almost on
    /// the island's edge there, and the boolean put the notch on the wrong side (found by
    /// the material-left property on Windows CI). The notch must still be shown.
    #[test]
    fn a_notch_beside_a_near_coincident_vertex_is_shown() {
        let poly = |pts: &[(f64, f64)]| {
            let mut p = Polyline::new_closed();
            for &(x, y) in pts {
                p.add(x, y, 0.0);
            }
            p
        };
        let outer = poly(&[
            (40.945449, 1.32253),
            (22.753549, 26.239641),
            (-42.535571, 18.67181),
            (-32.319766, -32.038946),
            (56.683192, 0.0),
        ]);
        let island = poly(&[
            (-8.363385, 9.840549),
            (-8.734276, 9.791431),
            (-6.501682, 10.832788),
            (-9.93716, 12.669076),
            (-10.264582, 11.860188),
            (-10.459567, 10.328525),
            (-11.547827, 11.41166),
            (-12.785683, 9.35055),
            (-11.61, 9.29592),
            (-12.389582, 7.846854),
            (-7.90973, 6.515219),
            (-8.38772, 8.56454),
            (-7.64152, 8.615552),
            (-6.344734, 8.167195),
        ]);
        let neighbour = poly(&[
            (-0.948688, 5.340228),
            (-2.1745, 5.065788),
            (-5.478237, 5.044509),
            (-4.668653, 4.167576),
            (-4.717223, 3.503425),
            (-3.989023, 0.314792),
            (-3.505816, 1.988827),
            (-1.717394, 3.061823),
            (-0.227616, 2.313822),
        ]);
        let region = Area {
            outer: ccw(outer),
            holes: vec![cw(island), cw(neighbour)],
        };
        let radius = 4.130773022615462;
        let pieces = inside(&region, radius);
        let notch = (-12.5, 9.0);
        let shown = pieces.iter().any(|piece| {
            let rings: Vec<Polyline<f64>> = piece.rings.iter().map(|r| poly(r)).collect();
            rings.iter().filter(|r| encloses(r, notch)).count() % 2 == 1
        });
        assert!(shown, "the notch at {notch:?} is not shown");
    }
}
