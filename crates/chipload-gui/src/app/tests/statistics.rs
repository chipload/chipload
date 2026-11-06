// SPDX-License-Identifier: GPL-3.0-or-later
//! The Statistics section of an operation's properties
//! (docs/specs/operation-statistics.md §3.1).

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use super::forms::Statistics;
use super::*;
use chipload_job::OpRef;

#[test]
fn the_numbers_come_with_a_run_and_go_with_a_change() {
    let mut app = lower_part();
    app.set_auto_generate(false);
    assert!(matches!(
        app.statistics_of(OpRef::op(0)),
        Statistics::OutOfDate
    ));
    assert!(app.generate_and_wait(), "{:?}", app.report);
    let Statistics::Counted { counts, copies, .. } = app.statistics_of(OpRef::op(0)) else {
        panic!("counted after a run");
    };
    let estimate = &app.toolpath.result.as_ref().unwrap().estimate;
    assert_eq!(estimate.operation_counts[0].1, counts);
    assert!(counts.cutting_mm > 0.0 && counts.lifts > 0, "{counts:?}");
    assert_eq!(copies, 1);
    // A change: out of date until the next run.
    edit_feed(&mut app, 650.0);
    assert!(matches!(
        app.statistics_of(OpRef::op(0)),
        Statistics::OutOfDate
    ));
    // An operation turned off isn't cut.
    match &mut app.edit.as_mut().unwrap().job.operations[1] {
        Operation::Profile(p) => p.enabled = false,
        _ => panic!("operation 2 is a profile"),
    }
    app.commit();
    assert!(app.generate_and_wait(), "{:?}", app.report);
    assert!(matches!(app.statistics_of(OpRef::op(1)), Statistics::Off));
    assert!(matches!(
        app.statistics_of(OpRef::op(0)),
        Statistics::Counted { .. }
    ));
    // A run that failed says so.
    app.toolpath.result = None;
    app.toolpath.failed = true;
    assert!(matches!(
        app.statistics_of(OpRef::op(0)),
        Statistics::Failed
    ));
}

#[test]
fn the_section_is_last_in_the_properties_and_folds_to_the_time() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .build_eframe(|cc| {
            open_sections(&cc.egui_ctx);
            let mut app = lower_part();
            app.set_auto_generate(false);
            app.focus = Focus::Op(0);
            app
        });
    harness.run_steps(4);
    let statistics = harness.get_by_label_contains("Statistics").rect().top();
    let feeds = harness.get_by_label_contains("Feeds").rect().top();
    assert!(statistics > feeds, "{statistics} under {feeds}");
    assert!(
        harness
            .query_by_label("Generate (F5) to see the statistics.")
            .is_some()
    );
    assert!(harness.state_mut().generate_and_wait());
    harness.run_steps(4);
    for label in ["Time", "Cutting", "Plunging", "Rapids", "Lifts"] {
        assert!(harness.query_by_label(label).is_some(), "{label}");
    }
    let seconds = harness
        .state()
        .toolpath
        .result
        .as_ref()
        .unwrap()
        .estimate
        .operation_counts[0]
        .1
        .seconds();
    let time = chipload_pipeline::estimate::duration_text(seconds);
    assert!(
        harness
            .query_by_label_contains(&format!("{time} (cutting "))
            .is_some(),
        "{time}"
    );
}

/// A part's Statistics: its operations' numbers summed, and why there are none
/// (docs/specs/part-statistics.md §3.1).
#[test]
fn a_part_sums_its_operations_and_says_why_when_it_cant() {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v22/two_parts.chipload")));
    app.set_auto_generate(false);
    app.display.all_parts = true;
    assert!(matches!(app.part_statistics(2), Statistics::OutOfDate));
    assert!(app.generate_and_wait(), "{:?}", app.report);
    let Statistics::Counted {
        counts,
        copies,
        operations,
    } = app.part_statistics(2)
    else {
        panic!("counted after a run");
    };
    let job = app.edited_job().unwrap();
    let estimate = &app.toolpath.result.as_ref().unwrap().estimate;
    let ops: Vec<_> = estimate
        .operation_counts
        .iter()
        .filter(|(at, _)| job.operations[at.operation].part() == 2)
        .collect();
    assert_eq!(operations, Some(ops.len()));
    assert_eq!(
        counts.lifts,
        ops.iter().map(|(_, c)| c.lifts).sum::<usize>()
    );
    assert_eq!(copies, 2);
    // Only the active part previewed: the other part wasn't planned.
    app.display.all_parts = false;
    let active = app.edited_job().unwrap().active_part_id().unwrap();
    let other = if active == 1 { 2 } else { 1 };
    assert!(app.generate_and_wait(), "{:?}", app.report);
    assert!(matches!(
        app.part_statistics(active),
        Statistics::Counted { .. }
    ));
    assert!(matches!(app.part_statistics(other), Statistics::OutOfDate));
    // A new part has no operations.
    let added = app.add_part().unwrap();
    assert!(matches!(app.part_statistics(added), Statistics::Empty));
    // A part that is off isn't cut.
    app.edit.as_mut().unwrap().job.parts[0].enabled = false;
    app.commit();
    let first = app.edited_job().unwrap().parts[0].id;
    assert!(app.generate_and_wait(), "{:?}", app.report);
    assert!(matches!(app.part_statistics(first), Statistics::Off));
}

#[test]
fn a_parts_section_is_last_in_its_properties() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .build_eframe(|cc| {
            open_sections(&cc.egui_ctx);
            let mut app = ChiploadApp::default();
            assert!(app.open(&corpus("jobs/v22/two_parts.chipload")));
            app.set_auto_generate(false);
            app.display.all_parts = true;
            app.focus = Focus::Part(2);
            app
        });
    harness.run_steps(4);
    assert!(harness.state_mut().generate_and_wait());
    harness.run_steps(4);
    let statistics = harness.get_by_label_contains("Statistics").rect().top();
    let note = harness
        .get_by_label_contains("Parts are cut one after the other")
        .rect()
        .top();
    assert!(statistics > note, "{statistics} under {note}");
    assert!(harness.query_by_label("Lifts").is_some());
    assert!(harness.query_by_label_contains("For 2 copies.").is_some());
}
