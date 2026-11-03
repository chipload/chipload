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
    let Statistics::Counted { counts, copies } = app.statistics_of(OpRef::op(0)) else {
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
        .build_eframe(|_| {
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
