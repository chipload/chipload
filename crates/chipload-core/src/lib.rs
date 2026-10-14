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

/// A 2D point in millimetres.
pub type Point = (f64, f64);
