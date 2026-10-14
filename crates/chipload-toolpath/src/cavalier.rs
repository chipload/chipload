// SPDX-License-Identifier: GPL-3.0-or-later
//! Every call into cavalier_contours (ADR 0006) for areas: offsets of a shape (an outer
//! loop with holes), of a single loop, and booleans, with the library's quirks handled in
//! one place. A panic inside the library becomes an error; a loop it returns open where a
//! closed one is expected is closed or refused; an offset that fails by a numerical
//! coincidence is retried a nudge farther (docs/specs/offset-retry.md). Nothing here is
//! trusted blindly: the planners verify every path afterwards.

use std::panic::{AssertUnwindSafe, catch_unwind};

use cavalier_contours::core::math::Vector2;
use cavalier_contours::polyline::{
    BooleanOp, BooleanResult, PlineContainsResult, PlineOrientation, PlineSource, PlineSourceMut,
    Polyline,
};
use cavalier_contours::shape_algorithms::{Shape, ShapeOffsetOptions};
use chipload_core::Point;
use chipload_core::order::Ordered;
use chipload_core::primitives::{Primitive, distance};
use chipload_core::tolerances::{CHAIN_TOL_MM, OFFSET_TIGHT_SLICE_EPS_MM};

use crate::ToolpathError;
use crate::offset::{pline_segments, point_distance};
use crate::retry::NUDGES;

pub(crate) fn geometry(message: String) -> ToolpathError {
    ToolpathError::Geometry(message)
}

/// A loop too small to be a path: within float noise of no area at all.
pub(crate) fn too_small(pline: &Polyline<f64>) -> bool {
    pline.area().abs() <= CHAIN_TOL_MM * CHAIN_TOL_MM
}

/// `a` combined with `b` by `op`; a panic inside the library is an error named `what`.
pub(crate) fn boolean(
    what: &str,
    a: &Polyline<f64>,
    b: &Polyline<f64>,
    op: BooleanOp,
) -> Result<BooleanResult<Polyline<f64>>, ToolpathError> {
    guarded(what, || a.boolean(b, op))
}

/// Run a cavalier operation; a panic inside the library becomes a GeometryError.
pub(crate) fn guarded<T>(what: &str, f: impl FnOnce() -> T) -> Result<T, ToolpathError> {
    catch_unwind(AssertUnwindSafe(f))
        .map_err(|_| geometry(format!("{what} failed inside the geometry library")))
}

pub(crate) fn ccw(mut pline: Polyline<f64>) -> Polyline<f64> {
    if pline.orientation() == PlineOrientation::Clockwise {
        pline.invert_direction_mut();
    }
    pline
}

pub(crate) fn cw(mut pline: Polyline<f64>) -> Polyline<f64> {
    if pline.orientation() == PlineOrientation::CounterClockwise {
        pline.invert_direction_mut();
    }
    pline
}

pub(crate) fn first_point(pline: &Polyline<f64>) -> Point {
    let v = pline.at(0);
    (v.x, v.y)
}

pub(crate) fn encloses(pline: &Polyline<f64>, p: Point) -> bool {
    pline.winding_number(Vector2::new(p.0, p.1)) != 0
}

fn min_corner(pline: &Polyline<f64>) -> Point {
    pline.extents().map_or((0.0, 0.0), |e| (e.min_x, e.min_y))
}

/// An outer loop (counter-clockwise) minus holes (clockwise), all with true arcs.
#[derive(Debug, Clone)]
pub struct Area {
    pub outer: Polyline<f64>,
    pub holes: Vec<Polyline<f64>>,
}

impl Area {
    /// An area without holes.
    pub fn of(outer: Polyline<f64>) -> Self {
        Area {
            outer,
            holes: Vec::new(),
        }
    }

    pub(crate) fn contains(&self, p: Point) -> bool {
        encloses(&self.outer, p) && !self.holes.iter().any(|h| encloses(h, p))
    }

    pub(crate) fn boundary(&self) -> Vec<Primitive> {
        let mut prims = pline_segments(&self.outer);
        for hole in &self.holes {
            prims.extend(pline_segments(hole));
        }
        prims
    }

    /// Distance from `p` to the area: 0 inside.
    pub(crate) fn distance(&self, p: Point) -> f64 {
        if self.contains(p) {
            return 0.0;
        }
        self.edge_distance(p)
    }

    /// Distance from `p` to the nearest edge, inside or out.
    pub(crate) fn edge_distance(&self, p: Point) -> f64 {
        self.boundary()
            .iter()
            .map(|s| point_distance(s, p))
            .fold(f64::INFINITY, f64::min)
    }

    /// (min y, min x) of the outer loop, for deterministic ordering.
    pub(crate) fn sort_key(&self) -> (Ordered, Ordered) {
        let (x, y) = min_corner(&self.outer);
        (Ordered(y), Ordered(x))
    }

    /// The area shrunk by `d`: every part farther than `d` from the boundary.
    /// Negative `d` grows it.
    pub(crate) fn shrunk(&self, d: f64) -> Result<Vec<Area>, ToolpathError> {
        self.shrunk_with(d, false)
    }

    /// [`Area::shrunk`], with the tight slice tolerance when `tight` (a pocket's last retry).
    pub(crate) fn shrunk_with(&self, d: f64, tight: bool) -> Result<Vec<Area>, ToolpathError> {
        let plines = std::iter::once(self.outer.clone()).chain(self.holes.iter().cloned());
        offset_loops_with(plines, d, tight)
    }
}

/// Outer loops (counter-clockwise) and holes (clockwise) offset as one shape by `d`
/// (positive shrinks, negative grows), as areas sorted by (y, x).
pub(crate) fn offset_loops(
    plines: impl Iterator<Item = Polyline<f64>>,
    d: f64,
) -> Result<Vec<Area>, ToolpathError> {
    offset_loops_with(plines, d, false)
}

fn offset_loops_with(
    plines: impl Iterator<Item = Polyline<f64>>,
    d: f64,
    tight: bool,
) -> Result<Vec<Area>, ToolpathError> {
    let plines: Vec<Polyline<f64>> = plines.collect();
    match offset_once(&plines, d, tight) {
        // An island across an area is the loose slice tolerance's doing: the tight one gets
        // it right (docs/specs/offset-retry.md §6).
        Err(error) if !tight => offset_once(&plines, d, true).map_err(|_| error),
        result => result,
    }
}

fn offset_once(plines: &[Polyline<f64>], d: f64, tight: bool) -> Result<Vec<Area>, ToolpathError> {
    let shape = Shape::from_plines(plines.iter().cloned());
    let offset = guarded("Pocket offset", || {
        let mut options = ShapeOffsetOptions::new();
        if tight {
            options.offset_dist_eps = OFFSET_TIGHT_SLICE_EPS_MM;
        }
        shape.parallel_offset(d, &options)
    })?;
    let keep = |p: &Polyline<f64>| !too_small(p);
    let mut areas: Vec<Area> = offset
        .ccw_plines
        .into_iter()
        .map(|p| p.polyline)
        .filter(keep)
        .map(|outer| Area {
            outer,
            holes: Vec::new(),
        })
        .collect();
    for hole in offset
        .cw_plines
        .into_iter()
        .map(|p| p.polyline)
        .filter(keep)
    {
        let inside = first_point(&hole);
        let parent = areas
            .iter_mut()
            .filter(|a| encloses(&a.outer, inside))
            .min_by_key(|a| Ordered(a.outer.area().abs()));
        if let Some(area) = parent {
            area.holes.push(hole);
            continue;
        }
        // The shape offset can return an island's offset after the area around it has
        // shrunk away (a rest area shrunk to scraps beside a grown island). An island that
        // neither crosses nor holds any area takes nothing away from them.
        let apart = |a: &Area| matches!(a.outer.contains(&hole), PlineContainsResult::Disjoint);
        if !areas.iter().all(apart) {
            return Err(geometry(
                "pocket offset produced an island outside every area".into(),
            ));
        }
    }
    areas.sort_by_key(Area::sort_key);
    Ok(areas)
}

impl Area {
    /// The area grown by `d` (≥ 0): the outline offset outwards and each hole shrunk, one
    /// loop at a time. Growing can't make two loops meet (the outline moves away from the
    /// holes, and shrinking holes move apart), so nothing is lost by not offsetting them as
    /// one shape, and cavalier's shape offset drops a hole at some distances (two islands
    /// whose offsets merge lost their hole when grown by r + 0.005 to r + 0.05 mm, not by r
    /// or r + 0.1).
    pub(crate) fn grown(&self, d: f64) -> Result<Vec<Area>, ToolpathError> {
        if d <= 0.0 {
            return self.shrunk(-d);
        }
        let keep = |p: &Polyline<f64>| !too_small(p);
        let mut outers: Vec<Polyline<f64>> = Vec::new();
        let mut holes: Vec<Polyline<f64>> = Vec::new();
        // Positive offsets go inwards for counter-clockwise loops.
        let outline = ccw(self.outer.clone());
        for loop_ in closed_loops(guarded("Pocket offset", || outline.parallel_offset(-d))?)? {
            if !keep(&loop_) {
                continue;
            }
            // A bay that closes up while growing leaves a clockwise void.
            if loop_.area() > 0.0 {
                outers.push(loop_);
            } else {
                holes.push(loop_);
            }
        }
        for hole in &self.holes {
            let inside = ccw(hole.clone());
            let raw = guarded("Pocket offset", || inside.parallel_offset(d))?;
            let loops = match closed_loops(raw) {
                Ok(loops) => loops,
                // A hole that touches itself can come back as open scraps (a piece with
                // its ends millimetres apart). Shrinking it as an area of its own goes
                // through the shape offset, which handles that; one loop shrunk makes no
                // holes.
                Err(_) => shrunk_loop(&inside, d)?,
            };
            for loop_ in loops {
                if keep(&loop_) {
                    holes.push(cw(loop_));
                }
            }
        }
        let mut areas: Vec<Area> = outers
            .into_iter()
            .map(|outer| Area {
                outer,
                holes: Vec::new(),
            })
            .collect();
        for hole in holes {
            let inside = first_point(&hole);
            let parent = areas
                .iter_mut()
                .filter(|a| encloses(&a.outer, inside))
                .min_by_key(|a| Ordered(a.outer.area().abs()));
            match parent {
                Some(area) => area.holes.push(hole),
                None => {
                    return Err(geometry(
                        "pocket offset produced an island outside every area".into(),
                    ));
                }
            }
        }
        areas.sort_by_key(Area::sort_key);
        Ok(areas)
    }
}

/// The counter-clockwise loop `outline` shrunk by `d`, through the shape offset: its outer
/// loops. A single loop shrunk has no holes; one that does is an error.
fn shrunk_loop(outline: &Polyline<f64>, d: f64) -> Result<Vec<Polyline<f64>>, ToolpathError> {
    let mut loops = Vec::new();
    for area in Area::of(outline.clone()).shrunk(d)? {
        if !area.holes.is_empty() {
            return Err(geometry(
                "pocket offset made a hole while shrinking a single loop".into(),
            ));
        }
        loops.push(area.outer);
    }
    Ok(loops)
}

/// The loops of a closed loop's offset, all closed. cavalier_contours can return a loop
/// that closes up while offsetting (a bay pinched off as the outline grows) as an open
/// polyline whose ends lie a hair apart; dropping it would lose a hole of the tool's reach.
/// Such a loop is closed; one whose ends lie farther apart than `CHAIN_TOL_MM` is an
/// error, never dropped.
fn closed_loops(loops: Vec<Polyline<f64>>) -> Result<Vec<Polyline<f64>>, ToolpathError> {
    let mut closed = Vec::with_capacity(loops.len());
    for mut loop_ in loops {
        if !loop_.is_closed() {
            let n = loop_.vertex_count();
            let ends = match (loop_.get(0), loop_.get(n.saturating_sub(1))) {
                (Some(a), Some(b)) if n > 2 => ((a.x, a.y), (b.x, b.y)),
                _ => continue,
            };
            if distance(ends.0, ends.1) > CHAIN_TOL_MM {
                return Err(geometry(
                    "pocket offset returned an open loop where a closed one was expected".into(),
                ));
            }
            loop_.set_is_closed(true);
        }
        closed.push(loop_);
    }
    Ok(closed)
}

/// `area` shrunk by `d`; when the offset fails, shrunk farther by each of the retry nudges
/// in turn (docs/specs/offset-retry.md §7). For rest machining, where farther is the safe
/// side: the larger tool is taken to reach a little less, so the rest pocket cuts a little
/// more. An offset can fail at one exact distance and 3 nm beyond it, but not 10 nm beyond.
pub(crate) fn shrunk_farther(area: &Area, d: f64) -> Result<Vec<Area>, ToolpathError> {
    let error = match area.shrunk(d) {
        Ok(areas) => return Ok(areas),
        Err(error) => error,
    };
    for by in NUDGES {
        if let Ok(areas) = area.shrunk(d + by) {
            return Ok(areas);
        }
    }
    Err(error)
}
