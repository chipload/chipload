// SPDX-License-Identifier: GPL-3.0-or-later
//! Text into outlines (docs/specs/text.md §4, §8).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chipload_core::Point;
use chipload_core::primitives::{Primitive, distance};
use chipload_core::tolerances::{ARC_APPROX_TOL_MM, CHAIN_TOL_MM};
use chipload_text::{Align, FONTS, Layout, TextError, font_names, letters};

fn layout(text: &str) -> Layout<'_> {
    Layout {
        text,
        font: "FreeSans",
        height: 10.0,
        spacing: 0.0,
        line_spacing: 1.6,
        align: Align::Left,
    }
}

fn points(outline: &[Primitive]) -> Vec<Point> {
    outline.iter().flat_map(|p| p.points()).collect()
}

fn box_of(outline: &[Primitive]) -> (Point, Point) {
    outline.iter().map(Primitive::bounds).fold(
        (
            (f64::INFINITY, f64::INFINITY),
            (f64::NEG_INFINITY, f64::NEG_INFINITY),
        ),
        |(lo, hi), (a, b)| {
            (
                (lo.0.min(a.0), lo.1.min(a.1)),
                (hi.0.max(b.0), hi.1.max(b.1)),
            )
        },
    )
}

/// Twice the signed area of a closed polygon.
fn area2(points: &[Point]) -> f64 {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(a, b)| a.0 * b.1 - b.0 * a.1)
        .sum()
}

#[test]
fn every_built_in_font_writes_the_alphabet() {
    let text = "ABCDEFGHIJKLMNOPQRSTUVWXYZ\nabcdefghijklmnopqrstuvwxyz\n0123456789 ÅÄÖ åäö Ø ⌀ €";
    assert_eq!(FONTS.len(), 17);
    for font in font_names() {
        let l = letters(&Layout {
            font,
            ..layout(text)
        })
        .unwrap();
        assert!(
            l.outlines.len() > 60,
            "{font}: {} outlines",
            l.outlines.len()
        );
        assert!(
            l.missing.is_empty() || l.missing == ['⌀'],
            "{font}: {:?}",
            l.missing
        );
        assert_eq!(l.dropped, 0, "{font}");
    }
}

#[test]
fn an_h_is_one_closed_outline_a_capital_height_tall() {
    let l = letters(&layout("H")).unwrap();
    assert_eq!(l.outlines.len(), 1);
    let outline = &l.outlines[0];
    // Closed: each piece starts where the one before ends, the last ends at the first.
    for (a, b) in outline.iter().zip(outline.iter().cycle().skip(1)) {
        assert!(
            distance(a.end(), b.start()) <= CHAIN_TOL_MM,
            "{a:?} -> {b:?}"
        );
    }
    assert!((l.height - 10.0).abs() <= ARC_APPROX_TOL_MM, "{}", l.height);
    let ((x0, y0), _) = box_of(outline);
    assert!(
        x0.abs() < 1e-9 && y0.abs() < 1e-9,
        "the box starts at (0, 0)"
    );
}

#[test]
fn an_o_is_an_outline_with_a_hole_and_its_arcs_follow_the_curve() {
    let l = letters(&layout("O")).unwrap();
    assert_eq!(l.outlines.len(), 2);
    let areas: Vec<f64> = l.outlines.iter().map(|o| area2(&points(o)).abs()).collect();
    let (outer, inner) = if areas[0] > areas[1] { (0, 1) } else { (1, 0) };
    let ((ix0, iy0), (ix1, iy1)) = box_of(&l.outlines[inner]);
    let ((ox0, oy0), (ox1, oy1)) = box_of(&l.outlines[outer]);
    assert!(
        ox0 < ix0 && oy0 < iy0 && ix1 < ox1 && iy1 < oy1,
        "the hole is inside"
    );
    // An O is round: it is mostly arcs, not a polygon of short lines.
    let arcs = l.outlines[outer]
        .iter()
        .filter(|p| matches!(p, Primitive::Arc(_)))
        .count();
    assert!(
        arcs >= 4 && l.outlines[outer].len() < 40,
        "{:?}",
        l.outlines[outer].len()
    );
}

#[test]
fn kerning_spacing_and_lines() {
    let serif = |text, spacing| {
        letters(&Layout {
            font: "FreeSerif",
            spacing,
            ..layout(text)
        })
        .unwrap()
    };
    // FreeSerif kerns A and V together.
    let av = serif("AV", 0.0).width;
    let a = serif("A", 0.0).width;
    let v = serif("V", 0.0).width;
    let apart = serif("A V", 0.0).width;
    assert!(av < apart && av < a + v + 0.5, "AV {av}, A {a} + V {v}");
    // Spacing adds between letters, not after the last.
    let spaced = serif("AV", 2.0).width;
    assert!((spaced - av - 2.0).abs() < 1e-6, "{spaced} vs {av}");
    // Two lines: the box is about one line spacing taller than one line.
    let one = letters(&layout("HH")).unwrap();
    let two = letters(&layout("HH\nHH")).unwrap();
    assert!(
        (two.height - one.height - 16.0).abs() < 1e-6,
        "{}",
        two.height
    );
}

#[test]
fn lines_align_left_centre_and_right() {
    let narrow_left_edge = |align| {
        let l = letters(&Layout {
            align,
            ..layout("HHHH\nH")
        })
        .unwrap();
        // The lower line's H: the outline with the lowest box.
        let low = l
            .outlines
            .iter()
            .min_by(|a, b| box_of(a).0.1.total_cmp(&box_of(b).0.1))
            .unwrap();
        (box_of(low).0.0, l.width)
    };
    let (left, width) = narrow_left_edge(Align::Left);
    let (centre, _) = narrow_left_edge(Align::Center);
    let (right, _) = narrow_left_edge(Align::Right);
    assert!(left.abs() < 0.5, "{left}");
    assert!(
        centre > width / 3.0 && centre < width / 2.0,
        "{centre} of {width}"
    );
    assert!(right > width * 0.7, "{right} of {width}");
}

#[test]
fn the_same_text_gives_the_same_outlines() {
    let a = letters(&layout("Chipload 0.19")).unwrap();
    let b = letters(&layout("Chipload 0.19")).unwrap();
    assert_eq!(a, b);
}

#[test]
fn what_cant_be_written_is_said() {
    assert_eq!(letters(&layout("  \n ")), Err(TextError::Empty));
    assert!(matches!(
        letters(&Layout {
            font: "Comic Sans",
            ..layout("A")
        }),
        Err(TextError::UnknownFont(..))
    ));
    assert!(matches!(
        letters(&Layout {
            height: 0.0,
            ..layout("A")
        }),
        Err(TextError::Height(_))
    ));
    // A letter the font lacks is left out and named.
    let l = letters(&layout("A☃B")).unwrap();
    assert_eq!(l.missing, ['☃']);
    assert_eq!(letters(&layout("☃")), Err(TextError::NoLetters));
}

#[test]
fn each_font_carries_its_licence_in_its_name_table() {
    for font in chipload_text::FONTS {
        let face = ttf_parser::Face::parse(font.data(), 0).unwrap();
        let has = |id: u16| {
            face.names()
                .into_iter()
                .any(|n| n.name_id == id && n.to_string().is_some_and(|t| !t.is_empty()))
        };
        // 0: copyright; 13: the licence (or 14, its address). Yellowtail's file has no
        // licence entry: it is Apache-2.0 by Google Fonts' metadata, and
        // fonts/LICENSE-Apache-2.0.txt is shipped with it (docs/specs/more-fonts.md §2).
        assert!(has(0), "{}: no copyright", font.name);
        assert!(
            has(13) || has(14) || font.name == "Yellowtail",
            "{}: no licence",
            font.name
        );
    }
}

/// A font file's bytes give the same letters as the built-in font of that file
/// (docs/specs/system-fonts.md §8).
#[test]
fn a_font_files_bytes_give_the_same_letters_as_its_name() {
    use chipload_text::{FontFile, builtin, letters_in};
    for name in ["FreeSans", "Satisfy"] {
        let font = builtin(name).unwrap();
        let by_name = letters(&Layout {
            font: name,
            ..layout("Chipload 42")
        })
        .unwrap();
        let file = FontFile {
            name: "Some computer font",
            data: font.data(),
            index: 0,
        };
        // The layout's own font name isn't looked at.
        let by_bytes = letters_in(&layout("Chipload 42"), file).unwrap();
        assert_eq!(by_name, by_bytes, "{name}");
    }
    let broken = FontFile {
        name: "Broken",
        data: b"not a font",
        index: 0,
    };
    assert_eq!(
        letters_in(&layout("A"), broken),
        Err(TextError::BadFont("Broken".into()))
    );
}

/// Every built-in font's file has a name and letters; a file that isn't a font has none.
#[test]
fn faces_are_named_by_their_full_name() {
    use chipload_text::face_name;
    for font in FONTS {
        let name = face_name(font.data(), 0);
        assert!(
            name.as_deref().is_some_and(|n| !n.is_empty()),
            "{}",
            font.name
        );
    }
    assert_eq!(
        face_name(chipload_text::builtin("FreeSans Bold").unwrap().data(), 0).as_deref(),
        Some("FreeSans Bold")
    );
    assert_eq!(face_name(b"not a font", 0), None);
    assert_eq!(face_name(FONTS[0].data(), 1), None, "no second face");
}
