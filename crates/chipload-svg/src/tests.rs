// SPDX-License-Identifier: GPL-3.0-or-later
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use chipload_core::primitives::Arc;

/// An SVG page `w` × `h` mm with a matching viewBox, so user units are mm.
fn page(w: f64, h: f64, body: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg"
             xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape"
             width="{w}mm" height="{h}mm" viewBox="0 0 {w} {h}">{body}</svg>"#
    )
}

fn read(svg: &str) -> SvgDrawing {
    read_svg(svg.as_bytes(), None, 1.0).unwrap()
}

fn bounds(shape: &SvgShape) -> (Point, Point) {
    shape
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

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-3
}

#[test]
fn an_inkscape_file_in_mm_gives_shapes_on_its_layers_with_y_up() {
    let svg = page(
        100.0,
        100.0,
        r##"<g id="layer1" inkscape:groupmode="layer" inkscape:label="Cut">
              <rect id="box" x="10" y="10" width="10" height="20" fill="none" stroke="#000"/>
              <g id="layer2" inkscape:groupmode="layer" inkscape:label="Inner">
                <circle id="hole" cx="50" cy="50" r="5"/>
              </g>
            </g>
            <path id="loose" d="M 0 0 L 5 5"/>"##,
    );
    let out = read(&svg);
    assert!(out.units_in_file);
    assert_eq!(out.shapes.len(), 3);
    let rect = &out.shapes[0];
    assert_eq!((rect.layer.as_str(), rect.key.as_str()), ("Cut", "box#1"));
    assert!(rect.closed);
    assert_eq!(rect.prims.len(), 4);
    // y = 10..30 down from the top of a 100 mm page: 70..90 up from its foot.
    let ((x0, y0), (x1, y1)) = bounds(rect);
    assert!(near(x0, 10.0) && near(x1, 20.0) && near(y0, 70.0) && near(y1, 90.0));
    // A nested layer belongs to the outermost one; a circle comes back as arcs.
    let hole = &out.shapes[1];
    assert_eq!(hole.layer, "Cut");
    assert!(hole.closed);
    let arcs: Vec<&Arc> = hole
        .prims
        .iter()
        .filter_map(|p| match p {
            Primitive::Arc(a) => Some(a),
            Primitive::Line(_) => None,
        })
        .collect();
    assert!(arcs.len() >= 2);
    for arc in &arcs {
        assert!((arc.radius - 5.0).abs() < 0.01, "{arc:?}");
    }
    // Nearly all of it: at most a short closing piece is a line.
    let swept: f64 = arcs.iter().map(|a| a.sweep.abs()).sum();
    assert!(swept > std::f64::consts::TAU * 0.95, "{swept}");
    // Outside every layer in a file that has layers: layer 0, and open.
    assert_eq!(out.shapes[2].layer, "0");
    assert!(!out.shapes[2].closed);
}

#[test]
fn without_layers_paths_go_on_their_colour() {
    let svg = page(
        50.0,
        50.0,
        r##"<path d="M 1 1 H 9" stroke="red" fill="none"/>
            <path d="M 1 2 H 9 V 8 Z" fill="#0000ff"/>
            <path d="M 1 3 H 9" fill="none"/>"##,
    );
    let out = read(&svg);
    let layers: Vec<&str> = out.shapes.iter().map(|s| s.layer.as_str()).collect();
    assert_eq!(layers, ["#ff0000", "#0000ff", "none"]);
    // The path with neither fill nor stroke is imported, and counted.
    assert_eq!(out.counts.hidden, 1);
    let keys: Vec<&str> = out.shapes.iter().map(|s| s.key.as_str()).collect();
    assert_eq!(keys, ["@1#1", "@2#1", "@3#1"]);
}

#[test]
fn a_file_in_plain_pixels_needs_its_pixels_per_inch() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100">
                   <rect x="0" y="0" width="96" height="48"/></svg>"#;
    assert_eq!(
        read_svg(svg.as_bytes(), None, 1.0),
        Err(SvgError::NeedsScale)
    );
    let out = read_svg(svg.as_bytes(), Some(96.0), 1.0).unwrap();
    assert!(!out.units_in_file);
    let ((x0, _), (x1, y1)) = bounds(&out.shapes[0]);
    assert!(near(x1 - x0, 25.4));
    // At the top of a 100 px page: its top edge is 100 px up.
    assert!(near(y1, 25.4 * 100.0 / 96.0));
    let at_72 = read_svg(svg.as_bytes(), Some(72.0), 1.0).unwrap();
    let ((x0, _), (x1, _)) = bounds(&at_72.shapes[0]);
    assert!(near(x1 - x0, 25.4 * 96.0 / 72.0));
    assert_eq!(
        read_svg(svg.as_bytes(), Some(0.0), 1.0),
        Err(SvgError::BadScale(0.0))
    );
}

#[test]
fn transforms_and_use_are_applied() {
    let svg = page(
        100.0,
        100.0,
        r##"<defs><path id="tick" d="M 0 0 L 10 0"/></defs>
            <g transform="translate(20 30) scale(2)">
              <use href="#tick"/>
              <use href="#tick" y="5"/>
            </g>"##,
    );
    let out = read(&svg);
    assert_eq!(out.shapes.len(), 2);
    // usvg leaves a used element's id off its copies: they are keyed by order.
    assert_eq!(out.shapes[0].key, "@1#1");
    assert_eq!(out.shapes[1].key, "@2#1");
    let ((x0, y0), (x1, _)) = bounds(&out.shapes[1]);
    assert!(near(x0, 20.0) && near(x1, 40.0) && near(y0, 100.0 - 40.0));
}

#[test]
fn a_scale_is_applied_about_the_page_foot_with_curves_fitted_at_that_size() {
    let svg = page(100.0, 100.0, r#"<circle cx="50" cy="90" r="5"/>"#);
    let out = read_svg(svg.as_bytes(), None, 4.0).unwrap();
    let ((x0, y0), (x1, y1)) = bounds(&out.shapes[0]);
    // Fitted arcs may bulge within the arc tolerance.
    let fitted = |a: f64, b: f64| (a - b).abs() <= ARC_APPROX_TOL_MM;
    assert!(fitted(x0, 180.0) && fitted(x1, 220.0) && fitted(y0, 20.0) && fitted(y1, 60.0));
}

#[test]
fn subpaths_are_shapes_and_z_closes_them() {
    let svg = page(
        100.0,
        100.0,
        r#"<path id="two" d="M 0 0 H 10 V 10 Z M 20 0 H 30 V 10 H 20 V 0 M 40 0 H 50"/>"#,
    );
    let out = read(&svg);
    let shapes: Vec<(&str, bool)> = out
        .shapes
        .iter()
        .map(|s| (s.key.as_str(), s.closed))
        .collect();
    // The second closes by its ends meeting; the third is open.
    assert_eq!(shapes, [("two#1", true), ("two#2", true), ("two#3", false)]);
}

#[test]
fn text_pictures_and_clips_are_counted_not_imported() {
    let svg = page(
        100.0,
        100.0,
        r#"<defs><clipPath id="c"><rect width="5" height="5"/></clipPath></defs>
           <text x="10" y="10">Hello</text>
           <image x="0" y="0" width="10" height="10" href="secret.png"/>
           <rect id="clipped" width="20" height="20" clip-path="url(#c)"/>"#,
    );
    let out = read(&svg);
    assert_eq!(
        (out.counts.texts, out.counts.images, out.counts.clips),
        (1, 1, 1)
    );
    // The clipped rectangle is imported whole.
    assert_eq!(out.shapes.len(), 1);
    let ((x0, _), (x1, _)) = bounds(&out.shapes[0]);
    assert!(near(x1 - x0, 20.0));
}

#[test]
fn an_svgz_is_read_like_the_svg() {
    use std::io::Write;
    let svg = page(10.0, 10.0, r#"<rect width="5" height="5"/>"#);
    let mut packed = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    packed.write_all(svg.as_bytes()).unwrap();
    let packed = packed.finish().unwrap();
    assert_eq!(read_svg(&packed, None, 1.0).unwrap(), read(&svg));
    assert!(matches!(
        read_svg(&packed[..packed.len() / 2], None, 1.0),
        Err(SvgError::Unreadable(_))
    ));
}

#[test]
fn hostile_files_are_refused_never_crash() {
    let deep = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="1mm" height="1mm">{}{}</svg>"#,
        "<g>".repeat(10_000),
        "</g>".repeat(10_000)
    );
    assert_eq!(read_svg(deep.as_bytes(), None, 1.0), Err(SvgError::TooDeep));
    let bomb = r#"<?xml version="1.0"?>
        <!DOCTYPE svg [
          <!ENTITY a "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa">
          <!ENTITY b "&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;">
          <!ENTITY c "&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;">
          <!ENTITY d "&c;&c;&c;&c;&c;&c;&c;&c;&c;&c;&c;&c;&c;&c;&c;&c;">
          <!ENTITY e "&d;&d;&d;&d;&d;&d;&d;&d;&d;&d;&d;&d;&d;&d;&d;&d;">
          <!ENTITY f "&e;&e;&e;&e;&e;&e;&e;&e;&e;&e;&e;&e;&e;&e;&e;&e;">
        ]>
        <svg xmlns="http://www.w3.org/2000/svg" width="1mm" height="1mm"><text>&f;</text></svg>"#;
    assert!(matches!(
        read_svg(bomb.as_bytes(), None, 1.0),
        Err(SvgError::Unreadable(_))
    ));
    let far = page(10.0, 10.0, r#"<path id="far" d="M 0 0 L 1e19 0"/>"#);
    assert_eq!(
        read_svg(far.as_bytes(), None, 1.0),
        Err(SvgError::FarOut("far".into()))
    );
    for junk in [&b"not xml at all"[..], &[0xff, 0xfe, 0x00][..], b"<svg"] {
        assert!(matches!(
            read_svg(junk, None, 1.0),
            Err(SvgError::Unreadable(_))
        ));
    }
    assert!(matches!(
        read_svg(b"", None, f64::NAN),
        Err(SvgError::BadScale(_))
    ));
}

#[test]
fn zero_length_subpaths_are_dropped_and_counted() {
    let svg = page(10.0, 10.0, r#"<path d="M 1 1 L 1 1 M 2 2 H 4"/>"#);
    let out = read(&svg);
    assert_eq!(out.shapes.len(), 1);
    assert_eq!(out.counts.dropped, 1);
}

/// How far `p` is from `prim`.
fn distance_to(prim: &Primitive, p: Point) -> f64 {
    match prim {
        Primitive::Line(l) => {
            let (dx, dy) = (l.end.0 - l.start.0, l.end.1 - l.start.1);
            let len2 = dx * dx + dy * dy;
            let t = if len2 > 0.0 {
                (((p.0 - l.start.0) * dx + (p.1 - l.start.1) * dy) / len2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            distance(p, (l.start.0 + t * dx, l.start.1 + t * dy))
        }
        Primitive::Arc(a) => {
            let on_circle = (distance(p, a.center) - a.radius).abs();
            if within(a, (p.1 - a.center.1).atan2(p.0 - a.center.0)) {
                on_circle
            } else {
                distance(p, a.point_at(a.start_angle))
                    .min(distance(p, a.point_at(a.start_angle + a.sweep)))
            }
        }
    }
}

fn within(arc: &Arc, angle: f64) -> bool {
    let tau = std::f64::consts::TAU;
    let from = arc.start_angle.rem_euclid(tau);
    let rel = if arc.sweep >= 0.0 {
        (angle - from).rem_euclid(tau)
    } else {
        (from - angle).rem_euclid(tau)
    };
    rel <= arc.sweep.abs()
}

mod properties {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// A cubic Bézier comes back as lines and arcs that stay within the arc tolerance
        /// of the curve everywhere.
        #[test]
        fn a_fitted_cubic_stays_within_the_tolerance(
            c in prop::collection::vec(0.0..100.0f64, 8),
        ) {
            let d = format!(
                "M {} {} C {} {} {} {} {} {}",
                c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]
            );
            let svg = page(100.0, 100.0, &format!(r#"<path d="{d}"/>"#));
            let out = read(&svg);
            let prims: Vec<&Primitive> = out.shapes.iter().flat_map(|s| &s.prims).collect();
            prop_assume!(!prims.is_empty());
            for k in 0..=400 {
                let t = f64::from(k) / 400.0;
                let u = 1.0 - t;
                let (a, b, e, f) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
                let x = a * c[0] + b * c[2] + e * c[4] + f * c[6];
                let y = a * c[1] + b * c[3] + e * c[5] + f * c[7];
                let p = (x, 100.0 - y);
                let gap = prims.iter().map(|q| distance_to(q, p)).fold(f64::INFINITY, f64::min);
                // usvg holds coordinates as f32: a few ten-thousandths of a mm at 100 mm.
                prop_assert!(gap <= ARC_APPROX_TOL_MM + 1e-3, "{gap} at t = {t}");
            }
        }
    }
}
