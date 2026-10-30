// SPDX-License-Identifier: GPL-3.0-or-later
//! The floor of strokes too wide to V-carve from their centre line
//! (docs/specs/vcarve-wide-strokes.md §4): where an end mill of a given radius reaches, how
//! much of the floor stays standing after it and the V-bit's clean-up, and the largest end
//! mill that leaves nothing.
//!
//! The floor is the region shrunk to where the V-bit's walls meet it. An end mill clears what
//! it can sweep inside: points within its radius of where its centre can go (the floor shrunk
//! by its radius). What it can't reach, the V-bit cleans along the centre line at the floor's
//! depth. Material still standing is measured from the nearest V cut at the floor: the cone
//! leaves it about its distance from the cut's edge ÷ the slope high.

use chipload_core::Point;
use chipload_core::primitives::{Line, Primitive};
use chipload_core::tolerances::{FLOOR_LEFT_MM, FLOOR_LEFT_SHARE};

use crate::ToolpathError;
use crate::cavalier::Area;
use crate::geom::PrimIndex;

/// The most samples a floor is measured at: a grid this fine over its box.
const MAX_SAMPLES: f64 = 40_000.0;
/// The finest sample spacing, mm.
const FINEST_MM: f64 = 0.05;
/// How close the largest end mill is found, as a radius, mm (0.1 mm on the diameter).
const HINT_STEP_MM: f64 = 0.05;

/// One region's floor.
pub(crate) struct Floor {
    areas: Vec<Area>,
    /// The floor's edges: where the V-bit's walls meet it.
    edges: PrimIndex,
    /// Points inside the floor, on a grid, each standing for the same area.
    samples: Vec<Point>,
    /// How much wider the V-bit's cut gets per mm of depth, on each side.
    slope: f64,
    /// The V-bit's flat tip, as a radius.
    tip_radius: f64,
}

/// Where an end mill of radius `radius` reaches in a floor.
pub(crate) struct Reach {
    centres: Vec<Area>,
    edges: Option<PrimIndex>,
    radius: f64,
}

impl Reach {
    /// Whether the end mill clears `p`: within its radius of where its centre can go.
    pub(crate) fn reaches(&self, p: Point) -> bool {
        if self.centres.iter().any(|a| a.contains(p)) {
            return true;
        }
        self.edges
            .as_ref()
            .and_then(|index| index.distance_within(p, self.radius))
            .is_some_and(|d| d <= self.radius)
    }
}

impl Floor {
    /// The floor of `region`: the region shrunk by `inset`. None when there is none.
    pub(crate) fn new(
        region: &Area,
        inset: f64,
        slope: f64,
        tip_radius: f64,
    ) -> Result<Option<Floor>, ToolpathError> {
        let areas = region.shrunk(inset)?;
        let prims: Vec<Primitive> = areas.iter().flat_map(Area::boundary).collect();
        let Some((lo, hi)) = bounds(&prims) else {
            return Ok(None);
        };
        let (w, h) = (hi.0 - lo.0, hi.1 - lo.1);
        let spacing = (w * h / MAX_SAMPLES).sqrt().max(FINEST_MM);
        let mut samples = Vec::new();
        let mut y = lo.1 + spacing / 2.0;
        while y < hi.1 {
            let mut x = lo.0 + spacing / 2.0;
            while x < hi.0 {
                if areas.iter().any(|a| a.contains((x, y))) {
                    samples.push((x, y));
                }
                x += spacing;
            }
            y += spacing;
        }
        if samples.is_empty() {
            return Ok(None);
        }
        let edges = PrimIndex::new(prims, spacing.max(1.0));
        Ok(Some(Floor {
            areas,
            edges,
            samples,
            slope,
            tip_radius,
        }))
    }

    /// Where an end mill of `radius` reaches.
    pub(crate) fn reach(&self, radius: f64) -> Result<Reach, ToolpathError> {
        let mut centres = Vec::new();
        for area in &self.areas {
            centres.extend(area.shrunk(radius)?);
        }
        let prims: Vec<Primitive> = centres.iter().flat_map(Area::boundary).collect();
        let edges = (!prims.is_empty()).then(|| PrimIndex::new(prims, radius.max(1.0)));
        Ok(Reach {
            centres,
            edges,
            radius,
        })
    }

    /// The share of the floor (0 to 1) left standing more than `FLOOR_LEFT_MM` after the
    /// end mill (`reach`) and the V-bit along the walls and along `cleanup`, the centre line
    /// it cuts at the floor's depth.
    pub(crate) fn left_share(&self, reach: &Reach, cleanup: &[(Point, Point)]) -> f64 {
        let lines: Vec<Primitive> = cleanup
            .iter()
            .map(|&(start, end)| Primitive::Line(Line { start, end }))
            .collect();
        let cleanup = (!lines.is_empty()).then(|| PrimIndex::new(lines, 1.0));
        // A point stands this far from a cut's edge before it is too high.
        let limit = FLOOR_LEFT_MM * self.slope;
        let left = self
            .samples
            .iter()
            .filter(|&&p| !reach.reaches(p))
            .filter(|&&p| {
                let near_wall = self.edges.any_within(p, limit);
                let near_cleanup = cleanup
                    .as_ref()
                    .is_some_and(|index| index.any_within(p, limit + self.tip_radius));
                !near_wall && !near_cleanup
            })
            .count();
        left as f64 / self.samples.len() as f64
    }

    /// The largest end mill radius, at most `radius`, that leaves no more than
    /// `FLOOR_LEFT_SHARE` standing, to
    /// `HINT_STEP_MM` (rounded down); `cleanup(reach)` is the V-bit's clean-up for it. A
    /// smaller end mill never leaves more, so halving finds it.
    pub(crate) fn largest_clearing(
        &self,
        radius: f64,
        cleanup: impl Fn(&Reach) -> Vec<(Point, Point)>,
    ) -> Result<f64, ToolpathError> {
        // Whole steps: k steps clear it all, `hi` steps don't.
        let (mut lo, mut hi) = (0_u32, (radius / HINT_STEP_MM).ceil() as u32);
        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            let reach = self.reach(f64::from(mid) * HINT_STEP_MM)?;
            if self.left_share(&reach, &cleanup(&reach)) > FLOOR_LEFT_SHARE {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        Ok(f64::from(lo) * HINT_STEP_MM)
    }
}

/// The box of `prims`' points, if any.
fn bounds(prims: &[Primitive]) -> Option<(Point, Point)> {
    let mut points = prims.iter().flat_map(Primitive::points).peekable();
    points.peek()?;
    Some(points.fold(
        (
            (f64::INFINITY, f64::INFINITY),
            (f64::NEG_INFINITY, f64::NEG_INFINITY),
        ),
        |(lo, hi), p| {
            (
                (lo.0.min(p.0), lo.1.min(p.1)),
                (hi.0.max(p.0), hi.1.max(p.1)),
            )
        },
    ))
}
