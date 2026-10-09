// SPDX-License-Identifier: GPL-3.0-or-later
//! Material an operation leaves behind, for display only. See docs/specs/material-left.md.
//!
//! Unreachable material is where no tool disk fits: R − opening(R, r) for a pocket or an
//! inside profile, closing(P, r) − P for an outside profile. Nothing here changes a toolpath,
//! and callers turn a failure into a note instead of an error.

use cavalier_contours::polyline::{
    BooleanOp, BooleanResult, PlineContainsResult, PlineSource, Polyline,
};
use chipload_core::Point;
use chipload_core::tolerances::{ARC_APPROX_TOL_MM, CHAIN_TOL_MM, REST_TOL_MM};

use crate::ToolpathError;
use crate::offset::flatten;
use crate::pocket::{Area, ccw, cw, encloses, first_point, geometry, guarded};

/// The gap left between the opening and the walls, so booleans never work on coincident
/// edges. Slivers thinner than this are not shown (spec section 3).
const WALL_GAP_MM: f64 = ARC_APPROX_TOL_MM;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LeftKind {
    /// No tool disk fits: inside corners, slots narrower than the tool. Normal.
    Unreachable,
    /// The tool fits but the toolpath doesn't pass close enough (also a warning).
    Missed,
}

impl LeftKind {
    pub fn name(self) -> &'static str {
        match self {
            LeftKind::Unreachable => "unreachable",
            LeftKind::Missed => "missed",
        }
    }
}

/// One piece of material left by one operation.
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialLeft {
    /// Index in `job.operations`.
    pub operation: usize,
    pub kind: LeftKind,
    /// Closed rings, arcs flattened, filled even-odd: an outline and its holes, or for missed
    /// material the squares it is made of.
    pub rings: Vec<Vec<Point>>,
}

impl MaterialLeft {
    /// (min, max) corners of the piece.
    pub fn bounds(&self) -> (Point, Point) {
        bounds_of(&self.rings)
    }
}

/// A piece before it is attached to an operation.
#[derive(Debug, Clone, PartialEq)]
pub struct LeftPiece {
    pub kind: LeftKind,
    pub rings: Vec<Vec<Point>>,
}

pub(crate) fn bounds_of(rings: &[Vec<Point>]) -> (Point, Point) {
    rings.iter().flatten().fold(
        (
            (f64::INFINITY, f64::INFINITY),
            (f64::NEG_INFINITY, f64::NEG_INFINITY),
        ),
        |((x0, y0), (x1, y1)), p| ((x0.min(p.0), y0.min(p.1)), (x1.max(p.0), y1.max(p.1))),
    )
}

/// Deterministic order: by (min y, min x).
pub(crate) fn sort_pieces(pieces: &mut [LeftPiece]) {
    pieces.sort_by(|a, b| {
        let ((ax, ay), _) = bounds_of(&a.rings);
        let ((bx, by), _) = bounds_of(&b.rings);
        ay.total_cmp(&by).then(ax.total_cmp(&bx))
    });
}

/// Where a tool of `radius` reaches, given where its centre can go (`level0`, the area
/// shrunk by the radius), grown by a small gap so it never shares an edge with a wall.
pub fn reach(level0: &[Area], radius: f64) -> Result<Vec<Area>, ToolpathError> {
    let mut areas = Vec::new();
    for area in level0 {
        areas.extend(area.shrunk(-(radius + WALL_GAP_MM))?);
    }
    Ok(areas)
}

/// Material inside `region` that the tool reaches nowhere. `reached` is everywhere the
/// operation's tool reaches ([`reach`]): this region's own pockets, and other pockets of the
/// same operation, which clear each other's corners where they cross.
pub fn unreachable_inside(
    region: &Area,
    reached: &[Area],
) -> Result<Vec<LeftPiece>, ToolpathError> {
    let mut pieces = vec![region.clone()];
    for area in reached {
        pieces = minus_area(pieces, area)?;
    }
    let pieces = keep_visible(pieces)?;
    for piece in &pieces {
        let outside = flatten(&piece.outer)
            .into_iter()
            .any(|p| region.distance(p) > ARC_APPROX_TOL_MM);
        if outside {
            return Err(geometry(
                "material left lies outside the area it was computed for".into(),
            ));
        }
    }
    Ok(finish(pieces))
}

/// Material outside the closed `contour` that no tool disk of `radius` beside it reaches:
/// the inside corners of a part cut with an outside profile.
pub fn unreachable_outside(
    contour: &Polyline<f64>,
    radius: f64,
) -> Result<Vec<LeftPiece>, ToolpathError> {
    let part = ccw(contour.clone());
    let grown = guarded("Material left offset", || part.parallel_offset(-radius))?
        .into_iter()
        .filter(|p| p.is_closed())
        .max_by(|a, b| a.area().abs().total_cmp(&b.area().abs()));
    let Some(grown) = grown else {
        return Ok(Vec::new());
    };
    let closing = Area {
        outer: ccw(grown),
        holes: Vec::new(),
    }
    .shrunk(radius + WALL_GAP_MM)?;
    let mut pieces = Vec::new();
    for area in &closing {
        pieces.extend(difference(area, &part)?);
    }
    let pieces = keep_visible(pieces)?;
    let part_area = Area::of(part.clone());
    for piece in &pieces {
        let inside = flatten(&piece.outer)
            .into_iter()
            .any(|p| part_area.contains(p) && part_area.edge_distance(p) > ARC_APPROX_TOL_MM);
        if inside {
            return Err(geometry(
                "material left lies inside the part it was computed for".into(),
            ));
        }
    }
    Ok(finish(pieces))
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

fn finish(pieces: Vec<Area>) -> Vec<LeftPiece> {
    let mut out: Vec<LeftPiece> = pieces
        .iter()
        .map(|a| LeftPiece {
            kind: LeftKind::Unreachable,
            rings: std::iter::once(&a.outer)
                .chain(&a.holes)
                .map(flatten)
                .collect(),
        })
        .collect();
    sort_pieces(&mut out);
    out
}

/// Every piece minus `area` (its outer loop minus its holes).
fn minus_area(pieces: Vec<Area>, area: &Area) -> Result<Vec<Area>, ToolpathError> {
    let mut out = Vec::new();
    for piece in &pieces {
        out.extend(difference(piece, &area.outer)?);
        for hole in &area.holes {
            out.extend(intersection(piece, &ccw(hole.clone()))?);
        }
    }
    Ok(out)
}

fn too_small(pline: &Polyline<f64>) -> bool {
    pline.area().abs() <= CHAIN_TOL_MM * CHAIN_TOL_MM
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
            .min_by(|a, b| a.outer.area().abs().total_cmp(&b.outer.area().abs()));
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
                        let mut union = guarded("Material left union", || {
                            hole.boolean(&existing, BooleanOp::Or)
                        })?;
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
        PlineContainsResult::Intersected => {
            let cut = guarded("Material left difference", || {
                piece.outer.boolean(cutter, BooleanOp::Not)
            })?;
            subtract_holes(areas_of(cut)?, &piece.holes)
        }
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
        PlineContainsResult::Intersected => {
            let common = guarded("Material left intersection", || {
                piece.outer.boolean(window, BooleanOp::And)
            })?;
            subtract_holes(areas_of(common)?, &piece.holes)
        }
        PlineContainsResult::InvalidInput => Err(invalid()),
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
    use std::f64::consts::PI;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Polyline<f64> {
        let mut p = Polyline::new_closed();
        for (x, y) in [(x0, y0), (x1, y0), (x1, y1), (x0, y1)] {
            p.add(x, y, 0.0);
        }
        p
    }

    fn area_of(piece: &LeftPiece) -> f64 {
        piece
            .rings
            .iter()
            .map(|r| chipload_core::primitives::signed_area(r))
            .sum::<f64>()
            .abs()
    }

    /// A square corner left by a tool of radius r: r²(1 − π/4), less the gap along the arc.
    fn is_corner(piece: &LeftPiece, r: f64) -> bool {
        let full = r * r * (1.0 - PI / 4.0);
        let a = area_of(piece);
        a <= full && a >= full - WALL_GAP_MM * PI / 2.0 * r - 0.005
    }

    fn inside(region: &Area, radius: f64) -> Vec<LeftPiece> {
        let level0 = region.shrunk(radius).unwrap();
        unreachable_inside(region, &reach(&level0, radius).unwrap()).unwrap()
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

    #[test]
    fn outside_profile_leaves_the_inside_corners_of_a_part() {
        let r = 3.0;
        assert!(
            unreachable_outside(&rect(0.0, 0.0, 50.0, 30.0), r)
                .unwrap()
                .is_empty()
        );
        let mut l = Polyline::new_closed();
        for (x, y) in [
            (0.0, 0.0),
            (50.0, 0.0),
            (50.0, 20.0),
            (20.0, 20.0),
            (20.0, 40.0),
            (0.0, 40.0),
        ] {
            l.add(x, y, 0.0);
        }
        let pieces = unreachable_outside(&l, r).unwrap();
        assert_eq!(pieces.len(), 1);
        let ((x0, y0), (x1, y1)) = pieces[0].bounds_check();
        assert!((x0 - 20.0).abs() < 1e-6 && (y0 - 20.0).abs() < 1e-6);
        assert!(x1 < 20.0 + r && y1 < 20.0 + r);
        assert!(is_corner(&pieces[0], r), "{}", area_of(&pieces[0]));
        // Clockwise input gives the same answer.
        let mut reversed = l.clone();
        reversed.invert_direction_mut();
        assert_eq!(unreachable_outside(&reversed, r).unwrap(), pieces);
    }

    #[test]
    fn a_bay_narrower_than_the_tool_is_unreachable_from_outside() {
        // A U shape whose 4 mm gap is narrower than the tool (D 6).
        let mut u = Polyline::new_closed();
        for (x, y) in [
            (0.0, 0.0),
            (24.0, 0.0),
            (24.0, 30.0),
            (14.0, 30.0),
            (14.0, 10.0),
            (10.0, 10.0),
            (10.0, 30.0),
            (0.0, 30.0),
        ] {
            u.add(x, y, 0.0);
        }
        let pieces = unreachable_outside(&u, 3.0).unwrap();
        assert_eq!(pieces.len(), 1);
        // The 4 x 20 bay, less the dip of a tool disk into its mouth (under 1 mm).
        assert!(area_of(&pieces[0]) > 4.0 * 19.0, "{}", area_of(&pieces[0]));
    }

    impl LeftPiece {
        fn bounds_check(&self) -> (Point, Point) {
            bounds_of(&self.rings)
        }
    }
}
