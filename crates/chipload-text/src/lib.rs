// SPDX-License-Identifier: GPL-3.0-or-later
//! Text into closed outlines, for V-carving, pocketing and profiling (docs/specs/text.md).
//!
//! What the text says, its font and its size go in; closed outlines of lines and arcs in mm
//! come out, with the box round all of them at (0, 0). The fonts are built in (GNU
//! FreeFont and ten Apache-2.0 fonts, cut down to Latin: fonts/LICENSE*.txt), so every build
//! makes the same letters; a font file's bytes can be given too, for the computer's fonts
//! (docs/specs/system-fonts.md §4). No file I/O.

use chipload_core::Point;
use chipload_core::fit::fit_path;
use chipload_core::primitives::{Primitive, distance};
use chipload_core::tolerances::{ARC_APPROX_TOL_MM, CHAIN_TOL_MM};
use chipload_i18n::{Digits, Message, Value};
use ttf_parser::gpos::{PairAdjustment, PositioningSubtable};
use ttf_parser::{Face, GlyphId, OutlineBuilder, Tag};

/// A built-in font: its name, as a project names it, its style and the font file.
pub struct BuiltinFont {
    pub name: &'static str,
    pub style: Style,
    data: &'static [u8],
}

impl BuiltinFont {
    /// The font file (TrueType), for showing its name in its own face.
    pub fn data(&self) -> &'static [u8] {
        self.data
    }
}

/// What a font is for, as the app groups them (docs/specs/more-fonts.md §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    /// Plain letters for any sign or label.
    Plain,
    /// Connected handwriting.
    Script,
    /// Bold and decorative, for headings.
    Display,
}

impl Style {
    pub fn name(self) -> &'static str {
        match self {
            Style::Plain => "plain",
            Style::Script => "script",
            Style::Display => "display",
        }
    }
}

/// The built-in fonts (text.md §2, more-fonts.md §2), the default first: GNU FreeFont
/// (fonts/LICENSE.txt) and ten Apache-2.0 fonts from Google Fonts
/// (fonts/LICENSE-Apache-2.0.txt), all cut down to Latin.
pub const FONTS: &[BuiltinFont] = &[
    BuiltinFont {
        name: "FreeSans",
        style: Style::Plain,
        data: include_bytes!("../fonts/FreeSans.ttf"),
    },
    BuiltinFont {
        name: "FreeSans Bold",
        style: Style::Plain,
        data: include_bytes!("../fonts/FreeSansBold.ttf"),
    },
    BuiltinFont {
        name: "FreeSerif",
        style: Style::Plain,
        data: include_bytes!("../fonts/FreeSerif.ttf"),
    },
    BuiltinFont {
        name: "FreeSerif Bold",
        style: Style::Plain,
        data: include_bytes!("../fonts/FreeSerifBold.ttf"),
    },
    BuiltinFont {
        name: "FreeMono",
        style: Style::Plain,
        data: include_bytes!("../fonts/FreeMono.ttf"),
    },
    BuiltinFont {
        name: "FreeMono Bold",
        style: Style::Plain,
        data: include_bytes!("../fonts/FreeMonoBold.ttf"),
    },
    BuiltinFont {
        name: "Roboto Slab",
        style: Style::Plain,
        data: include_bytes!("../fonts/RobotoSlab.ttf"),
    },
    BuiltinFont {
        name: "Roboto Slab Bold",
        style: Style::Plain,
        data: include_bytes!("../fonts/RobotoSlabBold.ttf"),
    },
    BuiltinFont {
        name: "Satisfy",
        style: Style::Script,
        data: include_bytes!("../fonts/Satisfy.ttf"),
    },
    BuiltinFont {
        name: "Yellowtail",
        style: Style::Script,
        data: include_bytes!("../fonts/Yellowtail.ttf"),
    },
    BuiltinFont {
        name: "Rochester",
        style: Style::Script,
        data: include_bytes!("../fonts/Rochester.ttf"),
    },
    BuiltinFont {
        name: "Luckiest Guy",
        style: Style::Display,
        data: include_bytes!("../fonts/LuckiestGuy.ttf"),
    },
    BuiltinFont {
        name: "Ultra",
        style: Style::Display,
        data: include_bytes!("../fonts/Ultra.ttf"),
    },
    BuiltinFont {
        name: "Chewy",
        style: Style::Display,
        data: include_bytes!("../fonts/Chewy.ttf"),
    },
    BuiltinFont {
        name: "Smokum",
        style: Style::Display,
        data: include_bytes!("../fonts/Smokum.ttf"),
    },
    BuiltinFont {
        name: "Cherry Cream Soda",
        style: Style::Display,
        data: include_bytes!("../fonts/CherryCreamSoda.ttf"),
    },
    BuiltinFont {
        name: "Permanent Marker",
        style: Style::Display,
        data: include_bytes!("../fonts/PermanentMarker.ttf"),
    },
];

/// The font a new text gets.
pub const DEFAULT_FONT: &str = "FreeSans";

/// More characters than this is refused: a text, not a page.
pub const MAX_CHARS: usize = 2000;

/// The built-in fonts' names.
pub fn font_names() -> impl Iterator<Item = &'static str> {
    FONTS.iter().map(|f| f.name)
}

/// The built-in font named `name`.
pub fn builtin(name: &str) -> Option<&'static BuiltinFont> {
    FONTS.iter().find(|f| f.name == name)
}

/// A font file's bytes and the face in it (a collection holds several), with the name a
/// project gives it.
#[derive(Debug, Clone, Copy)]
pub struct FontFile<'a> {
    pub name: &'a str,
    pub data: &'a [u8],
    pub index: u32,
}

/// The name a project gives face `index` of a font file, if letters can be made in it: its
/// full name ("Arial Bold"; else its family and style), when it has outlines and a capital
/// height (docs/specs/system-fonts.md §2).
pub fn face_name(data: &[u8], index: u32) -> Option<String> {
    let face = Face::parse(data, index).ok()?;
    capital_height(&face)?;
    let h = face.glyph_index('H')?;
    face.glyph_bounding_box(h)?;
    let english = |id: u16| {
        face.names()
            .into_iter()
            .filter(|n| n.name_id == id && n.is_unicode())
            .find(|n| n.language() == ttf_parser::Language::English_UnitedStates)
            .or_else(|| {
                face.names()
                    .into_iter()
                    .find(|n| n.name_id == id && n.is_unicode())
            })
            .and_then(|n| n.to_string())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };
    english(ttf_parser::name_id::FULL_NAME).or_else(|| {
        let family = english(ttf_parser::name_id::FAMILY)?;
        Some(match english(ttf_parser::name_id::SUBFAMILY) {
            Some(style) if style != "Regular" => format!("{family} {style}"),
            _ => family,
        })
    })
}

/// How the lines of a text line up against each other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

/// What to write and how.
#[derive(Debug, Clone, Copy)]
pub struct Layout<'a> {
    /// What it says; `\n` starts a new line.
    pub text: &'a str,
    /// A built-in font's name.
    pub font: &'a str,
    /// The height of a capital letter, mm.
    pub height: f64,
    /// Extra space between letters, mm.
    pub spacing: f64,
    /// From one line's base to the next, in capital heights.
    pub line_spacing: f64,
    pub align: Align,
}

/// A text's letters.
#[derive(Debug, Clone, PartialEq)]
pub struct Lettering {
    /// Closed outlines, lines and arcs end to end, each ending where it starts. The box
    /// round all of them has its lower-left corner at (0, 0).
    pub outlines: Vec<Vec<Primitive>>,
    /// For each outline, the letter it belongs to (its index in the text's letters), so a
    /// hole can be told from the letter round it when letters overlap
    /// (docs/specs/more-fonts.md §4).
    pub letter: Vec<usize>,
    /// The box's size, mm.
    pub width: f64,
    pub height: f64,
    /// Characters the font has no letter for, left out (each once).
    pub missing: Vec<char>,
    /// Outlines too small to keep (a speck of a glyph).
    pub dropped: usize,
}

/// Why a text can't be turned into letters. Its text is a message (`text.ftl`), shown in
/// the app's language; `Display` writes it in English.
#[derive(Debug, Clone, PartialEq)]
pub enum TextError {
    /// The font asked for, and the built-in fonts' names.
    UnknownFont(String, String),
    Empty,
    /// How many characters the text has.
    TooLong(usize),
    /// The height asked for, mm.
    Height(f64),
    LineSpacing(f64),
    Spacing(f64),
    /// The font's name.
    BadFont(String),
    NoLetters,
}

impl TextError {
    /// The error as a message.
    pub fn message(&self) -> Message {
        match self {
            TextError::UnknownFont(font, fonts) => Message::new("text-error-unknown-font")
                .arg("font", font)
                .arg("fonts", fonts),
            TextError::Empty => Message::new("text-error-empty"),
            TextError::TooLong(count) => Message::new("text-error-too-long")
                .arg("count", *count)
                .arg("most", MAX_CHARS),
            TextError::Height(got) => Message::new("text-error-height")
                .arg("zero", Value::mm(0.0))
                .arg("got", Value::LengthNumber(*got, Digits::Shortest)),
            TextError::LineSpacing(got) => Message::new("text-error-line-spacing")
                .arg("got", Value::Number(*got, Digits::Shortest)),
            TextError::Spacing(got) => {
                Message::new("text-error-spacing").arg("got", Value::Number(*got, Digits::Shortest))
            }
            TextError::BadFont(font) => Message::new("text-error-bad-font").arg("font", font),
            TextError::NoLetters => Message::new("text-error-no-letters"),
        }
    }
}

impl std::fmt::Display for TextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message().english())
    }
}

impl std::error::Error for TextError {}

/// The text's letters as closed outlines (spec §4).
/// In a built-in font, named by `layout.font`.
pub fn letters(layout: &Layout) -> Result<Lettering, TextError> {
    let font = builtin(layout.font).ok_or_else(|| {
        TextError::UnknownFont(
            layout.font.to_string(),
            font_names().collect::<Vec<_>>().join(", "),
        )
    })?;
    letters_in(
        layout,
        FontFile {
            name: font.name,
            data: font.data,
            index: 0,
        },
    )
}

/// The text's letters in a font file's face (`layout.font` is not looked at).
pub fn letters_in(layout: &Layout, font: FontFile) -> Result<Lettering, TextError> {
    check(layout)?;
    let face = Face::parse(font.data, font.index)
        .map_err(|_| TextError::BadFont(font.name.to_string()))?;
    let cap = capital_height(&face).ok_or_else(|| TextError::BadFont(font.name.to_string()))?;
    let scale = layout.height / cap;
    let lines = laid_out(&face, layout, scale);
    let mut missing: Vec<char> = Vec::new();
    for line in &lines {
        for &c in &line.missing {
            if !missing.contains(&c) {
                missing.push(c);
            }
        }
    }
    // Curves are flattened to a quarter of the arc tolerance and fitted within the rest, so
    // the result is within the arc tolerance of the font's curves.
    let flat = ARC_APPROX_TOL_MM / 4.0;
    let mut contours: Vec<(usize, Vec<Point>)> = Vec::new();
    let mut letter = 0;
    for line in &lines {
        for &(glyph, x) in &line.glyphs {
            let mut pen = Pen::new((x, line.baseline), scale, flat);
            face.outline_glyph(glyph, &mut pen);
            contours.extend(pen.finish().into_iter().map(|c| (letter, c)));
            letter += 1;
        }
    }
    let mut dropped = 0;
    let mut kept: Vec<(usize, Vec<Point>)> = Vec::new();
    for (letter, contour) in contours {
        match cleaned(contour) {
            Some(points) => kept.push((letter, points)),
            None => dropped += 1,
        }
    }
    if kept.is_empty() {
        return Err(TextError::NoLetters);
    }
    let mut fitted: Vec<Vec<Primitive>> = Vec::with_capacity(kept.len());
    let mut letters_of: Vec<usize> = Vec::with_capacity(kept.len());
    for (letter, points) in kept {
        let pieces = fit_path(&points, true, ARC_APPROX_TOL_MM - flat, true);
        if pieces.is_empty() {
            dropped += 1;
        } else {
            fitted.push(pieces);
            letters_of.push(letter);
        }
    }
    if fitted.is_empty() {
        return Err(TextError::NoLetters);
    }
    // The box round the lines and arcs themselves (an arc may bulge a little past the
    // points it was fitted to), moved to (0, 0).
    let (lo, hi) = bounds(fitted.iter().flatten().flat_map(|p| {
        let (a, b) = p.bounds();
        [a, b]
    }));
    let outlines: Vec<Vec<Primitive>> = fitted
        .iter()
        .map(|o| o.iter().map(|p| p.translated((-lo.0, -lo.1))).collect())
        .collect();
    Ok(Lettering {
        outlines,
        letter: letters_of,
        width: hi.0 - lo.0,
        height: hi.1 - lo.1,
        missing,
        dropped,
    })
}

fn check(layout: &Layout) -> Result<(), TextError> {
    if layout.text.trim().is_empty() {
        return Err(TextError::Empty);
    }
    let count = layout.text.chars().count();
    if count > MAX_CHARS {
        return Err(TextError::TooLong(count));
    }
    if !(layout.height.is_finite() && layout.height > 0.0) {
        return Err(TextError::Height(layout.height));
    }
    if !(layout.line_spacing.is_finite() && layout.line_spacing > 0.0) {
        return Err(TextError::LineSpacing(layout.line_spacing));
    }
    if !layout.spacing.is_finite() {
        return Err(TextError::Spacing(layout.spacing));
    }
    Ok(())
}

/// The height of a capital letter in font units: the font's own figure, else the H's top.
fn capital_height(face: &Face) -> Option<f64> {
    let measured = face
        .glyph_index('H')
        .and_then(|h| face.glyph_bounding_box(h))
        .map(|rect| f64::from(rect.y_max))
        .filter(|&y| y > 0.0);
    let stated = face.capital_height().filter(|&c| c > 0).map(f64::from);
    match (stated, measured) {
        // A font's stated height is used unless its own H says it is wrong (Rochester
        // states 326 units for an H of 1741, docs/specs/more-fonts.md §2).
        (Some(cap), Some(h)) if (cap - h).abs() > CAP_HEIGHT_TRUST * h => Some(h),
        (Some(cap), _) => Some(cap),
        (None, h) => h,
    }
}

/// How far a font's stated capital height may differ from its H's height, as a fraction,
/// before the H's is used.
const CAP_HEIGHT_TRUST: f64 = 0.1;

/// One line: its glyphs with their pen positions in mm, and its base, mm.
struct Line {
    glyphs: Vec<(GlyphId, f64)>,
    baseline: f64,
    missing: Vec<char>,
}

/// The glyphs of every line where they go: along the line by advance, kerning and spacing;
/// the lines `line_spacing` capital heights apart and aligned against the widest.
fn laid_out(face: &Face, layout: &Layout, scale: f64) -> Vec<Line> {
    let mut lines: Vec<(Line, f64)> = Vec::new();
    for (i, text) in layout.text.split('\n').enumerate() {
        let mut line = Line {
            glyphs: Vec::new(),
            baseline: -(i as f64) * layout.line_spacing * layout.height,
            missing: Vec::new(),
        };
        let mut pen = 0.0;
        let mut previous: Option<GlyphId> = None;
        for c in text.chars().filter(|c| !c.is_control()) {
            let Some(glyph) = face.glyph_index(c) else {
                line.missing.push(c);
                previous = None;
                continue;
            };
            if let Some(left) = previous {
                pen += f64::from(kerning(face, left, glyph)) * scale + layout.spacing;
            }
            line.glyphs.push((glyph, pen));
            pen += f64::from(face.glyph_hor_advance(glyph).unwrap_or(0)) * scale;
            previous = Some(glyph);
        }
        lines.push((line, pen));
    }
    let widest = lines.iter().map(|(_, w)| *w).fold(0.0, f64::max);
    lines
        .into_iter()
        .map(|(mut line, width)| {
            let shift = match layout.align {
                Align::Left => 0.0,
                Align::Center => (widest - width) / 2.0,
                Align::Right => widest - width,
            };
            for glyph in &mut line.glyphs {
                glyph.1 += shift;
            }
            line
        })
        .collect()
}

/// Kerning between two glyphs, font units: the OpenType `kern` feature, else the old
/// `kern` table, else none.
fn kerning(face: &Face, left: GlyphId, right: GlyphId) -> i16 {
    layout_kerning(face, left, right)
        .or_else(|| table_kerning(face, left, right))
        .unwrap_or(0)
}

fn layout_kerning(face: &Face, left: GlyphId, right: GlyphId) -> Option<i16> {
    let gpos = face.tables().gpos?;
    let feature = gpos.features.find(Tag::from_bytes(b"kern"))?;
    for index in feature.lookup_indices {
        let Some(lookup) = gpos.lookups.get(index) else {
            continue;
        };
        for subtable in lookup.subtables.into_iter::<PositioningSubtable>() {
            if let PositioningSubtable::Pair(pair) = subtable
                && let Some(x) = pair_advance(&pair, left, right)
            {
                return Some(x);
            }
        }
    }
    None
}

fn pair_advance(pair: &PairAdjustment, left: GlyphId, right: GlyphId) -> Option<i16> {
    match pair {
        PairAdjustment::Format1 { coverage, sets } => {
            let set = sets.get(coverage.get(left)?)?;
            Some(set.get(right)?.0.x_advance)
        }
        PairAdjustment::Format2 {
            coverage,
            classes,
            matrix,
        } => {
            coverage.get(left)?;
            let pair = (classes.0.get(left), classes.1.get(right));
            Some(matrix.get(pair)?.0.x_advance)
        }
    }
}

fn table_kerning(face: &Face, left: GlyphId, right: GlyphId) -> Option<i16> {
    let kern = face.tables().kern?;
    kern.subtables
        .into_iter()
        .filter(|s| s.horizontal && !s.variable && !s.has_cross_stream)
        .find_map(|s| s.glyphs_kerning(left, right))
}

/// Collects a glyph's contours in mm, its curves flattened to `flat`.
struct Pen {
    origin: Point,
    scale: f64,
    flat: f64,
    contours: Vec<Vec<Point>>,
    current: Vec<Point>,
}

impl Pen {
    fn new(origin: Point, scale: f64, flat: f64) -> Self {
        Pen {
            origin,
            scale,
            flat,
            contours: Vec::new(),
            current: Vec::new(),
        }
    }

    fn at(&self, x: f32, y: f32) -> Point {
        (
            self.origin.0 + f64::from(x) * self.scale,
            self.origin.1 + f64::from(y) * self.scale,
        )
    }

    fn last(&self) -> Point {
        self.current.last().copied().unwrap_or(self.origin)
    }

    /// Points along a curve by `point(t)`, enough that no chord is farther than `flat` from
    /// it: `bend` is the largest size of its second derivative.
    fn curve(&mut self, bend: f64, point: impl Fn(f64) -> Point) {
        let n = (bend / (8.0 * self.flat)).sqrt().ceil().clamp(1.0, 1000.0) as usize;
        for k in 1..=n {
            self.current.push(point(k as f64 / n as f64));
        }
    }

    fn finish(mut self) -> Vec<Vec<Point>> {
        self.close();
        self.contours
    }
}

impl OutlineBuilder for Pen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.close();
        let p = self.at(x, y);
        self.current.push(p);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let p = self.at(x, y);
        self.current.push(p);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (p0, p1, p2) = (self.last(), self.at(x1, y1), self.at(x, y));
        let bend = 2.0 * distance((p0.0 - p1.0, p0.1 - p1.1), (p1.0 - p2.0, p1.1 - p2.1));
        self.curve(bend, |t| {
            let u = 1.0 - t;
            (
                u * u * p0.0 + 2.0 * u * t * p1.0 + t * t * p2.0,
                u * u * p0.1 + 2.0 * u * t * p1.1 + t * t * p2.1,
            )
        });
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (p0, p1, p2, p3) = (self.last(), self.at(x1, y1), self.at(x2, y2), self.at(x, y));
        let second =
            |a: Point, b: Point, c: Point| distance((a.0 - b.0, a.1 - b.1), (b.0 - c.0, b.1 - c.1));
        let bend = 6.0 * second(p0, p1, p2).max(second(p1, p2, p3));
        self.curve(bend, |t| {
            let u = 1.0 - t;
            let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
            (
                a * p0.0 + b * p1.0 + c * p2.0 + d * p3.0,
                a * p0.1 + b * p1.1 + c * p2.1 + d * p3.1,
            )
        });
    }

    fn close(&mut self) {
        if !self.current.is_empty() {
            self.contours.push(std::mem::take(&mut self.current));
        }
    }
}

/// A contour without repeated points and without its closing point, or None if too small
/// to cut (fewer than three points apart, or no area).
fn cleaned(points: Vec<Point>) -> Option<Vec<Point>> {
    let mut out: Vec<Point> = Vec::with_capacity(points.len());
    for p in points {
        if out.last().is_none_or(|&q| distance(p, q) > CHAIN_TOL_MM) {
            out.push(p);
        }
    }
    while out.len() > 1
        && out
            .first()
            .zip(out.last())
            .is_some_and(|(&a, &b)| distance(a, b) <= CHAIN_TOL_MM)
    {
        out.pop();
    }
    let area = out
        .iter()
        .zip(out.iter().cycle().skip(1))
        .map(|(a, b)| a.0 * b.1 - b.0 * a.1)
        .sum::<f64>()
        .abs()
        / 2.0;
    (out.len() >= 3 && area > CHAIN_TOL_MM * CHAIN_TOL_MM).then_some(out)
}

fn bounds(points: impl Iterator<Item = Point>) -> (Point, Point) {
    points.fold(
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
    )
}
