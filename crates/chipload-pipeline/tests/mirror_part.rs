// SPDX-License-Identifier: GPL-3.0-or-later
//! Mirrored parts (docs/specs/mirror-part.md): a part cut mirrored about its box's centre
//! line, planned with the cutting sense turned so climb stays climb.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use chipload_job::{Copies, InstanceOrder, Job, Mirror, Turn};
use chipload_pipeline::run_job;
use chipload_pipeline::setups::add_back_side;
use chipload_toolpath::model::Program;
use common::{every_direction, loops, turning};

fn turn_of(mirror: Mirror) -> Turn {
    match mirror {
        Mirror::LeftRight => Turn::LeftRight,
        Mirror::TopBottom => Turn::TopBottom,
    }
}

/// The centre line of part 1's box along the mirror's axis.
fn axis(job: &Job, mirror: Mirror) -> f64 {
    let ((x0, y0), (x1, y1)) = job.part_box(1).unwrap();
    match mirror {
        Mirror::LeftRight => (x0 + x1) / 2.0,
        Mirror::TopBottom => (y0 + y1) / 2.0,
    }
}

/// The share of each operation's cut area that runs counter-clockwise, and the area.
fn shares(program: &Program, spans: &[chipload_toolpath::OpSpan]) -> Vec<(usize, f64, f64)> {
    turning(&loops(program, spans))
        .into_iter()
        .map(|(op, ccw, cw)| (op, ccw / (ccw - cw), ccw - cw))
        .collect()
}

/// Each operation turns the same way in `got` as in `want`, over about the same area.
fn same_turning(got: &[(usize, f64, f64)], want: &[(usize, f64, f64)], what: &str) {
    assert_eq!(got.len(), 5, "{what}: every operation cut: {got:?}");
    assert_eq!(got.len(), want.len(), "{what}");
    for (g, w) in got.iter().zip(want) {
        assert_eq!(g.0, w.0, "{what}");
        assert!(g.2 > 1.0, "{what}: operation {} cuts", g.0);
        // Where each ring starts moves its ramp's overlap a little (two-sided.rs).
        assert!(
            (g.2 - w.2).abs() < 0.1 * w.2,
            "{what}: operation {}: {g:?}, want {w:?}",
            g.0
        );
        assert!(
            (g.1 - w.1).abs() < 0.01,
            "{what}: operation {}: {g:?}, want {w:?}",
            g.0
        );
    }
}

/// The box round a program's feed moves.
fn cut_box(program: &Program) -> ((f64, f64), (f64, f64)) {
    let spans = [chipload_toolpath::OpSpan {
        operation: 0,
        pass: None,
        copy: 1,
        moves: 0..program.moves.len(),
        tool_radius: 0.0,
    }];
    chipload_toolpath::plan::cut_extent(&program.moves, &spans, None).unwrap()
}

fn close(a: ((f64, f64), (f64, f64)), b: ((f64, f64), (f64, f64))) -> bool {
    let near = |p: f64, q: f64| (p - q).abs() < 0.05;
    near(a.0.0, b.0.0) && near(a.0.1, b.0.1) && near(a.1.0, b.1.0) && near(a.1.1, b.1.1)
}

#[test]
fn climb_stays_climb_on_a_mirrored_part() {
    for mirror in [Mirror::LeftRight, Mirror::TopBottom] {
        let mut job = every_direction(None);
        let at = axis(&job, mirror);
        job.parts[0].mirror = Some(mirror);
        let result = run_job(&job).unwrap();
        // The shapes drawn mirrored by hand, cut as drawn.
        let by_hand = every_direction(Some((turn_of(mirror), at)));
        let expected = run_job(&by_hand).unwrap();
        let got = shares(&result.program, &result.op_spans);
        let want = shares(&expected.program, &expected.op_spans);
        same_turning(&got, &want, &format!("{mirror:?}"));
        // It is cut where the mirrored drawing is: the part keeps its box.
        assert!(
            close(cut_box(&result.program), cut_box(&expected.program)),
            "{mirror:?}"
        );
        // Not by chance: mirrored, the program turns as the unmirrored one does, while
        // the plain mirror of it would turn the other way.
        let plain = run_job(&every_direction(None)).unwrap();
        let unmirrored = shares(&plain.program, &plain.op_spans);
        same_turning(&got, &unmirrored, &format!("{mirror:?} as drawn"));
        // The G-code is a checked program.
        assert!(!result.files.is_empty());
    }
}

#[test]
fn each_copy_of_a_mirrored_part_is_mirrored_in_place() {
    let copies = Copies {
        columns: 2,
        rows: 1,
        dx: 230.0,
        dy: 0.0,
    };
    let mirror = Mirror::LeftRight;
    let mut job = every_direction(None);
    let at = axis(&job, mirror);
    job.parts[0].mirror = Some(mirror);
    job.set_grid_here(1, &copies);
    job.parts[0].order = InstanceOrder::ByInstance;
    let mut by_hand = every_direction(Some((turn_of(mirror), at)));
    by_hand.set_grid_here(1, &copies);
    by_hand.parts[0].order = InstanceOrder::ByInstance;
    for job in [&mut job, &mut by_hand] {
        job.setups[0].stock.fit = None;
        job.setups[0].stock.x = 520.0;
    }
    let result = run_job(&job).unwrap();
    let expected = run_job(&by_hand).unwrap();
    let got = shares(&result.program, &result.op_spans);
    let want = shares(&expected.program, &expected.op_spans);
    assert_eq!(got.len(), want.len());
    for (g, w) in got.iter().zip(&want) {
        assert!((g.1 - w.1).abs() < 0.01, "{g:?} {w:?}");
    }
    assert!(close(cut_box(&result.program), cut_box(&expected.program)));
}

#[test]
fn a_mirrored_part_on_a_back_turns_twice() {
    // Mirrored on a board turned over: the two mirrors cancel, so the operations turn on
    // the board as the part as drawn does on a front.
    for (turn, mirror) in [
        (Turn::LeftRight, Mirror::LeftRight),
        (Turn::LeftRight, Mirror::TopBottom),
    ] {
        let front = every_direction(None);
        let (two, back, _) = add_back_side(&front, 1, turn, None).unwrap();
        let mut two = two;
        let ids: Vec<u64> = two.parts.iter().map(|p| p.id).collect();
        for id in ids {
            two.move_part_to(id, back);
        }
        for part in &mut two.parts {
            part.mirror = Some(mirror);
        }
        two.follow_thickness();
        let one = two.for_setup(back);
        let result = run_job(&one).unwrap();
        let on_board = chipload_pipeline::on_the_board(&one, &result.program);
        let got = shares(&on_board, &result.op_spans);
        let plain = run_job(&every_direction(None)).unwrap();
        let want = shares(&plain.program, &plain.op_spans);
        same_turning(&got, &want, &format!("{turn:?} {mirror:?}"));
    }
}

#[test]
fn a_surface_pass_faces_the_board_not_the_mirrored_part() {
    let mut job = every_direction(None);
    job.parts[0].mirror = Some(Mirror::LeftRight);
    let at = chipload_pipeline::surface::add_surface(&mut job, None, Some(0.5), None).unwrap();
    assert!(matches!(
        job.operations[at],
        chipload_job::Operation::Surface(_)
    ));
    assert_eq!(job.mirror_of(&job.operations[at]), None);
    let other = (at + 1) % job.operations.len();
    assert!(job.mirror_of(&job.operations[other]).is_some());
    run_job(&job).unwrap();
}

#[test]
fn a_mirror_is_saved_and_read_back_and_left_out_when_none() {
    let mut job = every_direction(None);
    let plain = serde_json::to_value(&job).unwrap();
    assert!(plain["parts"][0].get("mirror").is_none());
    job.parts[0].mirror = Some(Mirror::TopBottom);
    let value = serde_json::to_value(&job).unwrap();
    assert_eq!(value["parts"][0]["mirror"], "top_bottom");
    let back = chipload_job::parse_job(value, "test").unwrap();
    assert_eq!(back.parts[0].mirror, Some(Mirror::TopBottom));
}

#[test]
fn a_mirrored_parts_text_is_said_to_read_backwards() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus/jobs/v35/sign_frame.chipload");
    let mut job = chipload_job::load_job(&path).unwrap();
    let said = |job: &Job| {
        run_job(job)
            .unwrap()
            .facts
            .iter()
            .filter(|n| n.message.id == "pipeline-mirrored-text")
            .count()
    };
    assert_eq!(said(&job), 0, "as drawn");
    for part in &mut job.parts {
        part.mirror = Some(Mirror::LeftRight);
    }
    assert_eq!(said(&job), job.drawing.texts.len(), "once per text");
}

#[test]
fn the_setup_sheet_says_a_part_is_mirrored() {
    let mut job = every_direction(None);
    let sheet = |job: &Job| {
        let result = run_job(job).unwrap();
        let info = chipload_pipeline::sheet::SheetInfo {
            project: "test".into(),
            written_by: "chipload test".into(),
            units: chipload_job::Units::Mm,
            language: chipload_i18n::Language::En,
            parts: Vec::new(),
        };
        chipload_pipeline::sheet::setup_sheet(job, &result, std::path::Path::new("t.nc"), &info)
    };
    assert!(!sheet(&job).contains("cut mirrored"));
    job.parts[0].mirror = Some(Mirror::LeftRight);
    assert!(sheet(&job).contains("Part 1 is cut mirrored left to right"));
}

#[test]
fn switching_an_operation_off_doesnt_move_the_rest() {
    // The outline (operation 3) sets the part's box; cut the pocket again alone and it
    // must land where it did with the outline on.
    let mut job = every_direction(None);
    job.parts[0].mirror = Some(Mirror::LeftRight);
    let pocket_box = |job: &Job| {
        let result = run_job(job).unwrap();
        let span = result.op_spans.iter().find(|s| s.operation == 0).unwrap();
        chipload_toolpath::plan::cut_extent(&result.program.moves, std::slice::from_ref(span), None)
            .unwrap()
    };
    let before = pocket_box(&job);
    for op in job.operations.iter_mut().skip(1) {
        op.set_enabled(false);
    }
    assert!(close(pocket_box(&job), before));
}

#[test]
fn a_mirrored_parts_bones_mind_the_other_parts_where_they_are() {
    let warned = |job: &Job| {
        run_job(job)
            .unwrap()
            .notes
            .iter()
            .any(|n| n.message.id.starts_with("toolpath-corner-cuts"))
    };
    let mut job = common::bones_beside_another_part([-0.6, 1.5]);
    // As drawn, the circle is a thin web beside the hole's corner.
    assert!(warned(&job), "the circle is close as drawn");
    // Mirrored about X 55, the hole lies at X 70 … 110, far from the circle: nothing to
    // warn of, though planned as drawn the circle would seem to be beside it.
    job.parts[0].mirror = Some(Mirror::LeftRight);
    assert!(!warned(&job), "mirrored, the circle is far away");
}
