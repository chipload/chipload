// SPDX-License-Identifier: GPL-3.0-or-later
//! Tracing pictures (docs/specs/image-trace.md §9).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::Cursor;

use chipload_core::Point;
use chipload_core::primitives::signed_area;
use chipload_image::{ImageError, Settings, Threshold, decode, trace};
use image::{ImageBuffer, ImageFormat, LumaA, Rgba};
use proptest::prelude::*;

/// A PNG of `w` × `h` pixels, black where `black(column, row)` (row 0 at the top).
fn png(w: u32, h: u32, black: impl Fn(u32, u32) -> bool) -> Vec<u8> {
    let picture = ImageBuffer::from_fn(w, h, |x, y| {
        if black(x, y) {
            Rgba([0u8, 0, 0, 255])
        } else {
            Rgba([255u8, 255, 255, 255])
        }
    });
    let mut bytes = Vec::new();
    picture
        .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .unwrap();
    bytes
}

/// Sharp: no blur, threshold in the middle, smoothing as small as allowed, nothing dropped.
fn sharp() -> Settings {
    Settings {
        threshold: Threshold::Level(128),
        invert: false,
        blur: 0.0,
        min_area: 0.0,
        smoothing: 0.01,
        arcs: false,
    }
}

/// How often the loops wind around `p` (outlines +1, holes −1).
fn winding(loops: &[Vec<Point>], p: Point) -> i32 {
    let mut w = 0;
    for ring in loops {
        for k in 0..ring.len() {
            let (a, b) = (ring[k], ring[(k + 1) % ring.len()]);
            let cross = (b.0 - a.0) * (p.1 - a.1) - (p.0 - a.0) * (b.1 - a.1);
            if a.1 <= p.1 && b.1 > p.1 && cross > 0.0 {
                w += 1;
            } else if a.1 > p.1 && b.1 <= p.1 && cross < 0.0 {
                w -= 1;
            }
        }
    }
    w
}

/// Distance from `p` to the nearest loop segment.
fn edge_distance(loops: &[Vec<Point>], p: Point) -> f64 {
    let mut best = f64::INFINITY;
    for ring in loops {
        for k in 0..ring.len() {
            let (a, b) = (ring[k], ring[(k + 1) % ring.len()]);
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
            best = best.min((p.0 - a.0 - t * dx).hypot(p.1 - a.1 - t * dy));
        }
    }
    best
}

/// Whether a-b and c-d meet. Segments whose boxes are apart never do, though on one line
/// their orientations are zero but for the last bits.
fn crosses(a: Point, b: Point, c: Point, d: Point) -> bool {
    let apart = |p: f64, q: f64, r: f64, s: f64| p.max(q) < r.min(s) || r.max(s) < p.min(q);
    if apart(a.0, b.0, c.0, d.0) || apart(a.1, b.1, c.1, d.1) {
        return false;
    }
    let o = |p: Point, q: Point, r: Point| (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0);
    let (o1, o2, o3, o4) = (o(a, b, c), o(a, b, d), o(c, d, a), o(c, d, b));
    o1 * o2 <= 0.0 && o3 * o4 <= 0.0 && !(o1 == 0.0 && o2 == 0.0)
}

/// No segment meets another, except neighbours in one loop.
fn no_crossings(loops: &[Vec<Point>]) -> bool {
    let segments: Vec<(usize, usize, Point, Point)> = loops
        .iter()
        .enumerate()
        .flat_map(|(l, r)| (0..r.len()).map(move |k| (l, k, r[k], r[(k + 1) % r.len()])))
        .collect();
    for (m, &(l1, k1, a, b)) in segments.iter().enumerate() {
        for &(l2, k2, c, d) in &segments[m + 1..] {
            let n = loops[l1].len();
            if l1 == l2 && ((k1 + 1) % n == k2 || (k2 + 1) % n == k1) {
                continue;
            }
            if crosses(a, b, c, d) {
                return false;
            }
        }
    }
    true
}

#[test]
fn a_black_square_traces_through_the_middle_between_pixels() {
    let bytes = png(10, 10, |x, y| (3..7).contains(&x) && (3..7).contains(&y));
    let t = trace(&decode(&bytes).unwrap(), &sharp(), 10.0).unwrap();
    assert_eq!(t.loops.len(), 1);
    let ring = &t.loops[0];
    // A 4 × 4 mm square whose corners are cut by half a pixel's diagonal.
    assert!(
        (signed_area(ring) - 15.5).abs() < 1e-9,
        "{}",
        signed_area(ring)
    );
    for &(x, y) in ring {
        assert!(
            (3.0..=7.0).contains(&x) && (3.0..=7.0).contains(&y),
            "{x} {y}"
        );
    }
    assert_eq!(t.height, 10.0);
    assert!(!t.black_at_border);
}

/// Every loop's pieces join end to end and close, as many loops as points.
fn pieces_close(t: &chipload_image::Trace) -> bool {
    let near = |a: Point, b: Point| (a.0 - b.0).hypot(a.1 - b.1) < 1e-9;
    t.pieces.len() == t.loops.len()
        && t.pieces.iter().all(|pieces| {
            !pieces.is_empty()
                && pieces.windows(2).all(|w| near(w[0].end(), w[1].start()))
                && near(pieces[pieces.len() - 1].end(), pieces[0].start())
        })
}

/// docs/specs/image-arcs-centrelines.md §8: a disk with arcs is a handful of arcs, round
/// within the smoothing; a square stays its lines.
#[test]
fn with_arcs_a_disk_is_a_few_arcs_and_a_square_its_lines() {
    use chipload_core::primitives::Primitive;
    let (n, r) = (200u32, 80.0f64);
    let bytes = png(n, n, |x, y| {
        let (dx, dy) = (f64::from(x) + 0.5 - 100.0, f64::from(y) + 0.5 - 100.0);
        dx.hypot(dy) <= r
    });
    let settings = Settings {
        blur: 1.0,
        smoothing: 0.05,
        arcs: true,
        ..sharp()
    };
    // 0.1 mm pixels, as a 20 mm logo of 200 pixels.
    let t = trace(&decode(&bytes).unwrap(), &settings, 20.0).unwrap();
    assert_eq!(t.pieces.len(), 1);
    let arcs = t.pieces[0]
        .iter()
        .filter(|p| matches!(p, Primitive::Arc(_)))
        .count();
    assert!(arcs >= 2, "{:?}", t.pieces[0]);
    assert!(t.pieces[0].len() <= 24, "{} pieces", t.pieces[0].len());
    let lines = trace(
        &decode(&bytes).unwrap(),
        &Settings {
            arcs: false,
            ..settings
        },
        20.0,
    )
    .unwrap();
    assert!(
        lines.pieces[0].len() > 2 * t.pieces[0].len(),
        "{} lines against {} with arcs",
        lines.pieces[0].len(),
        t.pieces[0].len()
    );
    assert_eq!(t.without_arcs, 0);
    let square = png(10, 10, |x, y| (3..7).contains(&x) && (3..7).contains(&y));
    let t = trace(
        &decode(&square).unwrap(),
        &Settings {
            arcs: true,
            ..sharp()
        },
        10.0,
    )
    .unwrap();
    assert!(t.pieces[0].iter().all(|p| matches!(p, Primitive::Line(_))));
    assert!((signed_area(&t.loops[0]) - 15.5).abs() < 1e-9);
}

#[test]
fn a_disk_traces_round() {
    let (n, r) = (60u32, 20.0f64);
    let bytes = png(n, n, |x, y| {
        let (dx, dy) = (f64::from(x) + 0.5 - 30.0, f64::from(y) + 0.5 - 30.0);
        dx.hypot(dy) <= r
    });
    let settings = Settings {
        blur: 1.0,
        ..sharp()
    };
    let t = trace(&decode(&bytes).unwrap(), &settings, 60.0).unwrap();
    assert_eq!(t.loops.len(), 1);
    for &(x, y) in &t.loops[0] {
        let d = (x - 30.0).hypot(y - 30.0);
        assert!((d - r).abs() < 0.75, "{d}");
    }
}

#[test]
fn invert_carves_the_light_parts() {
    let bytes = png(10, 10, |x, y| (3..7).contains(&x) && (3..7).contains(&y));
    let settings = Settings {
        invert: true,
        ..sharp()
    };
    let t = trace(&decode(&bytes).unwrap(), &settings, 10.0).unwrap();
    assert_eq!(t.loops.len(), 2);
    assert!(t.black_at_border);
    let areas: Vec<f64> = t.loops.iter().map(|l| signed_area(l)).collect();
    assert!(areas.iter().any(|&a| a > 0.0) && areas.iter().any(|&a| a < 0.0));
    assert_eq!(winding(&t.loops, (5.0, 5.0)), 0);
    assert_eq!(winding(&t.loops, (1.0, 1.0)), 1);
}

#[test]
fn specks_smaller_than_the_smallest_piece_are_dropped() {
    let bytes = png(20, 20, |x, y| {
        ((2..12).contains(&x) && (2..12).contains(&y)) || (x == 16 && y == 16)
    });
    let settings = Settings {
        min_area: 2.0,
        ..sharp()
    };
    let t = trace(&decode(&bytes).unwrap(), &settings, 20.0).unwrap();
    assert_eq!(t.loops.len(), 1);
    assert_eq!(t.dropped, 1);
}

#[test]
fn transparency_counts_as_white() {
    let picture = ImageBuffer::from_fn(10, 10, |x, y| {
        if (3..7).contains(&x) && (3..7).contains(&y) {
            LumaA([0u8, 255])
        } else {
            LumaA([0u8, 0])
        }
    });
    let mut bytes = Vec::new();
    picture
        .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .unwrap();
    let t = trace(&decode(&bytes).unwrap(), &sharp(), 10.0).unwrap();
    assert_eq!(t.loops.len(), 1);
    assert!((signed_area(&t.loops[0]) - 15.5).abs() < 1e-9);
}

#[test]
fn auto_picks_a_threshold_between_the_two_greys() {
    let picture = ImageBuffer::from_fn(10, 10, |x, _| {
        if x < 5 {
            Rgba([60u8, 60, 60, 255])
        } else {
            Rgba([190u8, 190, 190, 255])
        }
    });
    let mut bytes = Vec::new();
    picture
        .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .unwrap();
    let settings = Settings {
        threshold: Threshold::Auto,
        ..sharp()
    };
    let t = trace(&decode(&bytes).unwrap(), &settings, 10.0).unwrap();
    assert!(t.threshold > 60 && t.threshold <= 190, "{}", t.threshold);
    assert_eq!(t.loops.len(), 1);
}

#[test]
fn a_checkerboard_saddle_traces_the_same_every_time() {
    let bytes = png(2, 2, |x, y| x == y);
    let picture = decode(&bytes).unwrap();
    let first = trace(&picture, &sharp(), 2.0).unwrap();
    for _ in 0..3 {
        assert_eq!(trace(&picture, &sharp(), 2.0).unwrap(), first);
    }
    assert!(no_crossings(&first.loops));
}

#[test]
fn bad_input_is_an_error() {
    assert_eq!(decode(b"GIF89a....."), Err(ImageError::UnsupportedFormat));
    assert!(matches!(
        decode(b"not a picture"),
        Err(ImageError::UnsupportedFormat | ImageError::Decode(_))
    ));
    let mut png_header = png(4, 4, |_, _| false);
    png_header.truncate(40);
    assert!(matches!(decode(&png_header), Err(ImageError::Decode(_))));
    let picture = decode(&png(4, 4, |_, _| false)).unwrap();
    assert_eq!(trace(&picture, &sharp(), 0.0), Err(ImageError::Width));
    let bad = Settings {
        smoothing: 5.0,
        ..sharp()
    };
    assert!(matches!(
        trace(&picture, &bad, 10.0),
        Err(ImageError::Setting(_))
    ));
}

#[test]
fn an_all_white_picture_has_no_loops() {
    let t = trace(&decode(&png(8, 8, |_, _| false)).unwrap(), &sharp(), 8.0).unwrap();
    assert!(t.loops.is_empty());
}

/// 64 cases; `SOAK=N` for more.
fn cases() -> u32 {
    std::env::var("SOAK")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(64)
}

proptest! {
    #![proptest_config(ProptestConfig {
        failure_persistence: Some(Box::new(
            proptest::test_runner::FileFailurePersistence::WithSource("proptest-regressions"),
        )),
        ..ProptestConfig::with_cases(cases())
    })]

    /// Every black pixel centre is inside the loops and every white one outside, and no
    /// loops cross (spec §3, §9).
    #[test]
    fn traced_loops_separate_black_from_white(
        w in 1u32..12,
        h in 1u32..12,
        bits in proptest::collection::vec(any::<bool>(), 144),
        smoothing in 0.01f64..0.3,
        arcs in any::<bool>(),
    ) {
        let black = |x: u32, y: u32| bits[(y * 12 + x) as usize];
        let bytes = png(w, h, black);
        let settings = Settings { smoothing, arcs, ..sharp() };
        let t = trace(&decode(&bytes).unwrap(), &settings, f64::from(w)).unwrap();
        prop_assert!(no_crossings(&t.loops));
        prop_assert!(pieces_close(&t));
        for y in 0..h {
            for x in 0..w {
                let centre = (f64::from(x) + 0.5, f64::from(h - y) - 0.5);
                let inside = winding(&t.loops, centre);
                prop_assert_eq!(inside, i32::from(black(x, y)), "pixel {} {}", x, y);
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        failure_persistence: Some(Box::new(
            proptest::test_runner::FileFailurePersistence::WithSource("proptest-regressions"),
        )),
        ..ProptestConfig::with_cases(cases())
    })]

    /// With blur, any threshold and invert, the loops enclose exactly the pixels the
    /// black-and-white preview shows black (spec §3.1).
    #[test]
    fn traced_loops_match_the_preview(
        w in 1u32..10,
        h in 1u32..10,
        greys in proptest::collection::vec(any::<u8>(), 100),
        blur in 0.0f64..2.0,
        threshold in 1u8..=254,
        invert in any::<bool>(),
        arcs in any::<bool>(),
    ) {
        let picture = ImageBuffer::from_fn(w, h, |x, y| {
            let g = greys[(y * 10 + x) as usize];
            Rgba([g, g, g, 255])
        });
        let mut bytes = Vec::new();
        picture.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png).unwrap();
        let decoded = decode(&bytes).unwrap();
        let settings = Settings { blur, threshold: Threshold::Level(threshold), invert, arcs, ..sharp() };
        let t = trace(&decoded, &settings, f64::from(w)).unwrap();
        let bw = chipload_image::black_white(&decoded, &settings).unwrap();
        prop_assert!(no_crossings(&t.loops));
        prop_assert!(pieces_close(&t));
        for y in 0..h {
            for x in 0..w {
                let centre = (f64::from(x) + 0.5, f64::from(h - y) - 0.5);
                let black = bw.black[(y * w + x) as usize];
                // Simplifying moves edges by up to the smoothing: a centre that close may
                // land on either side.
                if edge_distance(&t.loops, centre) > settings.smoothing {
                    prop_assert_eq!(winding(&t.loops, centre), i32::from(black), "pixel {} {}", x, y);
                }
            }
        }
    }
}

/// A diagonal staircase of a few pixels with arcs (proptest seed cc 2c938f78…, found
/// 2026-10-28). The loops never crossed: a lone pixel's edge and the staircase's closing
/// chord lie on one line, y = x + 0.5, half a millimetre apart, and a last-bit error made
/// the orientation test call them crossing. `crosses` now wants their boxes to overlap.
#[test]
fn a_tiny_staircase_with_arcs_traces_without_crossings() {
    let rows = ["....", "...#", "..##", ".###", "#..."];
    let bytes = png(4, 5, |x, y| rows[y as usize].as_bytes()[x as usize] == b'#');
    let settings = Settings {
        smoothing: 0.135,
        arcs: true,
        ..sharp()
    };
    let t = trace(&decode(&bytes).unwrap(), &settings, 4.0).unwrap();
    assert!(no_crossings(&t.loops));
}
