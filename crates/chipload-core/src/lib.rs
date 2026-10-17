// SPDX-License-Identifier: GPL-3.0-or-later
//! Shared foundations for Chipload: tolerances and basic geometry types.
//!
//! Headless by design: no I/O, no GUI. Units are millimetres, angles radians (CCW positive).

pub mod chain;
pub mod fit;
pub mod fold;
pub mod nesting;
pub mod order;
pub mod primitives;
pub mod text;
pub mod tolerances;

/// Millimetres in an inch, exactly. Only for converting at the edges (DXF import, inch
/// G-code, inch display).
pub const MM_PER_INCH: f64 = 25.4;

/// A 2D point in millimetres.
pub type Point = (f64, f64);
