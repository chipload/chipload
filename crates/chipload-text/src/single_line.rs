// SPDX-License-Identifier: GPL-3.0-or-later
//! Single-line fonts: letters as open strokes for engraving with a follow path
//! (docs/specs/single-line-text.md). The font is an SVG font, read here: `<font-face>`
//! for the units and the capital height, a `<glyph>` per character with its strokes as path
//! data (Y up, font units) and its advance, and `<hkern>` pairs by glyph name or character.

use std::collections::BTreeMap;

use chipload_core::Point;
use chipload_core::fit::fit_path;
use chipload_core::primitives::{Primitive, distance};
use chipload_core::tolerances::{ARC_APPROX_TOL_MM, CHAIN_TOL_MM};
use svgtypes::{SimplePathSegment, SimplifyingPathParser};

use crate::{Align, Layout, Lettering, TextError};

/// A glyph: its advance and its path data, font units.
struct Glyph {
    advance: f64,
    path: String,
}

/// A single-line font as read.
struct SvgFont {
    cap_height: f64,
    glyphs: BTreeMap<char, Glyph>,
    /// Kerning by pair of characters, font units to take off the advance.
    kerning: BTreeMap<(char, char), f64>,
}

/// The font file `data` read. `name` names it in errors.
fn read(data: &[u8], name: &str) -> Result<SvgFont, TextError> {
    let bad = || TextError::BadFont(name.to_string());
    let text = std::str::from_utf8(data).map_err(|_| bad())?;
    // The bundled font names the SVG 1.1 DTD; nothing in it uses entities from it.
    let options = roxmltree::ParsingOptions {
        allow_dtd: true,
        ..roxmltree::ParsingOptions::default()
    };
    let doc = roxmltree::Document::parse_with_options(text, options).map_err(|_| bad())?;
    let font = doc
        .descendants()
        .find(|n| n.has_tag_name("font"))
        .ok_or_else(bad)?;
    let number = |node: roxmltree::Node, attr: &str| -> Option<f64> {
        node.attribute(attr)?.trim().parse::<f64>().ok()
    };
    let default_advance = number(font, "horiz-adv-x").unwrap_or(0.0);
    let face = font
        .children()
        .find(|n| n.has_tag_name("font-face"))
        .ok_or_else(bad)?;
    let cap_height = number(face, "cap-height")
        .filter(|c| *c > 0.0)
        .ok_or_else(bad)?;
    let mut glyphs = BTreeMap::new();
    let mut by_name: BTreeMap<String, char> = BTreeMap::new();
    for node in font.children().filter(|n| n.has_tag_name("glyph")) {
        // One character each; ligatures (several characters) are left out.
        let mut chars = node.attribute("unicode").unwrap_or("").chars();
        let (Some(c), None) = (chars.next(), chars.next()) else {
            continue;
        };
        if let Some(name) = node.attribute("glyph-name") {
            by_name.insert(name.to_string(), c);
        }
        glyphs.insert(
            c,
            Glyph {
                advance: number(node, "horiz-adv-x").unwrap_or(default_advance),
                path: node.attribute("d").unwrap_or("").to_string(),
            },
        );
    }
    if glyphs.is_empty() {
        return Err(bad());
    }
    let mut kerning = BTreeMap::new();
    for node in font.children().filter(|n| n.has_tag_name("hkern")) {
        let Some(k) = number(node, "k") else {
            continue;
        };
        let side = |names: &str, chars: &str| -> Vec<char> {
            let mut out: Vec<char> = node
                .attribute(names)
                .unwrap_or("")
                .split(',')
                .filter_map(|n| by_name.get(n.trim()).copied())
                .collect();
            out.extend(
                node.attribute(chars)
                    .unwrap_or("")
                    .split(',')
                    .filter_map(|u| {
                        let mut cs = u.trim().chars();
                        match (cs.next(), cs.next()) {
                            (Some(c), None) => Some(c),
                            _ => None,
                        }
                    }),
            );
            out
        };
        for left in side("g1", "u1") {
            for &right in &side("g2", "u2") {
                kerning.insert((left, right), k);
            }
        }
    }
    Ok(SvgFont {
        cap_height,
        glyphs,
        kerning,
    })
}

/// A glyph's strokes, mm, placed at `origin` and scaled by `scale`, curves flattened
/// within `flat`: each with whether it closes on itself.
fn strokes(path: &str, origin: Point, scale: f64, flat: f64) -> Vec<(Vec<Point>, bool)> {
    let at = |x: f64, y: f64| (origin.0 + x * scale, origin.1 + y * scale);
    let mut out: Vec<(Vec<Point>, bool)> = Vec::new();
    let mut current: Vec<Point> = Vec::new();
    let finish = |current: &mut Vec<Point>, out: &mut Vec<(Vec<Point>, bool)>, closed: bool| {
        if current.len() > 1 {
            out.push((std::mem::take(current), closed));
        } else {
            current.clear();
        }
    };
    for segment in SimplifyingPathParser::from(path) {
        // A path the parser can't read further ends there; what came before stays.
        let Ok(segment) = segment else {
            break;
        };
        match segment {
            SimplePathSegment::MoveTo { x, y } => {
                finish(&mut current, &mut out, false);
                current.push(at(x, y));
            }
            SimplePathSegment::LineTo { x, y } => current.push(at(x, y)),
            SimplePathSegment::Quadratic { x1, y1, x, y } => {
                let p0 = current.last().copied().unwrap_or(origin);
                let (p1, p2) = (at(x1, y1), at(x, y));
                let bend = 2.0 * distance((p0.0 - p1.0, p0.1 - p1.1), (p1.0 - p2.0, p1.1 - p2.1));
                curve(&mut current, bend, flat, |t| {
                    let u = 1.0 - t;
                    (
                        u * u * p0.0 + 2.0 * u * t * p1.0 + t * t * p2.0,
                        u * u * p0.1 + 2.0 * u * t * p1.1 + t * t * p2.1,
                    )
                });
            }
            SimplePathSegment::CurveTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                let p0 = current.last().copied().unwrap_or(origin);
                let (p1, p2, p3) = (at(x1, y1), at(x2, y2), at(x, y));
                let second = |a: Point, b: Point, c: Point| {
                    distance((a.0 - b.0, a.1 - b.1), (b.0 - c.0, b.1 - c.1))
                };
                let bend = 6.0 * second(p0, p1, p2).max(second(p1, p2, p3));
                curve(&mut current, bend, flat, |t| {
                    let u = 1.0 - t;
                    let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
                    (
                        a * p0.0 + b * p1.0 + c * p2.0 + d * p3.0,
                        a * p0.1 + b * p1.1 + c * p2.1 + d * p3.1,
                    )
                });
            }
            SimplePathSegment::ClosePath => finish(&mut current, &mut out, true),
        }
    }
    finish(&mut current, &mut out, false);
    out
}

/// Points along a curve by `point(t)`, enough that no chord strays more than `flat` from
/// it: `bend` is the largest size of its second derivative.
fn curve(current: &mut Vec<Point>, bend: f64, flat: f64, point: impl Fn(f64) -> Point) {
    let n = (bend / (8.0 * flat)).sqrt().ceil().clamp(1.0, 1000.0) as usize;
    for k in 1..=n {
        current.push(point(k as f64 / n as f64));
    }
}

/// A stroke without repeated points (a closed one without its closing point); None when
/// nothing is left to cut.
fn cleaned(points: Vec<Point>, closed: bool) -> Option<Vec<Point>> {
    let mut out: Vec<Point> = Vec::with_capacity(points.len());
    for p in points {
        if out.last().is_none_or(|&q| distance(p, q) > CHAIN_TOL_MM) {
            out.push(p);
        }
    }
    if closed {
        while out.len() > 1
            && out
                .first()
                .zip(out.last())
                .is_some_and(|(&a, &b)| distance(a, b) <= CHAIN_TOL_MM)
        {
            out.pop();
        }
    }
    let enough = if closed { 3 } else { 2 };
    (out.len() >= enough).then_some(out)
}

/// One line: its characters with their pen positions, mm, and its base, mm.
struct Line {
    chars: Vec<(char, f64)>,
    baseline: f64,
    missing: Vec<char>,
    width: f64,
}

/// The characters of every line where they go, as the outline fonts lay theirs out:
/// along the line by advance, kerning and spacing; the lines `line_spacing` capital
/// heights apart and aligned against the widest.
fn laid_out(font: &SvgFont, layout: &Layout, scale: f64) -> Vec<Line> {
    let mut lines: Vec<Line> = Vec::new();
    for (i, text) in layout.text.split('\n').enumerate() {
        let mut line = Line {
            chars: Vec::new(),
            baseline: -(i as f64) * layout.line_spacing * layout.height,
            missing: Vec::new(),
            width: 0.0,
        };
        let mut pen = 0.0;
        let mut previous: Option<char> = None;
        for c in text.chars().filter(|c| !c.is_control()) {
            let Some(glyph) = font.glyphs.get(&c) else {
                line.missing.push(c);
                previous = None;
                continue;
            };
            if let Some(left) = previous {
                let k = font.kerning.get(&(left, c)).copied().unwrap_or(0.0);
                pen += -k * scale + layout.spacing;
            }
            line.chars.push((c, pen));
            pen += glyph.advance * scale;
            previous = Some(c);
        }
        line.width = pen;
        lines.push(line);
    }
    let widest = lines.iter().map(|l| l.width).fold(0.0, f64::max);
    for line in &mut lines {
        let shift = match layout.align {
            Align::Left => 0.0,
            Align::Center => (widest - line.width) / 2.0,
            Align::Right => widest - line.width,
        };
        for c in &mut line.chars {
            c.1 += shift;
        }
    }
    lines
}

/// The text's letters in the single-line font `data` (spec §4): open strokes (and the
/// closed ones a letter draws, such as an O), lines and arcs, the box round them at (0, 0).
pub(crate) fn letters(layout: &Layout, data: &[u8], name: &str) -> Result<Lettering, TextError> {
    let font = read(data, name)?;
    let scale = layout.height / font.cap_height;
    let lines = laid_out(&font, layout, scale);
    let mut missing: Vec<char> = Vec::new();
    for c in lines.iter().flat_map(|l| &l.missing) {
        if !missing.contains(c) {
            missing.push(*c);
        }
    }
    // Curves flattened to a quarter of the arc tolerance and fitted within the rest.
    let flat = ARC_APPROX_TOL_MM / 4.0;
    let advance = |c: char| font.glyphs.get(&c).map_or(0.0, |g| g.advance * scale);
    let mut places: Vec<(usize, f64)> = Vec::new();
    let mut spans: Vec<(f64, f64)> = Vec::new();
    let mut pieces: Vec<(usize, Vec<Primitive>, bool)> = Vec::new();
    let mut dropped = 0;
    let mut letter = 0;
    for (n, line) in lines.iter().enumerate() {
        let start = line.chars.first().map_or(0.0, |c| c.1);
        let end = line.chars.last().map_or(start, |&(c, x)| x + advance(c));
        spans.push((start, end));
        for &(c, x) in &line.chars {
            places.push((n, x + advance(c) / 2.0));
            let path = font.glyphs.get(&c).map_or("", |g| g.path.as_str());
            for (points, closed) in strokes(path, (x, line.baseline), scale, flat) {
                let Some(points) = cleaned(points, closed) else {
                    dropped += 1;
                    continue;
                };
                let fitted = fit_path(&points, closed, ARC_APPROX_TOL_MM - flat, true);
                if fitted.is_empty() {
                    dropped += 1;
                } else {
                    pieces.push((letter, fitted, closed));
                }
            }
            letter += 1;
        }
    }
    if pieces.is_empty() {
        return Err(TextError::NoLetters);
    }
    let (lo, hi) = crate::bounds(pieces.iter().flat_map(|(_, p, _)| p).flat_map(|p| {
        let (a, b) = p.bounds();
        [a, b]
    }));
    Ok(Lettering {
        outlines: pieces
            .iter()
            .map(|(_, p, _)| p.iter().map(|q| q.translated((-lo.0, -lo.1))).collect())
            .collect(),
        letter: pieces.iter().map(|(l, _, _)| *l).collect(),
        closed: pieces.iter().map(|(_, _, c)| *c).collect(),
        width: hi.0 - lo.0,
        height: hi.1 - lo.1,
        missing,
        dropped,
        baselines: lines.iter().map(|l| l.baseline - lo.1).collect(),
        places: places.into_iter().map(|(n, x)| (n, x - lo.0)).collect(),
        spans: spans
            .into_iter()
            .map(|(a, b)| (a - lo.0, b - lo.0))
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_small_font_reads_with_its_kerning_by_name_and_character() {
        let svg = br#"<svg><defs><font horiz-adv-x="500"><font-face units-per-em="1000" cap-height="700"/>
            <glyph glyph-name="A" unicode="A" horiz-adv-x="600" d="M0 0L300 700L600 0M100 200H500"/>
            <glyph glyph-name="V" unicode="V" d="M0 700L250 0L500 700"/>
            <glyph glyph-name="fi" unicode="fi" d="M0 0L0 700"/>
            <hkern g1="A" g2="V" k="80"/><hkern u1="V" u2="A" k="60"/>
            </font></defs></svg>"#;
        let font = read(svg, "test").unwrap();
        assert_eq!(font.cap_height, 700.0);
        assert_eq!(font.glyphs.len(), 2, "the ligature is left out");
        assert_eq!(font.glyphs[&'V'].advance, 500.0);
        assert_eq!(font.kerning[&('A', 'V')], 80.0);
        assert_eq!(font.kerning[&('V', 'A')], 60.0);
        let strokes = strokes(&font.glyphs[&'A'].path, (0.0, 0.0), 0.01, 0.0025);
        assert_eq!(strokes.len(), 2);
        assert!(strokes.iter().all(|(_, closed)| !closed));
        assert_eq!(strokes[1].0, vec![(1.0, 2.0), (5.0, 2.0)]);
    }

    #[test]
    fn what_isnt_a_font_is_refused() {
        for data in [
            &b"not xml"[..],
            b"<svg/>",
            b"<svg><font><font-face/></font></svg>",
        ] {
            assert!(matches!(read(data, "x"), Err(TextError::BadFont(_))));
        }
    }
}
