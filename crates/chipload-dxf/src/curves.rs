// SPDX-License-Identifier: GPL-3.0-or-later
//! Bulges, splines and ellipses.

use std::f64::consts::{FRAC_PI_2, TAU};

/// A 3D point or vector in drawing units. Z is carried only to report nonzero Z.
pub type Vec3 = [f64; 3];

fn add(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(a: Vec3, f: f64) -> Vec3 {
    [a[0] * f, a[1] * f, a[2] * f]
}

fn dot(a: Vec3, b: Vec3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn norm(a: Vec3) -> f64 {
    dot(a, a).sqrt()
}

/// Ceiling on recursive subdivision; 2^16 pieces per span is far below any useful tolerance.
const MAX_SUBDIVISION_DEPTH: u32 = 16;
/// Every span is cut into this many pieces before adaptive refinement, so S-shaped spans
/// whose midpoint happens to lie on the chord are still refined.
const INITIAL_PIECES: usize = 4;

/// Distance from `p` to the segment a-b.
fn segment_distance(p: Vec3, a: Vec3, b: Vec3) -> f64 {
    let ab = sub(b, a);
    let len2 = dot(ab, ab);
    let t = if len2 == 0.0 {
        0.0
    } else {
        (dot(sub(p, a), ab) / len2).clamp(0.0, 1.0)
    };
    norm(sub(p, add(a, scale(ab, t))))
}

/// Adaptive flattening of a parametric curve over [t0, t1]: refine until every piece's
/// midpoint lies within `tol` of its chord.
fn flatten(curve: &dyn Fn(f64) -> Vec3, breaks: &[f64], tol: f64) -> Vec<Vec3> {
    let mut out = vec![curve(breaks[0])];
    for span in breaks.windows(2) {
        let (t0, t1) = (span[0], span[1]);
        for k in 0..INITIAL_PIECES {
            let a = t0 + (t1 - t0) * k as f64 / INITIAL_PIECES as f64;
            let b = t0 + (t1 - t0) * (k + 1) as f64 / INITIAL_PIECES as f64;
            refine(curve, a, b, curve(a), curve(b), tol, 0, &mut out);
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn refine(
    curve: &dyn Fn(f64) -> Vec3,
    a: f64,
    b: f64,
    pa: Vec3,
    pb: Vec3,
    tol: f64,
    depth: u32,
    out: &mut Vec<Vec3>,
) {
    let m = (a + b) / 2.0;
    let pm = curve(m);
    if depth < MAX_SUBDIVISION_DEPTH && segment_distance(pm, pa, pb) > tol {
        refine(curve, a, m, pa, pm, tol, depth + 1, out);
        refine(curve, m, b, pm, pb, tol, depth + 1, out);
    } else {
        out.push(pb);
    }
}

/// A rational B-spline as stored in a DXF SPLINE.
pub struct Spline<'a> {
    pub degree: usize,
    pub knots: &'a [f64],
    pub control: &'a [Vec3],
    pub weights: &'a [f64],
}

impl Spline<'_> {
    /// Problems that make the data unusable, or None.
    pub fn invalid_reason(&self) -> Option<String> {
        let n = self.control.len();
        if self.degree < 1 {
            return Some("degree must be at least 1".into());
        }
        if n <= self.degree {
            return Some(format!(
                "{n} control points is too few for degree {}",
                self.degree
            ));
        }
        if self.knots.len() != n + self.degree + 1 {
            return Some(format!(
                "{} knots, expected {} for {n} control points of degree {}",
                self.knots.len(),
                n + self.degree + 1,
                self.degree
            ));
        }
        if self.knots.windows(2).any(|w| w[1] < w[0]) {
            return Some("knots are not ascending".into());
        }
        if !self.weights.is_empty() && self.weights.len() != n {
            return Some("weight count differs from control point count".into());
        }
        None
    }

    /// Point at parameter u (de Boor on homogeneous coordinates).
    pub fn point(&self, u: f64) -> Vec3 {
        let p = self.degree;
        let n = self.control.len();
        let (low, high) = (self.knots[p], self.knots[n]);
        let u = u.clamp(low, high);
        // Span k with knots[k] <= u < knots[k+1], clamped so u == high uses the last span.
        let mut k = p;
        while k < n - 1 && self.knots[k + 1] <= u {
            k += 1;
        }
        let weight = |i: usize| self.weights.get(i).copied().unwrap_or(1.0);
        let mut d: Vec<[f64; 4]> = (0..=p)
            .map(|j| {
                let i = j + k - p;
                let w = weight(i);
                let c = self.control[i];
                [c[0] * w, c[1] * w, c[2] * w, w]
            })
            .collect();
        for r in 1..=p {
            for j in (r..=p).rev() {
                let i = j + k - p;
                let denom = self.knots[i + p + 1 - r] - self.knots[i];
                let alpha = if denom == 0.0 {
                    0.0
                } else {
                    (u - self.knots[i]) / denom
                };
                let previous = d[j - 1];
                for (value, prev) in d[j].iter_mut().zip(previous) {
                    *value = (1.0 - alpha) * prev + alpha * *value;
                }
            }
        }
        let [x, y, z, w] = d[p];
        if w == 0.0 {
            [x, y, z]
        } else {
            [x / w, y / w, z / w]
        }
    }

    pub fn flatten(&self, tol: f64) -> Vec<Vec3> {
        let p = self.degree;
        let n = self.control.len();
        let mut breaks: Vec<f64> = self.knots[p..=n].to_vec();
        breaks.dedup();
        if breaks.len() < 2 {
            return vec![self.point(self.knots[p])];
        }
        flatten(&|u| self.point(u), &breaks, tol)
    }
}

/// An elliptical arc `center + major·cos t + minor·sin t` for t from `start` to `end`
/// (radians; end <= start means it wraps once).
pub struct Ellipse {
    pub center: Vec3,
    pub major: Vec3,
    pub minor: Vec3,
    pub start: f64,
    pub end: f64,
}

impl Ellipse {
    pub fn flatten(&self, tol: f64) -> Vec<Vec3> {
        let mut end = self.end;
        while end <= self.start {
            end += TAU;
        }
        let curve = |t: f64| {
            add(
                self.center,
                add(scale(self.major, t.cos()), scale(self.minor, t.sin())),
            )
        };
        // Break at quarter turns so every piece is convex and well behaved.
        let mut breaks = vec![self.start];
        let mut t = (self.start / FRAC_PI_2).floor() * FRAC_PI_2 + FRAC_PI_2;
        while t < end {
            breaks.push(t);
            t += FRAC_PI_2;
        }
        breaks.push(end);
        flatten(&curve, &breaks, tol)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spline_endpoints_and_tolerance() {
        // Clamped cubic through (0,0) .. (30,10).
        let control = [
            [0.0, 0.0, 0.0],
            [10.0, 10.0, 0.0],
            [20.0, 0.0, 0.0],
            [30.0, 10.0, 0.0],
        ];
        let knots = [0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0];
        let spline = Spline {
            degree: 3,
            knots: &knots,
            control: &control,
            weights: &[],
        };
        assert!(spline.invalid_reason().is_none());
        let pts = spline.flatten(0.01);
        assert!(norm(sub(pts[0], control[0])) < 1e-9);
        assert!(norm(sub(*pts.last().unwrap(), control[3])) < 1e-9);
        // Every chord midpoint lies within the tolerance of the curve.
        for w in pts.windows(2) {
            let mid = scale(add(w[0], w[1]), 0.5);
            let nearest = (0..=10_000)
                .map(|i| norm(sub(spline.point(f64::from(i) / 10_000.0), mid)))
                .fold(f64::INFINITY, f64::min);
            assert!(
                nearest < 0.01 + 1e-3,
                "chord midpoint {nearest} from the curve"
            );
        }
    }

    #[test]
    fn full_ellipse_closes() {
        let e = Ellipse {
            center: [0.0; 3],
            major: [10.0, 0.0, 0.0],
            minor: [0.0, 5.0, 0.0],
            start: 0.0,
            end: TAU,
        };
        let pts = e.flatten(0.01);
        assert!(norm(sub(pts[0], *pts.last().unwrap())) < 1e-9);
        assert!(pts.iter().any(|p| (p[1] - 5.0).abs() < 0.02));
    }
}
