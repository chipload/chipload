// SPDX-License-Identifier: GPL-3.0-or-later
//! Every setup's programs and setup sheets written at once (docs/specs/parts-and-setups.md
//! §8, *Export all setups*): the files `chipload run` writes, for the window. Every setup is
//! planned and checked first; nothing is written unless all of them are.

use std::path::{Path, PathBuf};

use chipload_job::Job;

use crate::sheet::{PieceSheet, SheetInfo, pieces_sheet, setup_sheet_path, write_setup_sheet};
use crate::{PipelineError, SetupRun, run_setups_pieces, write_atomically, write_gcode_files};

/// What was written.
#[derive(Debug, Clone, PartialEq)]
pub struct Exported {
    /// The setups written, in their order: each one's id and name.
    pub setups: Vec<(u64, String)>,
    /// The G-code files, in the order written.
    pub files: Vec<PathBuf>,
    /// The setup sheets, one a setup, when asked for.
    pub sheets: Vec<PathBuf>,
}

/// Why nothing, or not everything, was written.
#[derive(Debug, Clone, PartialEq)]
pub enum ExportError {
    /// A setup couldn't be planned or failed the checker: nothing was written.
    Run(PipelineError),
    /// A file couldn't be written: the file system's own words.
    Write { path: PathBuf, error: String },
}

/// Plan every setup that is on and has something to cut, then write each one's programs
/// next to `output` (`-S1`, `-S2-1-T1`, a piece's `-S2-A1`) and, with `sheet`, its setup
/// sheet (one for all the pieces of a setup in pieces).
pub fn export_all(
    job: &Job,
    output: &Path,
    sheet: Option<&SheetInfo>,
) -> Result<Exported, ExportError> {
    let runs = run_setups_pieces(job, None, None).map_err(ExportError::Run)?;
    let mut exported = Exported {
        setups: Vec::new(),
        files: Vec::new(),
        sheets: Vec::new(),
    };
    for (index, run) in runs.iter().enumerate() {
        let written =
            write_gcode_files(output, &run.result.files).map_err(|e| ExportError::Write {
                path: output.to_path_buf(),
                error: e.to_string(),
            })?;
        exported.files.extend(written);
        // The last run of its setup: the setup is done, its sheet follows.
        let last = runs
            .get(index + 1)
            .is_none_or(|next| next.setup != run.setup);
        if !last {
            continue;
        }
        let one = job.for_setup(run.setup);
        let name = one
            .setup(run.setup)
            .map(|s| s.name.clone())
            .unwrap_or_default();
        exported.setups.push((run.setup, name));
        let Some(info) = sheet else {
            continue;
        };
        let path = setup_sheet_path(&one, output);
        let wrote = if run.piece.is_some() {
            let mine: Vec<&SetupRun> = runs.iter().filter(|r| r.setup == run.setup).collect();
            write_pieces_sheet(&one, &mine, output, info)
        } else {
            write_setup_sheet(&one, &run.result, output, info).map_err(|e| e.to_string())
        };
        match wrote {
            Ok(path) => exported.sheets.push(path),
            Err(error) => return Err(ExportError::Write { path, error }),
        }
    }
    Ok(exported)
}

/// The sheet of a setup in pieces, from its pieces' runs, next to the G-code: written where
/// a setup's sheet goes (docs/specs/pieces.md §8). Returns its path.
pub fn write_pieces_sheet(
    one: &Job,
    runs: &[&SetupRun],
    output: &Path,
    info: &SheetInfo,
) -> Result<PathBuf, String> {
    let map = crate::pieces::piece_map(one)
        .map_err(|e| e.to_string())?
        .ok_or("the setup is not in pieces")?;
    let pieces: Vec<PieceSheet> = runs
        .iter()
        .filter_map(|r| {
            Some(PieceSheet {
                name: r.piece.as_deref()?,
                job: &r.job,
                result: &r.result,
            })
        })
        .collect();
    let html = pieces_sheet(one, &map, &pieces, output, info);
    let path = setup_sheet_path(one, output);
    write_atomically(&path, &html).map_err(|e| e.to_string())?;
    Ok(path)
}
