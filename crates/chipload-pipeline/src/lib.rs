// SPDX-License-Identifier: GPL-3.0-or-later
//! End to end: job + DXF -> checked G-code. The CLI and the GUI both call this.

pub mod box_handles;
pub mod chip_load;
pub mod copies;
pub mod corners;
pub mod dowels;
pub mod drawing_edit;
pub mod dxfs;
mod error;
pub mod estimate;
pub mod explode;
pub mod export_all;
pub mod fonts;
pub mod frames;
pub mod groups;
pub mod images;
pub mod inlay;
pub mod instances;
pub mod library;
pub mod materials;
pub mod models;
pub mod nest;
pub mod new_ops;
pub mod notices;
pub mod part_stock;
pub mod part_view;
pub mod parts;
mod passes;
pub mod pieces;
pub mod placement;
pub mod posts;
mod problems;
mod project;
pub mod recovery;
pub mod relief_wizard;
pub mod report;
mod run;
pub mod selection;
pub mod setups;
pub mod shape_library;
pub mod shapes;
pub mod sheet;
pub mod sign;
pub mod simulate;
pub mod styles;
pub mod surface;
pub mod tabs;
pub mod texts;
mod texts_arc;
pub mod two_sided;
pub mod units;

use std::path::Path;

use chipload_core::chain::Contour;
use chipload_i18n::Message;
use chipload_job::{Job, Operation, Tool, ToolType};
pub use chipload_toolpath::Cancel;

pub use error::PipelineError;
pub(crate) use passes::no_operation;
pub use passes::{
    DEFAULT_ROUGHING_ALLOWANCE_MM, SMOOTHING_END_MILL_STEPOVER, SMOOTHING_RIDGE_MM, add_floor_pass,
    add_rest_pass, add_roughing_pass, add_smoothing_pass, fitting_end_mills, floor_pass_of,
    make_rest_pass, rest_pass_of, rest_pocket_parent, roughing_of, set_op_rest, set_op_tool,
    smoothing_of, vcarve_pass_depths,
};
pub use problems::{foreign_text, job_problems, joined, refuse_problems};
pub use project::{
    DEFAULT_EMPTY_STOCK, DEFAULT_MAX_OVERCUT, DEFAULT_RPM, DEFAULT_SAFE_Z, DEFAULT_STEP_DOWN,
    DEFAULT_STOCK_MARGIN, DEFAULT_THICKNESS, DEFAULT_TOOL_DIAMETER, UpdateReport, drawing_sha256,
    empty_project, load_drawing, new_project, stock_fit_message, stock_fit_note, update_drawing,
    update_drawing_with, with_inch_defaults,
};
pub use run::{
    CheckFindings, CutInputs, GcodeFile, RunResult, SetupRun, cut_inputs, front_board_z,
    in_work_coordinates, on_the_board, post_and_check, run_job, run_job_cancellable, run_setups,
    run_setups_pieces, shown_on_board, write_gcode_files,
};

pub(crate) fn op_label(index: usize, op: &Operation) -> String {
    op.label(index)
}

pub(crate) fn ids_text(ids: &[u64]) -> String {
    ids.iter()
        .map(|id| format!("S{id}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Write via a temporary file and rename, so a machine never picks up a half-written
/// program. G-code is ASCII with LF line endings on every platform.
pub fn write_atomically(path: &Path, text: &str) -> std::io::Result<()> {
    if !text.is_ascii() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "G-code must be ASCII",
        ));
    }
    let mut tmp_name = path.file_name().unwrap_or_default().to_os_string();
    tmp_name.push(".tmp");
    let tmp = path.with_file_name(tmp_name);
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)
}

/// The project's drill that fits most of the circles `select` picks, for a new Holes
/// operation (the review of 2026-11-02: it took the last tool, usually an end mill); ties
/// go to the lower tool number. None when no drill fits one of them.
pub fn matching_drill(
    job: &Job,
    select: &chipload_job::Selector,
    contours: &[Contour],
) -> Option<i64> {
    let label = Message::new("toolpath-error-no-operation").arg("at", "new");
    let selection =
        chipload_toolpath::select::select_paths(contours, &select.resolved(&job.drawing), &label)
            .ok()?;
    let (holes, _) = chipload_toolpath::holes::find_holes(&selection.contours);
    let fits = |tool: &Tool| {
        holes
            .iter()
            .filter(|h| {
                (2.0 * h.radius - tool.diameter).abs()
                    <= chipload_toolpath::holes::DRILL_FIT_MM
                        + chipload_core::tolerances::CHAIN_TOL_MM
            })
            .count()
    };
    let mut best: Option<(usize, i64)> = None;
    for tool in job.tools.iter().filter(|t| t.kind == ToolType::Drill) {
        let count = fits(tool);
        if count > 0 && best.is_none_or(|(n, id)| count > n || (count == n && tool.id < id)) {
            best = Some((count, tool.id));
        }
    }
    best.map(|(_, id)| id)
}
