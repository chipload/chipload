// SPDX-License-Identifier: GPL-3.0-or-later
//! Nesting (docs/specs/nesting.md): what each instance cuts, as a footprint on a raster of
//! square cells, for a bottom-left fill to place on a board.
//!
//! Step 1 (§11): the swept cut of a planned part ([`Sweep`]), turned as an instance is;
//! its footprint rasterised so that every cell it touches is occupied and its holes are
//! filled ([`Footprint`]); the footprint grown by the gap; keep-out areas on a board grid.
//! Step 2: bottom-left fill of the usable area ([`Sheet`], [`fill`]), several orders tried,
//! the board's edges checked in mm.
//! The raster representation follows Bennell & Oliveira, *The geometry of nesting
//! problems: A tutorial*, European Journal of Operational Research 184 (2008).

mod fill;
mod footprint;
mod grid;
mod sweep;

pub use fill::{Goal, Item, Layout, Lie, Put, Sheet, fill};
pub use footprint::{Dilated, Footprint, Frame};
pub use grid::Grid;
pub use sweep::{Capsule, Sweep};

/// The raster's cell side, mm, unless the board would need more than [`MAX_CELLS`]
/// (nesting.md §4.1).
pub const CELL_MM: f64 = 0.5;

/// The most cells a board's raster may have: a 1.4 × 1.4 m board at [`CELL_MM`].
pub const MAX_CELLS: f64 = 8_000_000.0;

/// What a larger cell is rounded up to, mm, so the side stays a tidy number.
const CELL_STEP_MM: f64 = 0.05;

/// The cell side for a usable area of `width` × `height` mm: [`CELL_MM`], or larger (in
/// steps of 0.05 mm) so the area needs no more than [`MAX_CELLS`] cells.
pub fn cell_size(width: f64, height: f64) -> f64 {
    let area = width.max(0.0) * height.max(0.0);
    let cells = |side: f64| (width / side).ceil().max(1.0) * (height / side).ceil().max(1.0);
    if !area.is_finite() || cells(CELL_MM) <= MAX_CELLS {
        return CELL_MM;
    }
    let mut side = ((area / MAX_CELLS).sqrt() / CELL_STEP_MM).ceil() * CELL_STEP_MM;
    // The ceilings of each side's count may still tip it over: one step more at a time.
    while cells(side) > MAX_CELLS {
        side += CELL_STEP_MM;
    }
    side
}

#[cfg(test)]
mod tests;
