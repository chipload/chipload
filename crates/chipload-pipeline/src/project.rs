// SPDX-License-Identifier: GPL-3.0-or-later
//! New projects with the app's defaults, and a drawing updated from its file.

use std::path::Path;

use chipload_core::tolerances::CHAIN_TOL_MM;
use chipload_dxf::{ImportResult, ImportedShapes, contours_from_shapes, sha256_hex};
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{
    CURRENT_VERSION, Drawing, Job, Machine, Spindle, Stock, StockCorner, StockOrigin, Tool,
    ToolChange, ToolType, Units, WorkZero, ZZero,
};

use crate::PipelineError;

/// The project's contours and import report (from its embedded shapes).
pub fn load_drawing(job: &Job) -> Result<ImportResult, PipelineError> {
    Ok(contours_from_shapes(&job.drawing.shapes, &job.drawing.dxfs))
}

/// Defaults for a new project (docs/specs/job-editor.md §6): all visible and editable.
pub const DEFAULT_THICKNESS: f64 = 10.0;
/// How far a new project's stock sticks out round the drawing, each side, mm (the user,
/// 2026-10-28).
pub const DEFAULT_STOCK_MARGIN: f64 = 10.0;
/// How far a new project may cut below the stock (spoilboard), mm (decision 2026-10-10).
pub const DEFAULT_MAX_OVERCUT: f64 = 0.5;
pub const DEFAULT_SAFE_Z: f64 = 5.0;
pub const DEFAULT_TOOL_DIAMETER: f64 = 6.0;
pub const DEFAULT_RPM: f64 = 18000.0;

/// Step down of a new operation with nothing to copy from, mm.
pub const DEFAULT_STEP_DOWN: f64 = 1.0;

/// Stock of a project started without a drawing, mm (docs/specs/new-project.md decision 1).
pub const DEFAULT_EMPTY_STOCK: (f64, f64) = (300.0, 200.0);

/// A new project with no drawing yet (docs/specs/new-project.md): stock fitting the drawing
/// with a 10 mm margin once there is one (300 × 200 mm centred on the origin until then),
/// work zero at its bottom-left corner, GRBL, one 6 mm endmill, no operations.
pub fn empty_project() -> Job {
    Job {
        schema_version: CURRENT_VERSION,
        written_by: None,
        drawing: Drawing {
            shapes: Vec::new(),
            next_shape_id: 1,
            dxfs: Vec::new(),
            images: Vec::new(),
            texts: Vec::new(),
            groups: Vec::new(),
            frames: Vec::new(),
            models: Vec::new(),
            hidden: Vec::new(),
        },
        setups: vec![chipload_job::Setup {
            id: 1,
            name: "Setup 1".into(),
            enabled: true,
            stock: Stock {
                x: DEFAULT_EMPTY_STOCK.0,
                y: DEFAULT_EMPTY_STOCK.1,
                thickness: DEFAULT_THICKNESS,
                max_overcut: DEFAULT_MAX_OVERCUT,
                // Fitting the drawing is the default (the user, 2026-10-28).
                origin: StockOrigin::Drawing,
                fit: Some(DEFAULT_STOCK_MARGIN),
                material: None,
                z_zero: ZZero::Top,
            },
            // The board's corner the operator touches off on (the user, 2026-10-29;
            // docs/specs/work-zero-marker.md §7).
            work_zero: WorkZero::Stock {
                corner: StockCorner::BottomLeft,
            },
            back_of: None,
            pieces: None,
            nest: None,
            instances: vec![chipload_job::Instance::drawn(1, 1)],
            made_for_stock: false,
            placed: None,
        }],
        active_setup: Some(1),
        machine: Machine {
            name: None,
            library_uid: None,
            post: "grbl".into(),
            gcode_units: chipload_job::Units::Mm,
            safe_z: DEFAULT_SAFE_Z,
            spindle: Spindle::Cw,
            max_feed: None,
            travel: None,
            tool_change: ToolChange::Files,
            custom_post: None,
        },
        tools: vec![Tool {
            id: 1,
            name: None,
            library_uid: None,
            kind: ToolType::Endmill,
            diameter: DEFAULT_TOOL_DIAMETER,
            flutes: None,
            center_cutting: None,
            rpm: Some(DEFAULT_RPM),
            angle: None,
            tip_diameter: None,
            corner_radius: None,
        }],
        parts: vec![chipload_job::Part {
            id: 1,
            name: "Part 1".into(),
            enabled: true,
            order: chipload_job::InstanceOrder::ByOperation,
            mirror: None,
            turns: None,
            stock: None,
        }],
        active_part: Some(1),
        operations: Vec::new(),
    }
}

/// A new project's starting values when lengths are shown in inches
/// (docs/specs/inch-units.md §5.2): tidy inch numbers, not converted mm ones. Stock 3/4"
/// thick (24" × 12" without a drawing) with a 1/2" margin round the drawing, 0.02" below it allowed, safe Z 0.2", a 1/4" endmill.
/// Only values that are still the mm defaults change; the project stays in mm.
pub fn with_inch_defaults(mut job: Job) -> Job {
    let inch = chipload_core::MM_PER_INCH;
    if job.drawing.shapes.is_empty() && (job.stock().x, job.stock().y) == DEFAULT_EMPTY_STOCK {
        job.stock_mut().x = 24.0 * inch;
        job.stock_mut().y = 12.0 * inch;
    }
    if job.stock().fit == Some(DEFAULT_STOCK_MARGIN) {
        job.stock_mut().fit = Some(0.5 * inch);
    }
    if job.stock().thickness == DEFAULT_THICKNESS {
        job.stock_mut().thickness = 0.75 * inch;
    }
    if job.stock().max_overcut == DEFAULT_MAX_OVERCUT {
        job.stock_mut().max_overcut = 0.02 * inch;
    }
    if job.machine.safe_z == DEFAULT_SAFE_Z {
        job.machine.safe_z = 0.2 * inch;
    }
    for tool in &mut job.tools {
        if tool.library_uid.is_none() && tool.diameter == DEFAULT_TOOL_DIAMETER {
            tool.diameter = 0.25 * inch;
        }
    }
    job
}

/// A new project holding an imported drawing: stock covering it (origin bottom left if it
/// lies in the first quadrant, else centred), GRBL, one 6 mm endmill, no operations.
pub fn new_project(imported: ImportedShapes) -> Job {
    let bbox = contours_from_shapes(&imported.shapes, std::slice::from_ref(&imported.source))
        .report
        .bbox;
    let ((x0, y0), (x1, y1)) = bbox.unwrap_or(((0.0, 0.0), (100.0, 100.0)));
    // Centred on the drawing, sticking out equally all round (the user, 2026-10-28;
    // docs/specs/stock-around-drawing.md).
    let (origin, x, y) = (
        StockOrigin::Drawing,
        x1 - x0 + 2.0 * DEFAULT_STOCK_MARGIN,
        y1 - y0 + 2.0 * DEFAULT_STOCK_MARGIN,
    );
    let next_shape_id = imported.shapes.iter().map(|s| s.id).max().unwrap_or(0) + 1;
    let mut job = empty_project();
    job.drawing = Drawing {
        shapes: imported.shapes,
        next_shape_id,
        dxfs: vec![imported.source],
        images: Vec::new(),
        texts: Vec::new(),
        groups: Vec::new(),
        frames: Vec::new(),
        models: Vec::new(),
        hidden: Vec::new(),
    };
    job.stock_mut().x = x.max(1.0);
    job.stock_mut().y = y.max(1.0);
    job.stock_mut().origin = origin;
    job.stock_mut().fit = Some(DEFAULT_STOCK_MARGIN);
    job
}

/// A note when the drawing reaches beyond the stock (docs/specs/new-project.md §3); None
/// when it fits or is empty.
pub fn stock_fit_note(job: &Job) -> Option<String> {
    stock_fit_message(job).map(|m| m.english())
}

/// [`stock_fit_note`] as a message.
pub fn stock_fit_message(job: &Job) -> Option<Message> {
    let bbox = contours_from_shapes(&job.drawing.shapes, &job.drawing.dxfs)
        .report
        .bbox?;
    let ((x0, y0), (x1, y1)) = bbox;
    let ((sx0, sy0), (sx1, sy1)) = job.stock_bounds();
    let fits = x0 >= sx0 - CHAIN_TOL_MM
        && y0 >= sy0 - CHAIN_TOL_MM
        && x1 <= sx1 + CHAIN_TOL_MM
        && y1 <= sy1 + CHAIN_TOL_MM;
    let number = |v: f64| Value::LengthNumber(v, Digits::Fixed(3));
    let length = |v: f64| Value::Length(v, Digits::Fixed(3));
    (!fits).then(|| {
        Message::new("pipeline-stock-fit")
            .arg("x0", number(x0))
            .arg("x1", length(x1))
            .arg("y0", number(y0))
            .arg("y1", length(y1))
            .arg("width", number(job.stock_size().0))
            .arg("length", length(job.stock_size().1))
    })
}

/// What updating a drawing from a DXF did (docs/specs/project-file.md §4).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct UpdateReport {
    pub unchanged: Vec<u64>,
    pub changed: Vec<u64>,
    pub added: Vec<u64>,
    pub removed: Vec<u64>,
    /// Operations (indices) that selected a removed shape: they can't run until fixed.
    pub lost: Vec<usize>,
    /// The report as sentences, in English, for the command line: `messages` in English.
    pub lines: Vec<String>,
    /// The report as messages, for the app (docs/specs/translations.md §7).
    pub messages: Vec<Message>,
}

/// Update the project's one DXF from `dxf` ([`crate::dxfs::update_dxf`]); with none, `dxf` becomes
/// its first ([`crate::dxfs::add_dxf`]). Refused when it has several: name the DXF.
pub fn update_drawing(
    job: &Job,
    dxf: &Path,
    units: Option<Units>,
) -> Result<(Job, UpdateReport), PipelineError> {
    update_drawing_with(job, dxf, units, None)
}

/// [`update_drawing`] from a drawing file, DXF or SVG; `px_per_inch` for an SVG without a
/// real size (docs/specs/svg-import.md §7).
pub fn update_drawing_with(
    job: &Job,
    dxf: &Path,
    units: Option<Units>,
    px_per_inch: Option<f64>,
) -> Result<(Job, UpdateReport), PipelineError> {
    match job.drawing.dxfs.as_slice() {
        [] => crate::dxfs::add_drawing_file(job, dxf, units, px_per_inch, None),
        [only] => crate::dxfs::update_drawing_file(job, only.id, dxf, units, px_per_inch),
        several => Err(PipelineError::Job(
            Message::new("pipeline-update-which-dxf")
                .arg("count", several.len())
                .arg("dxfs", crate::placement::dxf_list(several)),
        )),
    }
}

/// The sha256 of a drawing file, lowercase hex, as stored in a project.
pub fn drawing_sha256(path: &Path) -> std::io::Result<String> {
    Ok(sha256_hex(&std::fs::read(path)?))
}
