// SPDX-License-Identifier: GPL-3.0-or-later
//! DXF export of a project (docs/specs/dxf-export.md, part A).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use chipload_dxf::shapes_from_dxf_bytes;
use chipload_job::load_job;
use chipload_pipeline::dxfs::{ExportOptions, export_drawing};

fn corpus(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus")
        .join(rel)
}

/// Every corpus project, by its path relative to the corpus.
fn projects() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut todo = vec![corpus("jobs")];
    while let Some(dir) = todo.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                todo.push(path);
            } else if path.extension().is_some_and(|e| e == "chipload") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

#[test]
fn every_corpus_project_exports_and_reads_back() {
    // The export reads its file back before it hands it on; here every project of the
    // corpus goes through it, with every kind of shape they hold.
    let mut exported = 0;
    for path in projects() {
        let job = load_job(&path).unwrap();
        if job.drawing.shapes.is_empty() {
            continue;
        }
        let options = ExportOptions {
            hidden: true,
            ..ExportOptions::default()
        };
        let (out, _) = export_drawing(&job, &options)
            .unwrap_or_else(|e| panic!("{}: {}", path.display(), e.message().english()));
        let back = shapes_from_dxf_bytes(&out.bytes, "x.dxf", None, 1.0).unwrap();
        assert!(!back.shapes.is_empty(), "{}", path.display());
        exported += 1;
    }
    assert!(exported > 40, "only {exported} projects");
}

#[test]
fn hidden_layers_and_chosen_shapes_are_left_out_or_in() {
    // S1 rectangle, S2 circle, S3 triangle, all on layer "drawn".
    let mut job = load_job(&corpus("jobs/v5/drawn_part.chipload")).unwrap();
    let count = |job: &chipload_job::Job, options: &ExportOptions| {
        let (out, _) = export_drawing(job, options).unwrap();
        out.entities.values().sum::<usize>()
    };
    assert_eq!(count(&job, &ExportOptions::default()), 3);
    let only_two = ExportOptions {
        shapes: vec![2],
        ..ExportOptions::default()
    };
    assert_eq!(count(&job, &only_two), 1);
    // Hidden circles (the Browser's "drawn:circle") stay out unless asked for.
    job.drawing.hidden.push("drawn:circle".into());
    assert_eq!(count(&job, &ExportOptions::default()), 2);
    let hidden = ExportOptions {
        hidden: true,
        ..ExportOptions::default()
    };
    assert_eq!(count(&job, &hidden), 3);
    let missing = ExportOptions {
        layers: vec!["nope".into()],
        ..ExportOptions::default()
    };
    let err = export_drawing(&job, &missing).unwrap_err();
    assert_eq!(err.message().id, "pipeline-export-no-layer");
}
