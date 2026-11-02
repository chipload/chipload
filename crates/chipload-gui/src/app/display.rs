// SPDX-License-Identifier: GPL-3.0-or-later
//! What the canvas shows: top view or 3D, Edit or Simulate, the pictures, the toolpath,
//! every part or the active one, the legend.

/// What the canvas shows.
pub struct Display {
    /// Simulate mode (docs/specs/edit-and-simulate.md §2): the shaded simulation and
    /// playback, the canvas a viewer. Off: Edit, the canvas a drawing board.
    pub simulate: bool,
    /// The canvas is in 3D: left-drag orbits (docs/specs/3d-view.md).
    pub three_d: bool,
    /// The canvas legend is unfolded.
    pub legend_open: bool,
    /// Preview every part, not only the active one (canvas bar; this session only).
    pub all_parts: bool,
    /// View > Show images: the pictures, faintly over the stock and under the drawing.
    pub images: bool,
    /// View → Show toolpath (docs/specs/drawing-sources.md §5).
    pub toolpath: bool,
}

impl Default for Display {
    fn default() -> Self {
        Display {
            three_d: false,
            simulate: false,
            images: true,
            toolpath: true,
            all_parts: false,
            legend_open: true,
        }
    }
}
