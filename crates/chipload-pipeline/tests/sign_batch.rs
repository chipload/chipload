// SPDX-License-Identifier: GPL-3.0-or-later
//! Signs from a list (docs/specs/batch-signs.md §6).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use chipload_i18n::Language;
use chipload_job::{Job, Operation};
use chipload_pipeline::fonts::BuiltinOnly;
use chipload_pipeline::frames::follow_frames;
use chipload_pipeline::sign::{SignRequest, SignStyle};
use chipload_pipeline::sign_batch::{BatchRequest, DEFAULT_BATCH_GAP_MM, add_sign_batch};
use chipload_pipeline::{empty_project, run_setups};
use common::corpus_value;

/// An empty project with the corpus's tools: 1 a 6 mm end mill, 2 a 60° V-bit.
fn project() -> Job {
    let tools = chipload_job::parse_job(corpus_value("jobs/v27/text.chipload"), "test")
        .unwrap()
        .tools;
    let mut job = empty_project();
    job.tools = tools;
    job
}

fn batch(list: &[&str]) -> BatchRequest {
    let mut sign = SignRequest::new("", 2, 1);
    sign.size = Some((200.0, 60.0));
    BatchRequest {
        sign,
        list: list.iter().map(|s| s.to_string()).collect(),
        gap: DEFAULT_BATCH_GAP_MM,
    }
}

fn size_of(job: &Job, id: u64) -> (f64, f64) {
    let ((x0, y0), (x1, y1)) = chipload_pipeline::shapes::drawn_box(job, &[id]).unwrap();
    (x1 - x0, y1 - y0)
}

const NAMES: [&str; 5] = [
    "ANNA BERG",
    "BO LIND",
    "CARL | ROOM 12",
    "DAVID NORDSTRÖM",
    "EVA",
];

#[test]
fn a_list_makes_every_sign_in_one_part_with_one_set_of_operations() {
    let (job, made) =
        add_sign_batch(&project(), &batch(&NAMES), &BuiltinOnly, None, Language::En).unwrap();
    assert_eq!(made.texts.len(), 5);
    assert_eq!(made.outlines.len(), 5);
    assert_eq!(made.groups.len(), 5);
    assert_eq!(job.parts.len(), 1);
    assert_eq!(job.parts[0].name, "ANNA BERG +4");
    let names: Vec<&str> = made
        .operations
        .iter()
        .map(|&i| job.operations[i].name())
        .collect();
    assert_eq!(names, ["Letters", "Borders", "Cut out"]);
    // V-bit, V-bit, end mill: the tool changes once.
    let tools: Vec<i64> = made
        .operations
        .iter()
        .map(|&i| job.operations[i].tool())
        .collect();
    assert_eq!(tools, [2, 2, 1]);
    // Every frame the size asked; every text the same height, fitted to the longest name.
    for &outline in &made.outlines {
        let (w, h) = size_of(&job, outline);
        assert!(
            (w - 200.0).abs() < 1e-6 && (h - 60.0).abs() < 1e-6,
            "{w} × {h}"
        );
    }
    let height = made.fitted.unwrap();
    assert!(job.drawing.texts.iter().all(|t| t.height == height));
    let mut alone = SignRequest::new("EVA", 2, 1);
    alone.size = Some((200.0, 60.0));
    let (_, eva) =
        chipload_pipeline::sign::add_sign(&project(), &alone, &BuiltinOnly, None, Language::En)
            .unwrap();
    assert!(height < eva.fitted.unwrap(), "the longest name sets it");
    // A bar starts a second line.
    assert!(job.drawing.texts.iter().any(|t| t.text == "CARL\nROOM 12"));
    // No two signs overlap: their outlines' boxes are apart by the gap.
    let boxes: Vec<_> = made
        .outlines
        .iter()
        .map(|id| chipload_pipeline::shapes::drawn_box(&job, &[*id]).unwrap())
        .collect();
    for (i, a) in boxes.iter().enumerate() {
        for b in &boxes[i + 1..] {
            let apart = a.1.0 + DEFAULT_BATCH_GAP_MM - 1e-6 <= b.0.0
                || b.1.0 + DEFAULT_BATCH_GAP_MM - 1e-6 <= a.0.0
                || a.1.1 + DEFAULT_BATCH_GAP_MM - 1e-6 <= b.0.1
                || b.1.1 + DEFAULT_BATCH_GAP_MM - 1e-6 <= a.0.1;
            assert!(apart, "{a:?} {b:?}");
        }
    }
    run_setups(&job, None).unwrap();
}

#[test]
fn a_name_changed_later_keeps_its_frame_and_is_still_cut() {
    let (job, made) =
        add_sign_batch(&project(), &batch(&NAMES), &BuiltinOnly, None, Language::En).unwrap();
    let mut changed = job.clone();
    let text = made.texts[1][0];
    let at = changed
        .drawing
        .texts
        .iter()
        .position(|t| t.id == text)
        .unwrap();
    changed.drawing.texts[at].text = "BO LINDQVIST".into();
    let (changed, _) =
        chipload_pipeline::texts::remake(&job, &changed, text, &BuiltinOnly).unwrap();
    let (changed, _) = follow_frames(changed);
    let (w, _) = size_of(&changed, made.outlines[1]);
    assert!((w - 200.0).abs() < 1e-6);
    // The letters' operation selects by layer, so the new letters are cut.
    let Operation::Vcarve(letters) = &changed.operations[made.operations[0]] else {
        panic!("a V-carve");
    };
    assert_eq!(
        letters.select.layer.as_deref(),
        Some(made.layers[0].as_str())
    );
    run_setups(&changed, None).unwrap();
}

#[test]
fn a_raised_list_pockets_each_sign_and_inlays_are_refused() {
    // Letters without holes: a 6 mm end mill can't clear an A's or a B's at this size,
    // as for one raised sign.
    let mut raised = batch(&["TIM", "LUC", "SUE"]);
    raised.sign.style = SignStyle::Raised;
    let (job, made) =
        add_sign_batch(&project(), &raised, &BuiltinOnly, None, Language::En).unwrap();
    let names: Vec<&str> = made
        .operations
        .iter()
        .map(|&i| job.operations[i].name())
        .collect();
    assert_eq!(
        names,
        [
            "Borders",
            "Background",
            "Background",
            "Background",
            "Cut out"
        ]
    );
    run_setups(&job, None).unwrap();
    let mut inlay = batch(&NAMES[..2]);
    inlay.sign.style = SignStyle::Inlay;
    let err = add_sign_batch(&project(), &inlay, &BuiltinOnly, None, Language::En).unwrap_err();
    assert!(err.message().english().contains("one at a time"));
}

#[test]
fn a_kept_board_too_small_or_an_empty_list_is_refused() {
    let mut kept = batch(&NAMES);
    kept.sign.board = None;
    let mut job = project();
    job.stock_mut().x = 300.0;
    job.stock_mut().y = 100.0;
    job.stock_mut().fit = None;
    let err = add_sign_batch(&job, &kept, &BuiltinOnly, None, Language::En).unwrap_err();
    assert!(
        err.message().english().contains("need a board of"),
        "{}",
        err.message().english()
    );
    let err = add_sign_batch(
        &project(),
        &batch(&["", "  "]),
        &BuiltinOnly,
        None,
        Language::En,
    )
    .unwrap_err();
    assert!(err.message().english().contains("no signs"));
}
