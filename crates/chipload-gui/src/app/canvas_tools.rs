// SPDX-License-Identifier: GPL-3.0-or-later
//! The canvas tools: drawing, editing, placing a text and measuring, one at a time; a value
//! being typed; the tools' previews; and the values they keep for the session.

use super::*;

/// The canvas tool in use (the review of 2026-11-02, point 3): one at a time, so starting
/// one ends any other, and Esc or the 3D view ends it. Each also drops a value being typed
/// and the offset preview.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) enum ActiveTool {
    /// No tool: clicks select.
    #[default]
    Select,
    /// A shape being drawn with the mouse (docs/specs/drawing-tools.md).
    Draw(Sketch),
    /// An editing tool (docs/specs/drawing-editing.md).
    Modify(modify::ModifyTool),
    /// Draw > Text: the next canvas click places a text (docs/specs/text.md §5).
    PlaceText,
    /// View → Measure (docs/specs/small-items.md §3).
    Measure(measure::Measure),
}

/// The canvas tools' state.
pub struct CanvasTools {
    /// The canvas tool in use: drawing, an editing tool, placing a text or measuring, at
    /// most one at a time.
    pub(super) active: ActiveTool,
    /// A value being typed while drawing (docs/specs/drawing-input.md §2).
    pub typed: Option<drawing::Typed>,
    /// The last drawing tool used: Enter with no tool active starts it again.
    pub last_tool: Option<Tool>,
    /// The number of sides new polygons get, as last chosen.
    pub polygon_sides: u32,
    /// The drawing tools draw construction geometry (key K; docs/specs/dxf-export.md §B2).
    pub construction_mode: bool,
    /// The offset tool's corners and count, kept for the session.
    pub offset_corners: chipload_pipeline::drawing_edit::Corners,
    pub offset_count: u32,
    /// The fillet tool's radius and the chamfer tool's distance, mm, kept for the session.
    pub fillet_radius: f64,
    pub chamfer_distance: f64,
    pub(super) offset_preview: modify::OffsetPreview,
    /// The array tools' values, kept for the session, and their preview.
    pub array: modify::ArraySettings,
    /// Align lines the selection up with the stock instead of its own box.
    pub align_to_stock: bool,
    pub(super) array_preview: modify::ArrayPreview,
    /// Draw > Frame…'s values, kept for the session, and its preview.
    pub frame: frames::FrameSettings,
    pub(super) frame_preview: frames::FramePreview,
}

impl Default for CanvasTools {
    fn default() -> Self {
        CanvasTools {
            active: ActiveTool::Select,
            typed: None,
            last_tool: None,
            polygon_sides: crate::draw::POLYGON_SIDES_DEFAULT,
            construction_mode: false,
            offset_corners: chipload_pipeline::drawing_edit::Corners::Round,
            offset_count: 1,
            fillet_radius: 3.0,
            chamfer_distance: 1.0,
            offset_preview: modify::OffsetPreview::default(),
            array: modify::ArraySettings::default(),
            align_to_stock: false,
            array_preview: modify::ArrayPreview::default(),
            frame: frames::FrameSettings::default(),
            frame_preview: frames::FramePreview::default(),
        }
    }
}

impl ChiploadApp {
    /// Use `tool` on the canvas, ending any other. A value being typed and the offset
    /// preview belong to the tool they were made with, so they go.
    pub(super) fn set_tool(&mut self, tool: ActiveTool) {
        self.canvas_tools.typed = None;
        self.canvas_tools.offset_preview = modify::OffsetPreview::default();
        self.canvas_tools.active = tool;
    }

    /// End the canvas tool in use, if any: clicks select again.
    pub(super) fn end_tool(&mut self) {
        self.set_tool(ActiveTool::Select);
    }

    /// Whether a canvas tool is in use.
    pub fn tool_active(&self) -> bool {
        self.canvas_tools.active != ActiveTool::Select
    }

    /// The shape being drawn, while drawing.
    pub fn sketch(&self) -> Option<&Sketch> {
        match &self.canvas_tools.active {
            ActiveTool::Draw(sketch) => Some(sketch),
            _ => None,
        }
    }

    pub(super) fn sketch_mut(&mut self) -> Option<&mut Sketch> {
        match &mut self.canvas_tools.active {
            ActiveTool::Draw(sketch) => Some(sketch),
            _ => None,
        }
    }

    /// The editing tool in use.
    pub fn modify(&self) -> Option<modify::ModifyTool> {
        match self.canvas_tools.active {
            ActiveTool::Modify(tool) => Some(tool),
            _ => None,
        }
    }

    /// The measurement being made, while measuring.
    pub fn measure(&self) -> Option<&measure::Measure> {
        match &self.canvas_tools.active {
            ActiveTool::Measure(measure) => Some(measure),
            _ => None,
        }
    }

    pub(super) fn measure_mut(&mut self) -> Option<&mut measure::Measure> {
        match &mut self.canvas_tools.active {
            ActiveTool::Measure(measure) => Some(measure),
            _ => None,
        }
    }
}
