// SPDX-License-Identifier: GPL-3.0-or-later
//! What the canvas shows: top view or 3D, Edit or Simulate, the pictures, the toolpath,
//! a part alone or the active setup, the legend.

/// What the canvas shows.
pub struct Display {
    /// Simulate mode (docs/specs/edit-and-simulate.md §2): the shaded simulation and
    /// playback, the canvas a viewer. Off: Edit, the canvas a drawing board.
    pub simulate: bool,
    /// The canvas is in 3D: left-drag orbits (docs/specs/3d-view.md).
    pub three_d: bool,
    /// The canvas legend is unfolded.
    pub legend_open: bool,
    /// The part shown alone, once, where it is drawn (docs/specs/parts-and-setups.md §8a);
    /// None: the active setup, every instance on its board. Follows what is selected in
    /// the Browser; this session only.
    pub part: Option<u64>,
    /// The focus `part` last followed (`ChiploadApp::follow_focus`).
    pub followed: Option<super::Focus>,
    /// The space shown (docs/specs/drawing-and-setup-spaces.md §2), as last picked; what is
    /// selected picks it again (`ChiploadApp::space`).
    pub space: Space,
    /// Each space's view, scale and offset, kept while another is shown: the drawing's
    /// under None, a setup's under its id.
    pub views: std::collections::BTreeMap<Option<u64>, (f64, (f64, f64))>,
    /// The setup's tab a part dragged from the Browser is held over, and since when
    /// (seconds, egui's clock): held long enough, it opens.
    pub tab_held: Option<(u64, f64)>,
    /// View > Show images: the pictures, faintly over the stock and under the drawing.
    pub images: bool,
    /// View → Show toolpath (docs/specs/drawing-sources.md §5).
    pub toolpath: bool,
    /// The board is shown turned over, back face up (docs/specs/two-sided.md §10.2); only
    /// for a setup with a back or that is one.
    pub turned: bool,
    /// View > Show the turn axis: the line a two-sided board turns about
    /// (docs/specs/two-sided-wizard.md §2).
    pub turn_axis: bool,
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
            turn_axis: true,
            part: None,
            followed: None,
            space: Space::Drawing,
            views: std::collections::BTreeMap::new(),
            tab_held: None,
            legend_open: true,
            other: None,
        }
    }
}

/// The two kinds of space the canvas shows (docs/specs/drawing-and-setup-spaces.md §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Space {
    /// Every part's shapes where drawn, to draw and edit; no instances.
    #[default]
    Drawing,
    /// The active setup's board and its instances; no drawn shapes.
    Setup,
}
