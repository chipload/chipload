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
    /// The board is shown turned over, back face up (docs/specs/two-sided.md §10.2); only
    /// for a setup with a back or that is one.
    pub turned: bool,
    /// The other mode's view, put back when it is switched to (the user, 2026-11-03: "the
    /// view settings to be remembered between Edit mode and Simulation mode"). None until
    /// the mode is first left: the other mode then starts from this one's.
    pub other: Option<ModeView>,
}

/// What one mode shows: top or 3D, the toolpath, the drawing (Simulate only: Edit always
/// shows it), the pictures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModeView {
    pub three_d: bool,
    pub toolpath: bool,
    pub drawing: bool,
    pub images: bool,
}

impl Default for Display {
    fn default() -> Self {
        Display {
            three_d: false,
            simulate: false,
            images: true,
            toolpath: true,
            turned: false,
            all_parts: false,
            legend_open: true,
            other: None,
        }
    }
}
