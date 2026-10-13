// SPDX-License-Identifier: GPL-3.0-or-later
//! Pictures traced into closed loops, for V-carving (docs/specs/image-trace.md).
//!
//! A picture's bytes go in, together with its settings and its width in mm; closed loops in
//! mm come out, black inside: outlines counter-clockwise, holes clockwise, with the
//! picture's lower-left corner at (0, 0). The black-and-white picture they trace is
//! available too, for previews. No file I/O: callers pass the bytes.

mod contour;
mod decode;
mod field;
mod simplify;

use chipload_core::Point;
use chipload_core::primitives::Primitive;

pub use decode::{Decoded, MAX_BYTES, MAX_PIXELS, decode};
pub use field::otsu;

/// More loops than this is refused: a photo or noise, not a carving (spec §8).
pub const MAX_LOOPS: usize = 5000;

/// Where grey turns black.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Threshold {
    /// Picked from the picture's histogram (Otsu's method).
    Auto,
    /// Grey values (0–255) below this are black; 1 to 254.
    Level(u8),
}

/// How the picture becomes black and white and how its edges are traced (spec §2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    pub threshold: Threshold,
    /// Carve the light parts instead of the dark ones.
    pub invert: bool,
    /// Gaussian blur before the threshold, in pixels (the standard deviation); 0: none.
    pub blur: f64,
    /// Loops enclosing less than this (mm²) are dropped.
    pub min_area: f64,
    /// How far a simplified edge may deviate from the traced one, mm.
    pub smoothing: f64,
    /// Fit arcs as well as lines (docs/specs/image-arcs-centrelines.md §2).
    pub arcs: bool,
}

impl Default for Settings {
    /// The spec's defaults (§2).
    fn default() -> Self {
        Settings {
            threshold: Threshold::Auto,
            invert: false,
            blur: 1.0,
            min_area: 0.5,
            smoothing: 0.05,
            arcs: true,
        }
    }
}

/// Settings limits (spec §2), checked by [`Settings::check`].
pub const MAX_BLUR_PX: f64 = 10.0;
pub const MAX_MIN_AREA_MM2: f64 = 100.0;
pub const MIN_SMOOTHING_MM: f64 = 0.01;
pub const MAX_SMOOTHING_MM: f64 = 1.0;

impl Settings {
    /// The first setting out of its range, as a message.
    pub fn check(&self) -> Result<(), ImageError> {
        let bad = |what: &str| Err(ImageError::Setting(what.into()));
        if let Threshold::Level(t) = self.threshold
            && !(1..=254).contains(&t)
        {
            return bad("the threshold must be 1 to 254, or auto");
        }
        if !(0.0..=MAX_BLUR_PX).contains(&self.blur) {
            return bad("the blur must be 0 to 10 px");
        }
        if !(0.0..=MAX_MIN_AREA_MM2).contains(&self.min_area) {
            return bad("the smallest piece must be 0 to 100 mm²");
        }
        if !(MIN_SMOOTHING_MM..=MAX_SMOOTHING_MM).contains(&self.smoothing) {
            return bad("the smoothing must be 0.01 to 1 mm");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum ImageError {
    #[error("the picture is {0} bytes; at most 20 MB is supported: scale it down")]
    TooLarge(usize),
    #[error("the picture is {0} × {1} pixels; at most 20 megapixels is supported: scale it down")]
    TooManyPixels(u32, u32),
    #[error("not a PNG, JPEG or BMP picture")]
    UnsupportedFormat,
    #[error("the picture can't be read: {0}")]
    Decode(String),
    #[error("the width must be more than 0 mm")]
    Width,
    #[error("{0}")]
    Setting(String),
    #[error(
        "the picture traces into {0} loops, more than {MAX_LOOPS}: blur it more, raise the \
         smallest piece, or change the threshold"
    )]
    TooManyLoops(usize),
}

/// A traced picture.
#[derive(Debug, Clone, PartialEq)]
pub struct Trace {
    /// Closed loops in mm, black inside: outlines counter-clockwise, holes clockwise. The
    /// first point is not repeated at the end. Arcs are split into chords within the
    /// chaining tolerance (for previews and checks; [`Trace::pieces`] are exact).
    pub loops: Vec<Vec<Point>>,
    /// The same loops as lines and arcs, each loop ending where it starts.
    pub pieces: Vec<Vec<Primitive>>,
    /// The threshold used (the one auto picked, or the one set).
    pub threshold: u8,
    /// Loops dropped as smaller than the smallest piece.
    pub dropped: usize,
    /// Loops whose arcs came too near another loop or crossed, kept as lines.
    pub without_arcs: usize,
    /// Loops left unsimplified because simplifying made them cross.
    pub unsimplified: usize,
    /// Black touches the picture's border: closed along it.
    pub black_at_border: bool,
    /// The picture's size in mm.
    pub width: f64,
    pub height: f64,
}

/// The black-and-white picture, row by row from the top: true is black.
#[derive(Debug, Clone, PartialEq)]
pub struct BlackWhite {
    pub width: u32,
    pub height: u32,
    pub black: Vec<bool>,
    pub threshold: u8,
}

/// The picture as black and white (spec §2): what [`trace`] traces.
pub fn black_white(picture: &Decoded, settings: &Settings) -> Result<BlackWhite, ImageError> {
    settings.check()?;
    let level = field::Level::new(picture, settings);
    Ok(BlackWhite {
        width: picture.width,
        height: picture.height,
        black: level.black(),
        threshold: level.threshold,
    })
}

/// A picture's loops as traced, before simplifying: about a point per pixel.
#[derive(Debug, Clone, PartialEq)]
pub struct RawTrace {
    /// As [`Trace::loops`], unsimplified, small pieces dropped.
    pub loops: Vec<Vec<Point>>,
    pub threshold: u8,
    pub dropped: usize,
    pub black_at_border: bool,
    pub width: f64,
    pub height: f64,
    /// The size of a pixel, mm.
    pub pixel: f64,
}

/// The picture's loops at `width` mm as traced (spec §3), small pieces dropped: what
/// [`trace`] simplifies, and what centre lines are found in.
pub fn trace_raw(
    picture: &Decoded,
    settings: &Settings,
    width: f64,
) -> Result<RawTrace, ImageError> {
    settings.check()?;
    if !(width.is_finite() && width > 0.0) {
        return Err(ImageError::Width);
    }
    let level = field::Level::new(picture, settings);
    let pixel = width / f64::from(picture.width);
    let raw = contour::loops(&level, pixel);
    let total = raw.len();
    let kept: Vec<Vec<Point>> = raw
        .into_iter()
        .filter(|l| chipload_core::primitives::signed_area(l).abs() >= settings.min_area)
        .collect();
    let dropped = total - kept.len();
    if kept.len() > MAX_LOOPS {
        return Err(ImageError::TooManyLoops(kept.len()));
    }
    Ok(RawTrace {
        loops: kept,
        threshold: level.threshold,
        dropped,
        black_at_border: level.black_at_border(),
        width,
        height: pixel * f64::from(picture.height),
        pixel,
    })
}

/// The picture traced at `width` mm (spec §3), simplified within the smoothing: lines, or
/// lines and arcs.
pub fn trace(picture: &Decoded, settings: &Settings, width: f64) -> Result<Trace, ImageError> {
    let raw = trace_raw(picture, settings, width)?;
    let fitted = simplify::fitted(raw.loops, settings.smoothing, raw.pixel, settings.arcs);
    Ok(Trace {
        loops: fitted.loops,
        pieces: fitted.pieces,
        threshold: raw.threshold,
        dropped: raw.dropped,
        without_arcs: fitted.without_arcs,
        unsimplified: fitted.unsimplified,
        black_at_border: raw.black_at_border,
        width,
        height: raw.height,
    })
}
