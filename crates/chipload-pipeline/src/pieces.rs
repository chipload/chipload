// SPDX-License-Identifier: GPL-3.0-or-later
//! A setup cut as a puzzle (docs/specs/pieces.md §6, §7): one program per piece, on a board
//! fitted round it. The cut-out operations cut each piece's own loops; every other
//! operation is planned once on the whole drawing and its moves trimmed to the piece's
//! reach.

use chipload_core::Point;
use chipload_i18n::{Message, Value};
use chipload_job::{Geometry, Job, Note, Operation, Selector, Shape, StockOrigin, WorkZero};
use chipload_toolpath::model::Program;
use chipload_toolpath::pieces::{
    Piece, PieceSet, Trimmer, bulged, piece_set, positions, reach, slowest_feed,
};
use chipload_toolpath::plan::{APPROACH_CLEARANCE_MM, OpSpan, Plan, ToolGroup};
use chipload_toolpath::{plan_job, validate_job};

use crate::{PipelineError, RunResult, estimate, load_drawing, post_and_check, posts};

/// Operations whose cuts move in Z along the way: split at a seam, the cutter plunges into
/// a cut that may be deeper than one step down (docs/specs/pieces.md §6.2).
fn moves_in_z(op: &Operation) -> bool {
    matches!(
        op,
        Operation::Vcarve(_)
            | Operation::Chamfer(_)
            | Operation::Prism(_)
            | Operation::Moulding(_)
            | Operation::Relief(_)
            | Operation::Flute(_)
            | Operation::Pattern(_)
    )
}

/// The pieces of a setup as they go together, for the setup sheet's map.
#[derive(Debug, Clone, PartialEq)]
pub struct PieceMap {
    pub columns: usize,
    pub rows: usize,
    pub joint: chipload_job::Joint,
    pub size: f64,
    pub clearance: f64,
    /// Each piece's name and loops, as points within the arc tolerance.
    pub pieces: Vec<(String, Vec<Vec<Point>>)>,
}

/// The active setup's pieces as they go together, or None when it is one board.
pub fn piece_map(job: &Job) -> Result<Option<PieceMap>, PipelineError> {
    let imported = load_drawing(job)?;
    piece_map_of(job, &imported.contours)
}

/// [`piece_map`] from the drawing's contours, already loaded (the window's).
pub fn piece_map_of(
    job: &Job,
    contours: &[chipload_core::chain::Contour],
) -> Result<Option<PieceMap>, PipelineError> {
    let Some(set) = piece_set(job, contours)? else {
        return Ok(None);
    };
    let shape = set.layout.shape;
    let clearance = job
        .setups
        .get(job.active_setup_index())
        .and_then(|s| s.pieces.as_ref())
        .map_or(0.0, |p| p.clearance);
    Ok(Some(PieceMap {
        columns: set.layout.columns(),
        rows: set.layout.rows(),
        joint: shape.joint,
        size: shape.size,
        clearance,
        pieces: set
            .pieces
            .iter()
            .map(|p| {
                let rings = p
                    .regions
                    .iter()
                    .map(|r| {
                        r.iter()
                            .flat_map(|prim| {
                                let mut pts = prim.points();
                                pts.pop();
                                pts
                            })
                            .collect()
                    })
                    .collect();
                (p.name.clone(), rings)
            })
            .collect(),
    }))
}

/// The active setup's pieces, or None when it is one board.
pub fn pieces_of(job: &Job) -> Result<Option<PieceSet>, PipelineError> {
    let imported = load_drawing(job)?;
    Ok(piece_set(job, &imported.contours)?)
}

/// The project as piece `piece` is cut: its drawing only the piece's loops, the cut-out
/// operations cutting them and nothing else on, its board fitted round them with the
/// setup's margin, and the setup's work zero on that board. A zero off the piece's board
/// moves to its bottom-left corner, with a note.
fn piece_job(one: &Job, set: &PieceSet, piece: &Piece) -> (Job, Vec<Note>) {
    let mut job = one.clone();
    let mut notes = Vec::new();
    job.drawing.shapes.clear();
    let mut ids = Vec::new();
    for (k, region) in piece.regions.iter().enumerate() {
        let (points, bulges) = bulged(region);
        let id = k as u64 + 1;
        ids.push(id);
        job.drawing.shapes.push(Shape {
            id,
            layer: "pieces".into(),
            name: None,
            geometry: Geometry::Polyline {
                closed: true,
                points,
                bulges,
            },
            construction: false,
        });
    }
    job.drawing.next_shape_id = ids.len() as u64 + 1;
    for (i, op) in job.operations.iter_mut().enumerate() {
        if set.cut_out.contains(&i) {
            if let Some(select) = op.select_mut() {
                *select = Selector {
                    shapes: Some(ids.clone()),
                    ..Selector::default()
                };
            }
        } else {
            op.set_enabled(false);
        }
    }
    let at = job.active_setup_index();
    let margin = job.setups[at]
        .stock
        .fit
        .unwrap_or(chipload_job::DEFAULT_PIECE_MARGIN);
    job.setups[at].stock.origin = StockOrigin::Drawing;
    job.setups[at].stock.fit = Some(margin);
    let ((x0, y0), (x1, y1)) = job.stock_bounds();
    let zero = job.work_zero();
    let off = match zero {
        WorkZero::Stock { .. } => None,
        WorkZero::DrawingOrigin {} => Some((0.0, 0.0)),
        WorkZero::Point { x, y } => Some((x, y)),
    };
    if let Some((x, y)) = off
        && !(x0..=x1).contains(&x) | !(y0..=y1).contains(&y)
    {
        job.setups[at].work_zero = WorkZero::Stock {
            corner: chipload_job::StockCorner::BottomLeft,
        };
        notes.push(Note::job(
            Message::new("pipeline-pieces-zero-off").arg("piece", piece.name.clone()),
        ));
    }
    (job, notes)
}

/// Piece `piece`'s moves: the whole plan's, each operation's kept within the piece's cell
/// grown by its tool's radius, and the cut-out operations' from the piece's own plan, in
/// the whole plan's order and tool groups. Also the spans and groups of the result, and
/// the operations split at a seam where the cutter plunges into a deeper cut.
fn trimmed(
    one: &Job,
    set: &PieceSet,
    piece: &Piece,
    whole: &Plan,
    own: &Plan,
) -> Result<Trimmed, PipelineError> {
    let safe_z = one.machine.safe_z;
    let approach = one.stock().top_z() + APPROACH_CLEARANCE_MM;
    let start = (0.0, 0.0, safe_z);
    let whole_at = positions(&whole.program.moves, start);
    let own_at = positions(&own.program.moves, start);
    let mut reaches: Vec<(f64, Vec<Point>)> = Vec::new();
    let mut t = Trimmer::new(safe_z, approach);
    let mut spans = Vec::new();
    let mut groups = Vec::new();
    let mut split = Vec::new();
    for group in &whole.groups {
        let first = t.moves.len();
        for span in whole
            .op_spans
            .iter()
            .filter(|s| group.moves.contains(&s.moves.start))
        {
            let begin = t.moves.len();
            let before = t.split_entries;
            if set.cut_out.contains(&span.operation) {
                for s in own
                    .op_spans
                    .iter()
                    .filter(|s| s.operation == span.operation && s.pass == span.pass)
                {
                    let moves = &own.program.moves[s.moves.clone()];
                    let plunge = slowest_feed(moves).unwrap_or(0.0);
                    t.add(moves, own_at[s.moves.start], None, plunge)?;
                }
            } else {
                let ring = match reaches.iter().find(|(r, _)| *r == span.tool_radius) {
                    Some((_, ring)) => ring.clone(),
                    None => {
                        let ring = reach(&piece.cell, span.tool_radius)?;
                        reaches.push((span.tool_radius, ring.clone()));
                        ring
                    }
                };
                let moves = &whole.program.moves[span.moves.clone()];
                let plunge = slowest_feed(moves).unwrap_or(0.0);
                t.add(moves, whole_at[span.moves.start], Some(&ring), plunge)?;
            }
            if t.split_entries > before
                && one.operations.get(span.operation).is_some_and(moves_in_z)
                && !split.contains(&span.operation)
            {
                split.push(span.operation);
            }
            if t.moves.len() > begin {
                spans.push(OpSpan {
                    moves: begin..t.moves.len(),
                    ..span.clone()
                });
            }
        }
        t.end_group();
        if t.moves.len() > first {
            groups.push(ToolGroup {
                moves: first..t.moves.len(),
                ..group.clone()
            });
        }
    }
    let Some(first) = groups.first() else {
        return Err(PipelineError::Job(
            Message::new("pipeline-pieces-empty").arg("piece", piece.name.clone()),
        ));
    };
    let program = Program {
        tool_id: first.tool.id,
        tool_diameter: first.tool.diameter,
        rpm: first.tool.rpm.unwrap_or(0.0),
        moves: t.moves,
        ..whole.program.clone()
    };
    Ok(Trimmed {
        program,
        spans,
        groups,
        split,
    })
}

/// A piece's moves with their operations' spans and tool groups, and the operations split
/// at a seam where the cutter plunges into a deeper cut.
struct Trimmed {
    program: Program,
    spans: Vec<OpSpan>,
    groups: Vec<ToolGroup>,
    split: Vec<usize>,
}

/// One piece's program.
#[derive(Debug, Clone)]
pub struct PieceRun {
    /// "A1".
    pub name: String,
    /// The project as the piece is cut: its board and work zero (docs/specs/pieces.md §7).
    pub job: Job,
    pub result: RunResult,
}

/// Every piece of the active setup as a program of its own, or only piece `only` ("A1").
/// The setup is planned once; each piece's cut-out is planned on its own loops
/// (docs/specs/pieces.md §6, §7).
pub fn run_pieces(job: &Job, only: Option<&str>) -> Result<Vec<PieceRun>, PipelineError> {
    let one = job.active_setup_id().map(|id| job.for_setup(id));
    let one = one.as_ref().unwrap_or(job);
    validate_job(one)?;
    let post = posts::job_post(one)?;
    let imported = load_drawing(one)?;
    let Some(set) = piece_set(one, &imported.contours)? else {
        return Ok(Vec::new());
    };
    if let Some(name) = only
        && set.piece(name).is_none()
    {
        return Err(PipelineError::Job(
            Message::new("pipeline-pieces-no-such").arg("piece", name.to_string()),
        ));
    }
    let whole = plan_job(one, &imported.contours)?;
    let mut runs = Vec::new();
    for piece in set
        .pieces
        .iter()
        .filter(|p| only.is_none_or(|n| n == p.name))
    {
        let (piece_job, mut notes) = piece_job(one, &set, piece);
        let own_drawing = load_drawing(&piece_job)?;
        let own = plan_job(&piece_job, &own_drawing.contours)?;
        let Trimmed {
            program,
            spans,
            groups,
            split,
        } = trimmed(one, &set, piece, &whole, &own)?;
        let (files, checked) = post_and_check(&piece_job, &post, &program, &groups)?;
        let estimate =
            estimate::estimate(&piece_job, &program, &spans, &groups, post.spindle_dwell_s);
        notes.extend(
            whole
                .notes
                .iter()
                .filter(|n| n.operation.is_none_or(|i| !set.cut_out.contains(&i)))
                .cloned(),
        );
        notes.extend(own.notes.iter().filter(|n| n.operation.is_some()).cloned());
        for op in split {
            notes.push(Note::at(
                op,
                Message::new("pipeline-pieces-split")
                    .arg("label", one.label_message(chipload_job::OpRef::op(op)))
                    .arg("piece", piece.name.clone()),
            ));
        }
        let (bx, by) = piece_job.stock_size();
        let board = one.setups[one.active_setup_index()]
            .pieces
            .as_ref()
            .map(|p| p.board);
        if let Some(b) = board
            && (bx > b.x + chipload_core::tolerances::CHAIN_TOL_MM
                || by > b.y + chipload_core::tolerances::CHAIN_TOL_MM)
        {
            notes.push(Note::job(
                Message::new("pipeline-pieces-board-larger")
                    .arg("piece", piece.name.clone())
                    .arg("x", Value::mm(bx))
                    .arg("y", Value::mm(by))
                    .arg("board_x", Value::mm(b.x))
                    .arg("board_y", Value::mm(b.y)),
            ));
        }
        runs.push(PieceRun {
            name: piece.name.clone(),
            result: RunResult {
                estimate,
                files,
                post: post.clone(),
                import_report: imported.report.clone(),
                contours: imported.contours.clone(),
                program,
                tabs: own.tabs.clone(),
                pockets: Vec::new(),
                op_spans: spans,
                groups,
                ring_starts: Vec::new(),
                unreached_corners: Vec::new(),
                tab_centres: Vec::new(),
                floor_hints: whole.floor_hints.clone(),
                notes,
                facts: whole.facts.clone(),
                check_warnings: checked.warnings,
                check_notes: checked.notes,
            },
            job: piece_job,
        });
    }
    Ok(runs)
}

/// The pieces' programs one after the other, as they lie in the drawing: the setup's whole
/// board cut piece by piece, for the simulation of the pieces put together
/// (docs/specs/pieces.md §7).
pub fn assembled(runs: &[PieceRun]) -> Option<(Program, Vec<ToolGroup>)> {
    let first = runs.first()?;
    let mut program = first.result.program.clone();
    program.moves.clear();
    let mut groups = Vec::new();
    for run in runs {
        let base = program.moves.len();
        program
            .moves
            .extend(run.result.program.moves.iter().copied());
        groups.extend(run.result.groups.iter().map(|g| ToolGroup {
            moves: g.moves.start + base..g.moves.end + base,
            ..g.clone()
        }));
    }
    Some((program, groups))
}

/// The project with its drawing only the active setup's pieces: each piece's loops as drawn
/// polylines on a layer named after it ("A1"), for `dxf export --pieces`
/// (docs/specs/pieces.md §7). Refused when the setup is one board.
pub fn pieces_drawing(job: &Job) -> Result<Job, PipelineError> {
    let Some(set) = pieces_of(job)? else {
        return Err(PipelineError::Job(Message::new("pipeline-pieces-none")));
    };
    let mut out = job.clone();
    out.drawing.shapes.clear();
    out.drawing.hidden.clear();
    let mut id = 1;
    for piece in &set.pieces {
        for region in &piece.regions {
            let (points, bulges) = bulged(region);
            out.drawing.shapes.push(Shape {
                id,
                layer: piece.name.clone(),
                name: None,
                geometry: Geometry::Polyline {
                    closed: true,
                    points,
                    bulges,
                },
                construction: false,
            });
            id += 1;
        }
    }
    out.drawing.next_shape_id = id;
    Ok(out)
}

/// The active setup in pieces as one result for a view of it: the pieces' programs put
/// together on the whole board (the window's preview and simulation), every piece's files,
/// named after it (`-A1-1-T1`), and the time of them all (docs/specs/pieces.md §9).
pub fn run_assembled(job: &Job, cancel: &crate::Cancel) -> Result<RunResult, PipelineError> {
    let one = job.active_setup_id().map(|id| job.for_setup(id));
    let one = one.as_ref().unwrap_or(job);
    let runs = run_pieces(one, None)?;
    cancel.check()?;
    let Some((program, groups)) = assembled(&runs) else {
        return Err(PipelineError::Job(Message::new("pipeline-pieces-none")));
    };
    let mut files = Vec::new();
    let mut spans = Vec::new();
    let mut tabs = Vec::new();
    let mut notes: Vec<Note> = Vec::new();
    let mut check_warnings = Vec::new();
    let mut check_notes = Vec::new();
    let mut base = 0;
    for run in &runs {
        for file in &run.result.files {
            let mut file = file.clone();
            let tail = file.suffix.take().unwrap_or_default();
            file.suffix = Some(format!("-{}{tail}", run.name));
            files.push(file);
        }
        spans.extend(run.result.op_spans.iter().map(|s| OpSpan {
            moves: s.moves.start + base..s.moves.end + base,
            ..s.clone()
        }));
        base += run.result.program.moves.len();
        tabs.extend(run.result.tabs.iter().cloned());
        for note in &run.result.notes {
            if !notes.contains(note) {
                notes.push(note.clone());
            }
        }
        check_warnings.extend(run.result.check_warnings.iter().cloned());
        check_notes.extend(run.result.check_notes.iter().cloned());
    }
    let first = &runs[0].result;
    let estimate = estimate::estimate(one, &program, &spans, &groups, first.post.spindle_dwell_s);
    Ok(RunResult {
        estimate,
        files,
        post: first.post.clone(),
        import_report: first.import_report.clone(),
        contours: first.contours.clone(),
        program,
        tabs,
        pockets: Vec::new(),
        op_spans: spans,
        groups,
        ring_starts: Vec::new(),
        unreached_corners: Vec::new(),
        tab_centres: Vec::new(),
        floor_hints: first.floor_hints.clone(),
        notes,
        facts: first.facts.clone(),
        check_warnings,
        check_notes,
    })
}
