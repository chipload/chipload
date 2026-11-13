// SPDX-License-Identifier: GPL-3.0-or-later
//! A black-and-white picture's shapes inflated (docs/specs/inflated-shapes.md §4): each
//! shape cell's height from its distance to the shape's edge, by a profile, up to a reach.
//!
//! The distance is the exact Euclidean distance transform of the grid by the lower envelope
//! of parabolas: P. F. Felzenszwalb and D. P. Huttenlocher, "Distance Transforms of Sampled
//! Functions", Theory of Computing 8 (2012), one pass along the rows, one along the
//! columns.

use chipload_i18n::{Digits, Message, Value};

use crate::depth::{averaged, smoothed};
use crate::{Decoded, DepthGrid, ImageError, SMOOTHING_RANGE, Threshold, otsu};

/// How a shape rises from its edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    /// A quarter circle: steep at the edge, flat on top.
    Round,
    /// A straight rise.
    Straight,
}

/// How a picture's shapes are inflated (spec §3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InflateSettings {
    /// Where grey turns black.
    pub threshold: Threshold,
    /// White is the shape; false: black is.
    pub invert: bool,
    pub profile: Profile,
    /// How far in from the edge the height grows, mm; None: the widest shape's half-width.
    pub reach: Option<f64>,
    /// The blur over the heights, its radius on the board, mm.
    pub smoothing: f64,
}

/// A value that stands for "no outside cell yet": far beyond any squared distance on a
/// grid of [`crate::MAX_CELLS`] cells, yet finite, so the envelope's arithmetic stays exact.
const FAR: f64 = 1e18;

/// The squared distance from each cell to the nearest cell where `outside` is true, in cells,
/// on a `columns` × `rows` grid (row-major), and to the cells just beyond the grid's border,
/// which count as outside.
pub fn squared_distances(outside: &[bool], columns: usize, rows: usize) -> Vec<f64> {
    // The grid with a ring of outside cells round it.
    let (w, h) = (columns + 2, rows + 2);
    let mut f = vec![0.0; w * h];
    for r in 0..rows {
        for c in 0..columns {
            if !outside[r * columns + c] {
                f[(r + 1) * w + c + 1] = FAR;
            }
        }
    }
    // Along the rows, then along the columns of that.
    let mut line = Vec::with_capacity(w.max(h));
    for r in 0..h {
        line.clear();
        line.extend_from_slice(&f[r * w..(r + 1) * w]);
        let d = envelope(&line);
        f[r * w..(r + 1) * w].copy_from_slice(&d);
    }
    for c in 0..w {
        line.clear();
        line.extend((0..h).map(|r| f[r * w + c]));
        for (r, v) in envelope(&line).into_iter().enumerate() {
            f[r * w + c] = v;
        }
    }
    let mut out = Vec::with_capacity(columns * rows);
    for r in 0..rows {
        out.extend_from_slice(&f[(r + 1) * w + 1..(r + 1) * w + 1 + columns]);
    }
    out
}

/// Where the parabola rooted at `q` comes below the one rooted at `p` (p < q).
fn meet(f: &[f64], p: usize, q: usize) -> f64 {
    let square = |i: usize| (i * i) as f64;
    ((f[q] + square(q)) - (f[p] + square(p))) / (2.0 * (q - p) as f64)
}

/// The one-dimensional squared distance transform of `f`: for each q, the least of
/// (q − p)² + f(p) over all p, by the lower envelope of the parabolas rooted at each p.
fn envelope(f: &[f64]) -> Vec<f64> {
    let n = f.len();
    if n == 0 {
        return Vec::new();
    }
    // The parabolas in the envelope, and where each starts being lowest.
    let mut roots: Vec<usize> = vec![0];
    let mut starts: Vec<f64> = vec![f64::NEG_INFINITY, f64::INFINITY];
    for q in 1..n {
        // Drop the parabolas the new one is lower than from where they start; the first
        // starts at −∞, so it is never dropped and `roots` is never empty.
        let mut k = roots.len() - 1;
        let mut s = meet(f, roots[k], q);
        while k > 0 && s <= starts[k] {
            roots.pop();
            starts.pop();
            k -= 1;
            s = meet(f, roots[k], q);
        }
        roots.push(q);
        starts[k + 1] = s;
        starts.push(f64::INFINITY);
    }
    let mut out = vec![0.0; n];
    let mut k = 0;
    for (q, slot) in out.iter_mut().enumerate() {
        while starts[k + 1] < q as f64 {
            k += 1;
        }
        let p = roots[k];
        let d = q as f64 - p as f64;
        *slot = d * d + f[p];
    }
    out
}

/// `picture`'s shapes inflated (spec §4), `width` mm wide: on the depth map's grid, black
/// and white by the threshold, each shape cell's distance to the edge (to the nearest
/// outside cell's centre, less half a cell), its height by the profile up to the reach,
/// then smoothed; outside cells left out. Refused with no shape at all.
pub fn inflate_grid(
    picture: &Decoded,
    settings: &InflateSettings,
    width: f64,
) -> Result<DepthGrid, ImageError> {
    let (lo, hi) = SMOOTHING_RANGE;
    if !(settings.smoothing.is_finite() && (lo..=hi).contains(&settings.smoothing)) {
        return Err(ImageError::Setting(
            Message::new("image-error-depth-setting")
                .arg("field", "smoothing".to_string())
                .arg("low", Value::Number(lo, Digits::Shortest))
                .arg("high", Value::Number(hi, Digits::Shortest))
                .arg("got", Value::Number(settings.smoothing, Digits::Shortest)),
        ));
    }
    if settings.reach.is_some_and(|r| !(r.is_finite() && r > 0.0)) {
        return Err(ImageError::Setting(
            Message::new("image-error-inflate-reach").arg(
                "got",
                Value::Length(settings.reach.unwrap_or(0.0), Digits::Shortest),
            ),
        ));
    }
    if !(width.is_finite() && width > 0.0) {
        return Err(ImageError::Width);
    }
    let (columns, rows, cell, greys) = averaged(picture, width);
    let level = match settings.threshold {
        Threshold::Auto => otsu(&greys),
        Threshold::Level(level) => level,
    };
    let outside: Vec<bool> = greys
        .iter()
        .map(|&g| (g < f64::from(level)) == settings.invert)
        .collect();
    if outside.iter().all(|&o| o) {
        return Err(ImageError::Setting(Message::new(
            "image-error-inflate-empty",
        )));
    }
    let distances: Vec<f64> = squared_distances(&outside, columns, rows)
        .into_iter()
        .zip(&outside)
        .map(|(d2, &out)| if out { 0.0 } else { (d2.sqrt() - 0.5) * cell })
        .collect();
    let reach = settings
        .reach
        .unwrap_or_else(|| distances.iter().copied().fold(0.0, f64::max))
        .max(f64::MIN_POSITIVE);
    let heights: Vec<f64> = distances
        .iter()
        .map(|&d| {
            let t = (d.min(reach) / reach).clamp(0.0, 1.0);
            match settings.profile {
                Profile::Round => (1.0 - (1.0 - t) * (1.0 - t)).sqrt(),
                Profile::Straight => t,
            }
        })
        .collect();
    let heights = smoothed(heights, (columns, rows), cell, settings.smoothing);
    Ok(DepthGrid {
        columns,
        rows,
        cell,
        heights: heights
            .into_iter()
            .zip(&outside)
            .map(|(h, &out)| (!out).then_some(h))
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const ROUND: InflateSettings = InflateSettings {
        threshold: Threshold::Level(128),
        invert: false,
        profile: Profile::Round,
        reach: None,
        smoothing: 0.0,
    };

    /// A picture `n` × `n`: black where `f(x, y)` (pixel centres, y up from the bottom).
    fn picture(n: u32, f: impl Fn(f64, f64) -> bool) -> Decoded {
        let grey = (0..n)
            .flat_map(|row| (0..n).map(move |x| (x, row)))
            .map(|(x, row)| {
                let y = f64::from(n - 1 - row) + 0.5;
                if f(f64::from(x) + 0.5, y) { 0.0 } else { 255.0 }
            })
            .collect();
        Decoded {
            width: n,
            height: n,
            grey,
        }
    }

    fn brute(outside: &[bool], columns: usize, rows: usize) -> Vec<f64> {
        let mut out = Vec::new();
        for r in 0..rows as isize {
            for c in 0..columns as isize {
                let mut best = f64::INFINITY;
                // Every outside cell, and the ring beyond the border.
                for rr in -1..=rows as isize {
                    for cc in -1..=columns as isize {
                        let beyond =
                            rr < 0 || cc < 0 || rr >= rows as isize || cc >= columns as isize;
                        let out = beyond || outside[rr as usize * columns + cc as usize];
                        if out {
                            let d = ((r - rr) * (r - rr) + (c - cc) * (c - cc)) as f64;
                            best = best.min(d);
                        }
                    }
                }
                out.push(best);
            }
        }
        out
    }

    proptest! {
        #[test]
        fn the_transform_is_exact(
            columns in 1usize..9,
            rows in 1usize..9,
            bits in proptest::collection::vec(any::<bool>(), 64),
        ) {
            let outside: Vec<bool> = bits[..columns * rows].to_vec();
            prop_assert_eq!(squared_distances(&outside, columns, rows), brute(&outside, columns, rows));
        }
    }

    #[test]
    fn a_disc_rises_as_a_dome_or_a_cone() {
        let r = 40.0;
        let disc = picture(100, |x, y| (x - 50.0).hypot(y - 50.0) < r);
        let grid = inflate_grid(&disc, &ROUND, 100.0).unwrap();
        // At a distance s from the centre: d = r − s from the edge, height √(1 − (s/r)²).
        for s in [0.5, 10.5, 20.5, 30.5] {
            let h = grid.at(50 + s as usize, 50).unwrap();
            let want = (1.0 - (s / r) * (s / r)).sqrt();
            assert!((h - want).abs() < 0.04, "{s}: {h} {want}");
        }
        assert_eq!(grid.at(0, 0), None);
        let cone = InflateSettings {
            profile: Profile::Straight,
            ..ROUND
        };
        let grid = inflate_grid(&disc, &cone, 100.0).unwrap();
        let h = grid.at(70, 50).unwrap();
        assert!((h - 0.5).abs() < 0.03, "{h}");
    }

    #[test]
    fn a_set_reach_levels_wide_shapes_off() {
        // Discs of radius 10 and 20.
        let two = picture(100, |x, y| {
            (x - 20.0).hypot(y - 50.0) < 10.0 || (x - 70.0).hypot(y - 50.0) < 20.0
        });
        // Automatic: the reach is the large one's 20; the small one stops lower.
        let auto = inflate_grid(&two, &ROUND, 100.0).unwrap();
        let small = auto.at(20, 50).unwrap();
        assert!((small - 0.75f64.sqrt()).abs() < 0.05, "{small}");
        assert!(auto.at(70, 50).unwrap() > 0.99);
        // Reach 10 mm: both reach the top; the large one is flat within 10 mm of its middle.
        let set = InflateSettings {
            reach: Some(10.0),
            ..ROUND
        };
        let grid = inflate_grid(&two, &set, 100.0).unwrap();
        assert!(grid.at(20, 50).unwrap() > 0.99);
        assert_eq!(grid.at(70, 50), Some(1.0));
        assert_eq!(grid.at(75, 50), Some(1.0));
    }

    #[test]
    fn a_ring_rises_from_both_its_sides() {
        let ring = picture(100, |x, y| {
            let s = (x - 50.0).hypot(y - 50.0);
            (20.0..40.0).contains(&s)
        });
        let grid = inflate_grid(&ring, &ROUND, 100.0).unwrap();
        assert_eq!(grid.at(50, 50), None);
        let (inner, middle, outer) = (
            // 4.5 mm in from the inner edge (20 mm out), the middle, 4.5 mm in from the
            // outer edge (40 mm out).
            grid.at(74, 50).unwrap(),
            grid.at(80, 50).unwrap(),
            grid.at(85, 50).unwrap(),
        );
        assert!(middle > inner && middle > outer, "{inner} {middle} {outer}");
        assert!((inner - outer).abs() < 0.05);
    }

    #[test]
    fn invert_and_the_border_and_nothing() {
        // White is the shape: the whole picture but a black square, rising from the border.
        let square = picture(40, |x, y| {
            (10.0..30.0).contains(&x) && (10.0..30.0).contains(&y)
        });
        let white = InflateSettings {
            invert: true,
            ..ROUND
        };
        let grid = inflate_grid(&square, &white, 40.0).unwrap();
        assert_eq!(grid.at(20, 20), None);
        assert!(grid.at(0, 20).unwrap() < grid.at(5, 20).unwrap());
        let empty = picture(10, |_, _| false);
        assert!(inflate_grid(&empty, &ROUND, 10.0).is_err());
        let bad = InflateSettings {
            reach: Some(0.0),
            ..ROUND
        };
        assert!(inflate_grid(&square, &bad, 40.0).is_err());
    }
}
