// SPDX-License-Identifier: GPL-3.0-or-later
//! Drawing geometry on `chipload-core`'s primitives (ADR 0023): where segments cross, the
//! points the cursor snaps to, and later the editing operations on drawn shapes. No I/O and
//! no project model; `chipload-pipeline` applies results to a project.
//! See docs/specs/drawing-roadmap.md and drawing-input.md.

pub mod construct;
pub mod corner;
pub mod intersect;
pub mod offset;
pub mod select;
pub mod snap;
