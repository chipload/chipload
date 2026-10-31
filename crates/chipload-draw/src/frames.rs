// SPDX-License-Identifier: GPL-3.0-or-later
//! Frames for signs (docs/specs/shape-library.md §3): a closed outline from a width, a
//! height and one corner size *r*, and the size a frame needs round a box. Plain geometry
//! worked out here: lines and arcs about the corners and sides; the oval is an ellipse
//! fitted with arcs (`chipload_core::fit`).

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use chipload_core::Point;
use chipload_core::fit::fit_path;
use chipload_core::primitives::{Arc, Line, Primitive};
use chipload_core::tolerances::{ARC_APPROX_TOL_MM, CHAIN_TOL_MM};

use crate::offset::{Chain, Side, nearest_distance, side_at};

/// The built-in frames (§3.1, decision 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FrameKind {
    /// Sharp corners.
    Rectangle,
    /// Corners rounded by *r*.
    Rounded,
    /// Each corner cut in by a quarter circle of radius *r* about it.
    Notched,
    /// Each corner cut off at 45° by *r* along both sides.
    Chamfered,
    /// An ellipse touching the box's four sides.
    Oval,
    /// The top side an arc rising *r* above the top corners.
    Arched,
    /// The short sides bulging outwards as arcs of height *r*.
    Plaque,
}

impl FrameKind {
    pub const ALL: [FrameKind; 7] = [
        FrameKind::Rectangle,
        FrameKind::Rounded,
        FrameKind::Notched,
        FrameKind::Chamfered,
        FrameKind::Oval,
        FrameKind::Arched,
        FrameKind::Plaque,
    ];

    /// Its name on the command line and in messages.
    pub fn name(self) -> &'static str {
        match self {
            FrameKind::Rectangle => "rectangle",
            FrameKind::Rounded => "rounded",
            FrameKind::Notched => "notched",
            FrameKind::Chamfered => "chamfered",
            FrameKind::Oval => "oval",
            FrameKind::Arched => "arched",
            FrameKind::Plaque => "plaque",
        }
    }

    pub fn from_name(name: &str) -> Option<FrameKind> {
        FrameKind::ALL
            .into_iter()
            .find(|k| k.name().eq_ignore_ascii_case(name.trim()))
    }

    /// Whether *r* means anything for it.
    pub fn uses_r(self) -> bool {
        !matches!(self, FrameKind::Rectangle | FrameKind::Oval)
    }

    /// The largest *r* that keeps the outline a simple loop, for a `width` × `height` frame.
    pub fn largest_r(self, width: f64, height: f64) -> f64 {
        let shorter = width.min(height);
        match self {
            FrameKind::Rectangle | FrameKind::Oval => 0.0,
            FrameKind::Rounded | FrameKind::Notched | FrameKind::Chamfered => shorter / 2.0,
            FrameKind::Arched => (width / 2.0).min(height),
            FrameKind::Plaque => shorter / 2.0,
        }
    }

    /// *r* when none is given: a tenth of the shorter side.
    pub fn default_r(self, width: f64, height: f64) -> f64 {
        if self.uses_r() {
            width.min(height) / 10.0
        } else {
            0.0
        }
    }
}

/// Why a frame can't be drawn.
#[derive(Debug, Clone, PartialEq)]
pub enum FrameError {
    /// Width or height isn't a positive number.
    BadSize,
    /// *r* is negative, or larger than `largest`.
    BadR { largest: f64 },
    /// No size makes the box fit (it doesn't happen for the built-in frames).
    NoFit,
}

/// The outline of a `kind` frame `width` × `height` centred on `center`, counter-clockwise.
pub fn outline(
    kind: FrameKind,
    center: Point,
    width: f64,
    height: f64,
    r: f64,
) -> Result<Chain, FrameError> {
    if !(width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0) {
        return Err(FrameError::BadSize);
    }
    let largest = kind.largest_r(width, height);
    let r = if kind.uses_r() { r } else { 0.0 };
    if !r.is_finite() || r < 0.0 || r > largest + CHAIN_TOL_MM {
        return Err(FrameError::BadR { largest });
    }
    let r = r.min(largest);
    let (a, b) = (width / 2.0, height / 2.0);
    let mut path = Path::default();
    match kind {
        FrameKind::Rectangle => rounded(&mut path, a, b, 0.0),
        FrameKind::Rounded => rounded(&mut path, a, b, r),
        FrameKind::Notched => notched(&mut path, a, b, r),
        FrameKind::Chamfered => chamfered(&mut path, a, b, r),
        FrameKind::Oval => oval(&mut path, a, b),
        FrameKind::Arched => arched(&mut path, a, b, r),
        FrameKind::Plaque if width >= height => plaque(&mut path, a, b, r),
        FrameKind::Plaque => {
            // Tall: drawn wide and turned a quarter turn (still counter-clockwise).
            plaque(&mut path, b, a, r);
            path.turn_quarter();
        }
    }
    Ok(Chain {
        prims: path.prims.iter().map(|p| p.translated(center)).collect(),
        closed: true,
    })
}

/// The `width` × `height` a `kind` frame needs so that the box `inner` (width, height)
/// lies inside it at least `inset` from its outline: inside its innermost border line
/// when `inset` is the borders' depth (§3.3). Rectangle-like frames grow equally in both
/// directions, the oval in proportion, the arched one in height, the plaque along its
/// long side; each just enough.
pub fn fit_round(
    kind: FrameKind,
    inner: (f64, f64),
    inset: f64,
    r: f64,
) -> Result<(f64, f64), FrameError> {
    if !(inner.0.is_finite() && inner.1.is_finite() && inner.0 > 0.0 && inner.1 > 0.0) {
        return Err(FrameError::BadSize);
    }
    let start = (inner.0 + 2.0 * inset, inner.1 + 2.0 * inset);
    let grow = match kind {
        FrameKind::Rectangle | FrameKind::Rounded | FrameKind::Notched | FrameKind::Chamfered => {
            (1.0, 1.0)
        }
        FrameKind::Oval => start,
        FrameKind::Arched => (0.0, 1.0),
        FrameKind::Plaque if start.0 >= start.1 => (1.0, 0.0),
        FrameKind::Plaque => (0.0, 1.0),
    };
    let size = |t: f64| (start.0 + t * grow.0, start.1 + t * grow.1);
    let fits = |t: f64| {
        let (w, h) = size(t);
        outline(kind, (0.0, 0.0), w, h, r).is_ok_and(|chain| holds(&chain, inner, inset))
    };
    if fits(0.0) {
        return Ok(start);
    }
    let rate = grow.0.max(grow.1);
    let mut hi = 0.05 * start.0.max(start.1) / rate;
    let mut doublings = 0;
    while !fits(hi) {
        hi *= 2.0;
        doublings += 1;
        if doublings > 60 {
            return Err(FrameError::NoFit);
        }
    }
    let mut lo = 0.0;
    while (hi - lo) * rate > CHAIN_TOL_MM {
        let mid = (lo + hi) / 2.0;
        if fits(mid) {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    Ok(size(hi))
}

/// Whether the box `inner` centred on the origin lies inside `chain` at least `inset` from
/// it. Its corners are enough: every frame is convex but the notched one, whose notches
/// come nearest a centred box at its corners.
pub fn holds(chain: &Chain, inner: (f64, f64), inset: f64) -> bool {
    let (x, y) = (inner.0 / 2.0, inner.1 / 2.0);
    [(-x, -y), (x, -y), (x, y), (-x, y)].into_iter().all(|p| {
        let d = nearest_distance(chain, p);
        let inside = side_at(chain, p) == Side::Inside || d <= CHAIN_TOL_MM;
        inside && d >= inset - CHAIN_TOL_MM
    })
}

/// An outline built segment by segment from a start point.
#[derive(Default)]
struct Path {
    prims: Vec<Primitive>,
    at: Point,
}

impl Path {
    fn start(&mut self, p: Point) {
        self.at = p;
    }

    /// A line to `p`; none when it would have no length.
    fn line_to(&mut self, p: Point) {
        if (p.0 - self.at.0).hypot(p.1 - self.at.1) > CHAIN_TOL_MM {
            self.prims.push(Primitive::Line(Line {
                start: self.at,
                end: p,
            }));
        }
        self.at = p;
    }

    /// An arc about `center` from where the path is, turning `sweep` (radians, positive
    /// counter-clockwise); none when it has no radius or no turn.
    fn arc(&mut self, center: Point, sweep: f64) {
        let radius = (self.at.0 - center.0).hypot(self.at.1 - center.1);
        if radius <= CHAIN_TOL_MM || sweep == 0.0 {
            return;
        }
        let arc = Arc {
            center,
            radius,
            start_angle: (self.at.1 - center.1).atan2(self.at.0 - center.0),
            sweep,
        };
        self.prims.push(Primitive::Arc(arc));
        self.at = arc.point_at(arc.start_angle + sweep);
    }

    /// Turned a quarter turn counter-clockwise about the origin.
    fn turn_quarter(&mut self) {
        let turn = |p: Point| (-p.1, p.0);
        for prim in &mut self.prims {
            *prim = match *prim {
                Primitive::Line(l) => Primitive::Line(Line {
                    start: turn(l.start),
                    end: turn(l.end),
                }),
                Primitive::Arc(a) => Primitive::Arc(Arc {
                    center: turn(a.center),
                    start_angle: (a.start_angle + FRAC_PI_2).rem_euclid(TAU),
                    ..a
                }),
            };
        }
    }
}

/// Half-sizes `a`, `b`; corners rounded by `r` (0: sharp). From the bottom side's left end.
fn rounded(path: &mut Path, a: f64, b: f64, r: f64) {
    path.start((-a + r, -b));
    path.line_to((a - r, -b));
    path.arc((a - r, -b + r), FRAC_PI_2);
    path.line_to((a, b - r));
    path.arc((a - r, b - r), FRAC_PI_2);
    path.line_to((-a + r, b));
    path.arc((-a + r, b - r), FRAC_PI_2);
    path.line_to((-a, -b + r));
    path.arc((-a + r, -b + r), FRAC_PI_2);
}

/// Each corner cut in by a quarter circle of radius `r` about it, turning clockwise.
fn notched(path: &mut Path, a: f64, b: f64, r: f64) {
    path.start((-a + r, -b));
    path.line_to((a - r, -b));
    path.arc((a, -b), -FRAC_PI_2);
    path.line_to((a, b - r));
    path.arc((a, b), -FRAC_PI_2);
    path.line_to((-a + r, b));
    path.arc((-a, b), -FRAC_PI_2);
    path.line_to((-a, -b + r));
    path.arc((-a, -b), -FRAC_PI_2);
}

fn chamfered(path: &mut Path, a: f64, b: f64, r: f64) {
    path.start((-a + r, -b));
    for p in [
        (a - r, -b),
        (a, -b + r),
        (a, b - r),
        (a - r, b),
        (-a + r, b),
        (-a, b - r),
        (-a, -b + r),
        (-a + r, -b),
    ] {
        path.line_to(p);
    }
}

/// The ellipse with half-axes `a`, `b`, as arcs within the arc tolerance: points close
/// enough that their chords stay within a tenth of it (closer where it curves tighter),
/// fitted within the rest.
fn oval(path: &mut Path, a: f64, b: f64) {
    let tol = ARC_APPROX_TOL_MM / 10.0;
    let mut points = Vec::new();
    let mut t = 0.0;
    while t < TAU {
        let (sin, cos) = t.sin_cos();
        points.push((a * cos, b * sin));
        // How fast the point moves with t, and the radius of the curve there: a chord of
        // length L sags L² / 8ρ.
        let speed = (a * a * sin * sin + b * b * cos * cos).sqrt();
        let curve = speed.powi(3) / (a * b);
        let step = (8.0 * curve * tol).sqrt();
        t += (step / speed).min(TAU / 16.0);
    }
    path.prims = fit_path(&points, true, ARC_APPROX_TOL_MM - tol, true);
}

/// The top side an arc of height `r` above the top corners; with `r` the whole height the
/// sides are gone.
fn arched(path: &mut Path, a: f64, b: f64, r: f64) {
    path.start((-a, -b));
    path.line_to((a, -b));
    path.line_to((a, b - r));
    if r > CHAIN_TOL_MM {
        // The circle through both top corners and the top middle.
        let radius = (a * a + r * r) / (2.0 * r);
        let center = (0.0, b - radius);
        let start = (b - r - center.1).atan2(a);
        path.arc(center, PI - 2.0 * start);
    } else {
        path.line_to((-a, b));
    }
    path.line_to((-a, -b));
}

/// Wide (`a` ≥ `b`): the left and right sides bulging out by `r`.
fn plaque(path: &mut Path, a: f64, b: f64, r: f64) {
    path.start((-a + r, -b));
    path.line_to((a - r, -b));
    if r > CHAIN_TOL_MM {
        let radius = (b * b + r * r) / (2.0 * r);
        let sweep = 2.0 * b.atan2(radius - r);
        path.arc((a - radius, 0.0), sweep);
        path.line_to((-a + r, b));
        path.arc((-a + radius, 0.0), sweep);
    } else {
        path.line_to((a, b));
        path.line_to((-a, b));
        path.line_to((-a, -b));
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use chipload_core::primitives::distance;
    use proptest::prelude::*;

    fn bounds(chain: &Chain) -> (Point, Point) {
        chain
            .prims
            .iter()
            .map(Primitive::bounds)
            .reduce(|(lo, hi), (a, b)| {
                (
                    (lo.0.min(a.0), lo.1.min(a.1)),
                    (hi.0.max(b.0), hi.1.max(b.1)),
                )
            })
            .unwrap()
    }

    /// Closed: each piece starts where the one before ends.
    fn assert_closed(chain: &Chain) {
        let n = chain.prims.len();
        assert!(n >= 2, "{chain:?}");
        for i in 0..n {
            let gap = distance(chain.prims[i].end(), chain.prims[(i + 1) % n].start());
            assert!(gap < CHAIN_TOL_MM, "gap {gap} after piece {i}");
        }
    }

    /// No two pieces cross; they meet only at their ends (neighbours, or two notches at
    /// their largest meeting on a side).
    fn assert_simple(chain: &Chain) {
        let n = chain.prims.len();
        for i in 0..n {
            for j in i + 1..n {
                let hits = crate::intersect::intersections(&chain.prims[i], &chain.prims[j]);
                let at_ends = hits.iter().all(|&p| {
                    [chain.prims[i].start(), chain.prims[i].end()]
                        .iter()
                        .any(|&q| distance(p, q) < 10.0 * CHAIN_TOL_MM)
                });
                assert!(at_ends, "pieces {i} and {j} cross at {hits:?}");
            }
        }
    }

    fn assert_box(chain: &Chain, w: f64, h: f64) {
        let ((x0, y0), (x1, y1)) = bounds(chain);
        // Each side within the arc tolerance.
        let tol = 2.0 * ARC_APPROX_TOL_MM;
        assert!((x1 - x0 - w).abs() < tol, "width {} of {w}", x1 - x0);
        assert!((y1 - y0 - h).abs() < tol, "height {} of {h}", y1 - y0);
        assert!((x0 + x1).abs() < tol && (y0 + y1).abs() < tol);
    }

    fn area(chain: &Chain) -> f64 {
        let ring: Vec<Point> = chain.prims.iter().flat_map(Primitive::points).collect();
        chipload_core::primitives::signed_area(&ring)
    }

    #[test]
    fn every_frame_is_closed_simple_counter_clockwise_and_the_size_asked() {
        for kind in FrameKind::ALL {
            for (w, h) in [(300.0, 150.0), (100.0, 240.0), (80.0, 80.0)] {
                let largest = kind.largest_r(w, h);
                for r in [0.0, kind.default_r(w, h), largest] {
                    let chain = outline(kind, (0.0, 0.0), w, h, r).unwrap();
                    assert_closed(&chain);
                    assert_simple(&chain);
                    assert!(area(&chain) > 0.0, "{kind:?} turns clockwise");
                    assert_box(&chain, w, h);
                }
            }
        }
    }

    #[test]
    fn r_past_the_largest_is_refused_with_the_largest() {
        assert_eq!(
            outline(FrameKind::Rounded, (0.0, 0.0), 200.0, 100.0, 60.0),
            Err(FrameError::BadR { largest: 50.0 })
        );
        assert_eq!(
            outline(FrameKind::Arched, (0.0, 0.0), 200.0, 60.0, 80.0),
            Err(FrameError::BadR { largest: 60.0 })
        );
        assert!(outline(FrameKind::Rounded, (0.0, 0.0), 200.0, 100.0, -1.0).is_err());
        assert_eq!(
            outline(FrameKind::Oval, (0.0, 0.0), 0.0, 100.0, 0.0),
            Err(FrameError::BadSize)
        );
        // r means nothing for an oval: any is taken.
        assert!(outline(FrameKind::Oval, (0.0, 0.0), 200.0, 100.0, 500.0).is_ok());
    }

    /// How far `p` is from the ellipse with half-axes `a`, `b`: the nearest of many of its
    /// points near the ray to `p` (a thin ellipse's other side is near too), narrowed down
    /// by golden sections.
    fn ellipse_distance(a: f64, b: f64, p: Point) -> f64 {
        let at = |t: f64| distance(p, (a * t.cos(), b * t.sin()));
        let guess = (p.1 / b).atan2(p.0 / a);
        let step = 0.6 / 4000.0;
        let best = (0..=4000)
            .map(|i| guess - 0.3 + step * f64::from(i))
            .min_by(|&s, &t| at(s).total_cmp(&at(t)))
            .unwrap();
        let (mut lo, mut hi) = (best - step, best + step);
        let golden = (5f64.sqrt() - 1.0) / 2.0;
        for _ in 0..100 {
            let (m1, m2) = (hi - golden * (hi - lo), lo + golden * (hi - lo));
            if at(m1) < at(m2) {
                hi = m2;
            } else {
                lo = m1;
            }
        }
        at((lo + hi) / 2.0)
    }

    #[test]
    fn the_oval_stays_within_the_arc_tolerance_of_the_ellipse() {
        for (a, b) in [(150.0, 60.0), (200.0, 3.0), (40.0, 40.0)] {
            let chain = outline(FrameKind::Oval, (0.0, 0.0), 2.0 * a, 2.0 * b, 0.0).unwrap();
            assert!(chain.prims.len() < 100, "{} pieces", chain.prims.len());
            assert!(chain.prims.iter().any(|p| matches!(p, Primitive::Arc(_))));
            for prim in &chain.prims {
                for p in prim.points() {
                    let off = ellipse_distance(a, b, p);
                    assert!(off < ARC_APPROX_TOL_MM, "{p:?} is {off} off");
                }
            }
        }
    }

    #[test]
    fn a_frame_centred_elsewhere_is_moved() {
        let chain = outline(FrameKind::Notched, (100.0, 50.0), 40.0, 20.0, 5.0).unwrap();
        let ((x0, y0), (x1, y1)) = bounds(&chain);
        assert!((x0 - 80.0).abs() < 1e-9 && (x1 - 120.0).abs() < 1e-9);
        assert!((y0 - 40.0).abs() < 1e-9 && (y1 - 60.0).abs() < 1e-9);
    }

    #[test]
    fn a_rectangle_round_a_box_is_the_box_and_the_inset() {
        let size = fit_round(FrameKind::Rectangle, (100.0, 40.0), 5.0, 0.0).unwrap();
        assert_eq!(size, (110.0, 50.0));
        // Rounded corners need more room only when they cut into the box.
        let size = fit_round(FrameKind::Rounded, (100.0, 40.0), 5.0, 5.0).unwrap();
        assert_eq!(size, (110.0, 50.0));
        let size = fit_round(FrameKind::Rounded, (100.0, 40.0), 0.0, 10.0).unwrap();
        assert!(size.0 > 100.0, "{size:?}");
        assert!(((size.0 - 100.0) - (size.1 - 40.0)).abs() < 1e-9);
    }

    #[test]
    fn grown_frames_grow_the_way_their_shape_needs() {
        let inner = (120.0, 40.0);
        let oval = fit_round(FrameKind::Oval, inner, 0.0, 0.0).unwrap();
        // The ellipse of the box's proportions through its corners: √2 times the box.
        assert!((oval.0 / inner.0 - 2f64.sqrt()).abs() < 1e-3, "{oval:?}");
        assert!((oval.0 / oval.1 - 3.0).abs() < 1e-9);
        let arched = fit_round(FrameKind::Arched, inner, 0.0, 20.0).unwrap();
        assert_eq!(arched.0, 120.0);
        // The box's top corners on the sides, just below the arch.
        assert!((arched.1 - 80.0).abs() < 0.01, "{arched:?}");
        let plaque = fit_round(FrameKind::Plaque, inner, 0.0, 10.0).unwrap();
        assert_eq!(plaque.1, 40.0);
        assert!((plaque.0 - 140.0).abs() < 0.01, "{plaque:?}");
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]
        /// Round any box, for every frame: the box lies inside at least the inset from the
        /// outline, and a frame a little smaller doesn't hold it (just enough).
        #[test]
        fn the_box_fits_inside_and_only_just(
            w in 5.0f64..400.0,
            h in 5.0f64..400.0,
            inset in 0.0f64..20.0,
            share in 0.0f64..1.0,
        ) {
            for kind in FrameKind::ALL {
                let r = share * (w.min(h) / 2.0);
                let (fw, fh) = fit_round(kind, (w, h), inset, r).unwrap();
                let chain = outline(kind, (0.0, 0.0), fw, fh, r).unwrap();
                prop_assert!(holds(&chain, (w, h), inset), "{:?}", kind);
                if (fw - w - 2.0 * inset) > 0.02 || (fh - h - 2.0 * inset) > 0.02 {
                    let less = (fw * 0.999, fh * 0.999);
                    let fits_less = outline(kind, (0.0, 0.0), less.0, less.1, r)
                        .is_ok_and(|c| holds(&c, (w, h), inset));
                    prop_assert!(!fits_less, "{:?}: a smaller one fits", kind);
                }
            }
        }
    }
}
