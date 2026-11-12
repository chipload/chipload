// SPDX-License-Identifier: GPL-3.0-or-later
//! Two-sided work (docs/specs/two-sided.md): a back setup is the same board as its front,
//! turned over.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

mod common;

use chipload_job::{BackOf, Job, Register, StockOrigin, Turn, WorkZero, load_job};
use chipload_pipeline::setups::{
    BoardChange, add_back_side, move_part_to_setup, remove_setup, set_board,
};
use common::{every_direction, loops, turning};

/// Two parts: "simple part" (operations 1-3) and "part B" (operation 4), one setup.
fn sample() -> Job {
    load_job(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/corpus/jobs/v22/two_parts.chipload"),
    )
    .unwrap()
}

/// The sample with a back side, setup 2, holding part B.
fn with_back() -> Job {
    let (job, id, _) = add_back_side(&sample(), 1, Turn::LeftRight, None).unwrap();
    assert_eq!(id, 2);
    move_part_to_setup(&job, 2, 2).unwrap()
}

#[test]
fn a_back_side_shares_its_fronts_board_and_zero() {
    let job = with_back();
    let back = job.setup(2).unwrap();
    assert_eq!(back.name, "Setup 1 back");
    assert_eq!(
        back.back_of,
        Some(BackOf {
            setup: 1,
            turn: Turn::LeftRight,
            register: Register::Pins,
        })
    );
    assert_eq!(back.stock, job.setup(1).unwrap().stock);
    assert_eq!(back.work_zero, job.setup(1).unwrap().work_zero);
    // The front changes, the back follows wherever a project is read or changed.
    let mut changed = job.clone();
    changed.setups[0].stock.thickness = 18.0;
    changed.setups[0].work_zero = WorkZero::Point { x: 3.0, y: 4.0 };
    // Set on the back by hand: not its own, so it goes.
    changed.setups[1].stock.thickness = 5.0;
    changed.follow_thickness();
    assert_eq!(changed.setup(2).unwrap().stock.thickness, 18.0);
    assert_eq!(
        changed.setup(2).unwrap().work_zero,
        WorkZero::Point { x: 3.0, y: 4.0 }
    );
}

#[test]
fn a_board_that_fits_fits_both_sides() {
    let mut job = with_back();
    for setup in &mut job.setups {
        setup.stock.origin = StockOrigin::Drawing;
        setup.stock.fit = Some(10.0);
    }
    job.follow_thickness();
    let front = job.for_setup(1).stock_bounds();
    let back = job.for_setup(2).stock_bounds();
    assert_eq!(front, back, "one board");
    // Unlinked, each setup fits its own part, so the front's board is smaller.
    let own = set_board(
        &job,
        2,
        BoardChange {
            back_of: Some(None),
            ..BoardChange::default()
        },
    )
    .unwrap();
    let alone = own.for_setup(1).stock_bounds();
    let area = |((x0, y0), (x1, y1)): ((f64, f64), (f64, f64))| (x1 - x0) * (y1 - y0);
    assert!(area(alone) < area(front), "{alone:?} {front:?}");
}

#[test]
fn one_board_has_one_back_and_a_back_is_not_turned_again() {
    let job = with_back();
    // A second back of setup 1.
    assert!(add_back_side(&job, 1, Turn::TopBottom, None).is_err());
    // The back of the back.
    assert!(add_back_side(&job, 2, Turn::LeftRight, None).is_err());
    // A setup turned over onto itself.
    let itself = BoardChange {
        back_of: Some(Some(2)),
        ..BoardChange::default()
    };
    assert!(set_board(&job, 2, itself).is_err());
    // A turn for a setup that is not a back.
    let turn = BoardChange {
        turn: Some(Turn::TopBottom),
        ..BoardChange::default()
    };
    let err = set_board(&job, 1, turn).unwrap_err();
    assert!(err.to_string().contains("not the back"), "{err}");
    // A back's turn and way of locating change; its front stays.
    let changed = set_board(
        &job,
        2,
        BoardChange {
            turn: Some(Turn::TopBottom),
            register: Some(Register::Corner),
            ..BoardChange::default()
        },
    )
    .unwrap();
    assert_eq!(
        changed.setup(2).unwrap().back_of,
        Some(BackOf {
            setup: 1,
            turn: Turn::TopBottom,
            register: Register::Corner,
        })
    );
}

#[test]
fn removing_the_front_leaves_the_back_its_board() {
    let mut job = with_back();
    job.setups[0].stock.thickness = 15.0;
    job.follow_thickness();
    let (job, lines) = remove_setup(&job, 1, true).unwrap();
    assert!(
        lines.iter().any(|l| l.contains("board of its own")),
        "{lines:?}"
    );
    let back = job.setup(2).unwrap();
    assert_eq!(back.back_of, None);
    assert_eq!(back.stock.thickness, 15.0);
}

#[test]
fn a_back_side_is_saved_and_read_back() {
    let job = with_back();
    let dir = std::env::temp_dir().join(format!("chipload-two-sided-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("back.chipload");
    chipload_job::save_job(&job, &path).unwrap();
    let read = load_job(&path).unwrap();
    assert_eq!(
        read.setup(2).unwrap().back_of,
        job.setup(2).unwrap().back_of
    );
    assert_eq!(read.schema_version, 36);
    std::fs::remove_dir_all(&dir).unwrap();
}

// -- the cut direction on the back face (docs/specs/two-sided.md §4)

#[test]
fn climb_stays_climb_on_the_back_face() {
    for turn in [Turn::LeftRight, Turn::TopBottom] {
        let front = every_direction(None);
        // The same shapes cut from the back: the whole part in setup 2, the back of 1.
        let (two, back, _) = add_back_side(&front, 1, turn, None).unwrap();
        let mut two = two;
        for part in &mut two.parts {
            part.setup = back;
        }
        two.follow_thickness();
        let one = two.for_setup(back);
        let ((x0, y0), (x1, y1)) = one.stock_bounds();
        let axis = match turn {
            Turn::LeftRight => (x0 + x1) / 2.0,
            Turn::TopBottom => (y0 + y1) / 2.0,
        };
        let result = chipload_pipeline::run_job(&one).unwrap();
        let on_board = chipload_pipeline::on_the_board(&one, &result.program);
        // The shapes drawn mirrored by hand, cut from the front.
        let by_hand = every_direction(Some((turn, axis)));
        let expected = chipload_pipeline::run_job(&by_hand).unwrap();
        let got = turning(&loops(&on_board, &result.op_spans));
        let want = turning(&loops(&expected.program, &expected.op_spans));
        assert_eq!(got.len(), 5, "{turn:?}: every operation cut: {got:?}");
        assert_eq!(got.len(), want.len(), "{turn:?}");
        // The share of each operation's cut area that runs counter-clockwise: 1 or 0 for
        // a direction kept throughout. It must be the same on the back as by hand.
        let share = |(_, ccw, cw): &(usize, f64, f64)| ccw / (ccw - cw);
        assert_eq!(got.len(), 5, "{turn:?}: every operation cut: {got:?}");
        assert_eq!(got.len(), want.len(), "{turn:?}");
        for (g, w) in got.iter().zip(&want) {
            assert_eq!(g.0, w.0);
            assert!(g.1 - g.2 > 1.0, "{turn:?} operation {} cuts", g.0);
            // The same loops: the areas agree but for where each ring starts, which moves
            // its ramp's overlap (a follow path starts on a line on one side, an arc on the
            // other: 6 %).
            let total = |t: &(usize, f64, f64)| t.1 - t.2;
            assert!(
                (total(g) - total(w)).abs() < 0.1 * total(w),
                "{turn:?} operation {}: back {g:?}, by hand {w:?}",
                g.0
            );
            assert!(
                (share(g) - share(w)).abs() < 0.01,
                "{turn:?} operation {}: back {g:?}, by hand {w:?}",
                g.0
            );
        }
        // Not by chance: the program as planned, before the mirror, turns the other way.
        let planned = turning(&loops(&result.program, &result.op_spans));
        for (p, w) in planned.iter().zip(&want) {
            assert!(
                (share(p) - (1.0 - share(w))).abs() < 0.01,
                "{turn:?} operation {}: planned {p:?}",
                p.0
            );
        }
        // And the G-code is the program on the board: its spindle is the machine's, M3.
        let gcode: String = result.files.iter().map(|f| f.gcode.clone()).collect();
        assert!(gcode.contains("M3") && !gcode.contains("M4"), "{turn:?}");
    }
}

#[test]
fn the_back_is_as_thick_as_the_front_leaves_the_board() {
    // docs/specs/two-sided.md §5: the corpus sample surfaces its 10 mm board 0.5 mm on the
    // front, and cuts its outline through from the back.
    let job = load_job(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/corpus/jobs/v36/two_sided.chipload"),
    )
    .unwrap();
    assert_eq!(job.setup(1).unwrap().stock.thickness, 10.0);
    assert_eq!(job.setup(2).unwrap().stock.thickness, 9.5);
    let outline = |job: &Job| {
        job.operations
            .iter()
            .find(|op| op.name() == "Profile 2 (outline)")
            .unwrap()
            .depths()
            .1
    };
    // Through the 9.5 mm, 0.5 mm into the spoilboard.
    assert_eq!(outline(&job), 10.0);
    // Without the surfacing the back is the whole board again.
    let mut whole = job.clone();
    whole.operations.remove(0);
    whole.follow_thickness();
    assert_eq!(whole.setup(2).unwrap().stock.thickness, 10.0);
    assert_eq!(outline(&whole), 10.5);
    // The back's program, checked against its own 9.5 mm board, goes no deeper.
    let back = chipload_pipeline::run_job(&job.for_setup(2)).unwrap();
    let deepest = back
        .program
        .moves
        .iter()
        .filter_map(|m| m.z)
        .fold(f64::INFINITY, f64::min);
    assert!((deepest + 10.0).abs() < 1e-9, "{deepest}");
}

// -- dowel holes (docs/specs/two-sided.md §6)

/// The corpus sample: a 10 mm board fitting its shapes with a 10 mm margin, X 40-260 and
/// Y 40-180, its back turned left to right, so about X 150.
fn sample_two_sided() -> Job {
    load_job(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/corpus/jobs/v36/two_sided.chipload"),
    )
    .unwrap()
}

#[test]
fn dowel_holes_sit_on_the_turn_axis_in_the_margin() {
    use chipload_job::{Geometry, Operation};
    use chipload_pipeline::dowels::{Dowels, add_dowels, dowel_centres};
    let mut job = sample_two_sided();
    // The sample has its dowels: take them out to add them again.
    let at = job.operations.iter().position(|op| op.is_dowels()).unwrap();
    job.operations.remove(at);
    job.drawing.shapes.retain(|s| s.layer != "dowels");
    let (x0, x1) = (
        job.for_setup(1).stock_bounds().0.0,
        job.for_setup(1).stock_bounds().1.0,
    );
    // Beyond the outline's profile, which the 6 mm end mill cuts out to Y 44 and Y 176,
    // centred in what is left of the 16 mm margin.
    assert_eq!(
        dowel_centres(&job, 2, 8.0).unwrap(),
        [(150.0, 39.0), (150.0, 181.0)]
    );
    assert_eq!((x0 + x1) / 2.0, 150.0);
    let dowels = Dowels {
        zero: true,
        ..Dowels::default()
    };
    let (with, index) = add_dowels(&job, 2, dowels, chipload_i18n::Language::En).unwrap();
    // After the surfacing, before everything else of the front.
    assert_eq!(index, 1);
    let Operation::Holes(h) = &with.operations[1] else {
        panic!("{:?}", with.operations[1]);
    };
    assert!(h.dowels);
    assert_eq!(
        (h.through, h.end_depth),
        (Some(8.0), 18.0),
        "through 10 mm and 8 more"
    );
    assert_eq!(h.name, "Dowel holes");
    let circles: Vec<[f64; 2]> = with
        .drawing
        .shapes
        .iter()
        .filter(|s| s.layer == "dowels")
        .map(|s| match s.geometry {
            Geometry::Circle { center, radius } => {
                assert_eq!(radius, 4.0);
                center
            }
            _ => panic!("{s:?}"),
        })
        .collect();
    assert_eq!(circles, [[150.0, 39.0], [150.0, 181.0]]);
    // The front's zero is pin 1, and the back's with it.
    let pin = chipload_job::WorkZero::Point { x: 150.0, y: 39.0 };
    assert_eq!(
        (
            with.setup(1).unwrap().work_zero,
            with.setup(2).unwrap().work_zero
        ),
        (pin, pin)
    );
    // The fitted board grows round them, its margin beyond each (the user, 2026-11-12).
    let ((_, y0), (_, y1)) = with.for_setup(1).stock_bounds();
    assert_eq!((y0, y1), (39.0 - 4.0 - 16.0, 181.0 + 4.0 + 16.0));
    assert_eq!(
        with.for_setup(1).stock_bounds().0.0,
        job.for_setup(1).stock_bounds().0.0,
        "across the axis: as it was"
    );
    // The front passes the checker, its holes 8 mm into the spoilboard and nothing else.
    let front = chipload_pipeline::run_job(&with.for_setup(1)).unwrap();
    assert!(
        front.check_notes.iter().any(|n| n
            .message
            .contains("deeper into dowel holes, down to Z-18.000")),
        "{:?}",
        front.check_notes
    );
    // Only one pair per board.
    assert!(add_dowels(&with, 2, dowels, chipload_i18n::Language::En).is_err());
    // The 10 mm margin a board fits with by default: the profile leaves 4 mm of it, and
    // the board grows round the holes instead of refusing them (the user, 2026-11-12).
    let mut narrow = job.clone();
    for setup in &mut narrow.setups {
        setup.stock.fit = Some(10.0);
    }
    narrow.follow_thickness();
    let (made, _) = add_dowels(&narrow, 2, dowels, chipload_i18n::Language::En).unwrap();
    let ((_, y0), (_, y1)) = made.for_setup(1).stock_bounds();
    assert_eq!((y0, y1), (39.0 - 4.0 - 10.0, 181.0 + 4.0 + 10.0));
}

/// The sample with its dowel circles moved to (150, y) and (150, 220 − y).
fn dowels_at(y: f64) -> Job {
    let mut job = sample_two_sided();
    let mut ends = [y, 220.0 - y].into_iter();
    for shape in &mut job.drawing.shapes {
        if shape.layer == "dowels"
            && let chipload_job::Geometry::Circle { center, .. } = &mut shape.geometry
        {
            center[1] = ends.next().unwrap();
        }
    }
    job
}

#[test]
fn the_checker_refuses_a_back_that_cuts_into_a_pin() {
    // The sample as it is: the back's cuts stay clear of the pins.
    let job = sample_two_sided();
    chipload_pipeline::run_job(&job.for_setup(2)).unwrap();
    // A hole moved to Y 45, under the outline's profile, which the back cuts.
    let err = chipload_pipeline::run_job(&dowels_at(45.0).for_setup(2)).unwrap_err();
    assert!(
        err.to_string()
            .contains("into the dowel pin at (0.000 mm, 6.000 mm), which stands in the board"),
        "{err}"
    );
    // The front doesn't have pins in it: it may cut there (and makes the holes).
    chipload_pipeline::run_job(&dowels_at(45.0).for_setup(1)).unwrap();
}

#[test]
fn a_front_cut_into_a_dowel_hole_after_it_is_made_is_noted() {
    let note = "cuts into a dowel hole after it is made";
    let notes = |job: &Job| -> Vec<(Option<usize>, String)> {
        chipload_pipeline::run_job(&job.for_setup(1))
            .unwrap()
            .notes
            .iter()
            .filter(|n| n.text.contains(note))
            .map(|n| (n.operation, n.text.clone()))
            .collect()
    };
    let job = sample_two_sided();
    assert!(notes(&job).is_empty(), "{:?}", notes(&job));
    // Holes moved along the axis to Y 102 and Y 118: the front's inside profile
    // (operation 4, after the dowel holes) cuts the slot to Y 100 and the line at Y 120.
    let found = notes(&dowels_at(102.0));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, Some(3));
}

#[test]
fn a_dowel_off_the_axis_or_without_a_back_is_refused() {
    use chipload_job::Geometry;
    let job = sample_two_sided();
    let problems =
        |job: &Job| -> Vec<String> { job.validate().into_iter().map(|p| p.to_string()).collect() };
    assert!(problems(&job).is_empty(), "{:?}", problems(&job));
    // A circle moved 1 mm off the line the board turns about.
    let mut off = job.clone();
    let shape = off
        .drawing
        .shapes
        .iter_mut()
        .find(|s| s.layer == "dowels")
        .unwrap();
    if let Geometry::Circle { center, .. } = &mut shape.geometry {
        center[0] += 1.0;
    }
    assert!(
        problems(&off)
            .iter()
            .any(|p| p.contains("off the line the board turns about")),
        "{:?}",
        problems(&off)
    );
    // The board with no back any more.
    let own = chipload_pipeline::setups::set_board(
        &job,
        2,
        BoardChange {
            back_of: Some(None),
            ..BoardChange::default()
        },
    );
    assert!(
        own.unwrap_err()
            .to_string()
            .contains("has a back side, and setup 1 has none")
    );
}

#[test]
fn text_on_the_back_is_said_to_read_mirrored() {
    // docs/specs/two-sided.md §4, decision 9: a plain note, nothing mirrored.
    use chipload_pipeline::fonts::BuiltinOnly;
    use chipload_pipeline::new_ops::{NewRequest, OpKind, insert, new_operations};
    use chipload_pipeline::texts::{NewText, add_text};
    let job = sample_two_sided();
    let mut new = NewText::new("HI", 40.0);
    new.x = 110.0;
    new.y = 110.0;
    new.layer = "words";
    let (mut job, _) = add_text(&job, new, &BuiltinOnly).unwrap();
    // A groove along the letters in the back's part, the active one.
    let contours = chipload_pipeline::load_drawing(&job).unwrap().contours;
    let request = NewRequest {
        kind: OpKind::Path,
        select: chipload_job::Selector::on_layer("words"),
        tool: None,
    };
    let made = new_operations(&job, &request, &contours, None, chipload_i18n::Language::En);
    insert(&mut job, made);
    let back = chipload_pipeline::run_job(&job.for_setup(2)).unwrap();
    let said: Vec<String> = back.facts.iter().map(|f| f.message.english()).collect();
    assert!(
        said.iter()
            .any(|f| f.contains("carves the text \"HI\" on the back")),
        "{said:?}"
    );
    // The same text on the front reads right: nothing to say.
    let mut front = job.clone();
    let last = front.operations.len() - 1;
    front.operations[last].set_part(1);
    front.follow_thickness();
    let cut = chipload_pipeline::run_job(&front.for_setup(1)).unwrap();
    assert!(
        !cut.facts
            .iter()
            .any(|f| f.message.english().contains("carves the text"))
    );
}

// -- the simulation of both sides (docs/specs/two-sided.md §10)

#[test]
fn the_back_is_simulated_from_below_on_the_fronts_board() {
    use chipload_pipeline::simulate::simulate;
    let job = sample_two_sided();
    let cancel = chipload_pipeline::Cancel::new();
    let sim_of = |job: &Job, setup: u64| {
        let one = job.for_setup(setup);
        let result = chipload_pipeline::run_job(&one).unwrap();
        // The whole project, its active setup the one simulated, as the app and `simulate`
        // pass it.
        let mut whole = job.clone();
        whole.active_setup = Some(setup);
        simulate(&whole, &result.program, &result.groups, Some(1.0), &cancel).unwrap()
    };
    // The front alone: nothing from below, as always.
    let front = sim_of(&job, 1);
    assert!(front.floors.iter().all(|f| *f == front.bottom as f32));
    assert_eq!(front.numbers.through_cm2, None);
    // The back on it: its outline cut from below goes through, the front's pocket stays.
    let back = sim_of(&job, 2);
    assert!(back.removed_more_than(&front));
    let through = back.numbers.through_cm2.unwrap();
    assert!(through > 10.0, "{through}");
    assert_eq!(
        back.numbers.meet_cm2,
        Some(0.0),
        "no cut of one side reaches the other's"
    );
    // The front's pocket of the 30 mm circle 6 mm deep, and the back's 6 mm on the same
    // circle: 10 mm less the 0.5 surfaced is 9.5, so they meet. Not in the 6 mm ring the
    // front's inside profile cuts through alone: within 15 - 6 = 9 mm of the centre.
    let mut meeting = job.clone();
    let pocket = meeting
        .operations
        .iter()
        .position(|op| op.name() == "Pocket 1")
        .unwrap();
    let mut from_back = meeting.operations[pocket].clone();
    if let chipload_job::Operation::Pocket(p) = &mut meeting.operations[pocket] {
        p.end_depth = 6.0;
    }
    if let chipload_job::Operation::Pocket(p) = &mut from_back {
        p.end_depth = 6.0;
        p.part = 2;
        p.name = "Pocket from the back".into();
    }
    meeting.operations.push(from_back);
    meeting.follow_thickness();
    let met = sim_of(&meeting, 2);
    let circle = std::f64::consts::PI * 9.0 * 9.0 / 100.0;
    let meet = met.numbers.meet_cm2.unwrap();
    assert!((meet - circle).abs() < 0.05 * circle, "{meet} vs {circle}");
    assert!(
        met.lines().iter().any(|l| l.contains("The two sides meet")),
        "{:?}",
        met.lines()
    );
}

trait Removed {
    fn removed_more_than(&self, other: &Self) -> bool;
}

impl Removed for chipload_pipeline::simulate::Simulation {
    fn removed_more_than(&self, other: &Self) -> bool {
        self.numbers.removed_cm3 > other.numbers.removed_cm3 + 1.0
    }
}

#[test]
fn a_back_surfaced_over_its_pins_is_allowed_with_a_warning() {
    // docs/specs/two-sided.md §6 (the user, 2026-11-08: "allow surfacing above pins with a
    // warning"): the back faced 0.5 mm over the whole board, pins and all.
    let mut back = sample_two_sided().for_setup(2);
    let index = chipload_pipeline::surface::add_surface(&mut back, None, Some(0.5), None).unwrap();
    assert_eq!(back.surface_over_pins(), Some((index, 0.5)));
    let result = chipload_pipeline::run_job(&back).unwrap();
    let warning: Vec<_> = result
        .notes
        .iter()
        .filter(|n| n.text.contains("over the dowel pins"))
        .collect();
    assert_eq!(warning.len(), 1, "{:?}", result.notes);
    assert_eq!(warning[0].operation, Some(index));
    assert!(
        warning[0]
            .text
            .contains("set the pins at least 0.5 mm below the back face"),
        "{}",
        warning[0].text
    );
    // The sheet's turning step says so.
    let info = chipload_pipeline::sheet::SheetInfo {
        project: "two_sided".into(),
        written_by: "chipload (test)".into(),
        units: chipload_job::Units::Mm,
        language: chipload_i18n::Language::En,
        parts: Vec::new(),
    };
    let sheet = chipload_pipeline::sheet::setup_sheet(&back, &result, Path::new("back.nc"), &info);
    assert!(
        sheet.contains("Tap the pins down at least 0.5 mm below the back face"),
        "the sheet's turning step"
    );
    // Without a surface pass, no warning; the front never has one about pins.
    let plain = chipload_pipeline::run_job(&sample_two_sided().for_setup(2)).unwrap();
    assert!(
        plain
            .notes
            .iter()
            .all(|n| !n.text.contains("over the dowel pins"))
    );
    let mut front = sample_two_sided().for_setup(1);
    chipload_pipeline::surface::add_surface(&mut front, None, Some(0.5), None).unwrap();
    assert_eq!(front.surface_over_pins(), None);
}

#[test]
fn missing_dowel_holes_are_a_warning_and_remove_takes_them_out() {
    use chipload_pipeline::dowels::{missing_dowels_warning, remove_dowels};
    // docs/specs/two-sided-wizard.md §3
    let job = sample_two_sided();
    let at = job.dowels_of(1).expect("the sample has dowels");
    assert!(!job.missing_dowels(2));
    assert!(missing_dowels_warning(&job).is_none());
    let circles = job
        .drawing
        .shapes
        .iter()
        .filter(|s| s.layer == "dowels")
        .count();
    assert_eq!(circles, 2);
    let removed = remove_dowels(&job, 2).unwrap();
    assert_eq!(removed.operations.len(), job.operations.len() - 1);
    assert!(removed.operations.iter().all(|op| !op.is_dowels()));
    assert!(removed.drawing.shapes.iter().all(|s| s.layer != "dowels"));
    assert!(removed.missing_dowels(2));
    // The warning, whichever side is active, and in a run's notes.
    for active in [1, 2] {
        let mut one = removed.clone();
        one.active_setup = Some(active);
        let note = missing_dowels_warning(&one).expect("the warning");
        assert!(note.text.contains("has no dowel holes"), "{}", note.text);
    }
    // Removing again is refused; switched off counts as missing too.
    assert!(remove_dowels(&removed, 2).is_err());
    let mut off = job.clone();
    off.operations[at].set_enabled(false);
    assert!(off.missing_dowels(2));
    // A fence corner needs no holes.
    let mut corner = removed.clone();
    corner.setups[1].back_of.as_mut().unwrap().register = Register::Corner;
    assert!(!corner.missing_dowels(2));
    assert!(missing_dowels_warning(&corner).is_none());
}

#[test]
fn a_back_side_with_its_pins_is_made_and_changed_in_one_step() {
    use chipload_job::{Geometry, Operation, ToolType};
    use chipload_pipeline::dowels::pin_tool;
    use chipload_pipeline::two_sided::{
        PinsRequest, TwoSidedRequest, make_two_sided, two_sided_layout,
    };
    // docs/specs/two-sided-wizard.md §4, §6
    let lang = chipload_i18n::Language::En;
    let sample = sample_two_sided();
    let pins = PinsRequest {
        diameter: 8.0,
        depth: 8.0,
        zero: true,
        tool: None,
    };
    // A front without a back: the back, its dowel holes and the zero at the first pin.
    let (single, _) = remove_setup(&remove_dowels_of(&sample), 2, true).unwrap();
    assert!(single.back_of(1).is_none());
    let request = TwoSidedRequest {
        front: 1,
        turn: Turn::LeftRight,
        register: Register::Pins,
        pins: Some(pins),
    };
    let (made, what) = make_two_sided(&single, &request, lang).unwrap();
    assert!(what.added);
    assert_eq!(made.active_setup_id(), Some(what.back));
    assert_eq!(made.front_of(what.back).unwrap().id, 1);
    let at = what.dowels.unwrap();
    assert!(made.operations[at].is_dowels());
    assert!(!made.missing_dowels(what.back));
    assert!(matches!(
        made.setup(1).unwrap().work_zero,
        chipload_job::WorkZero::Point { .. }
    ));
    // The tool: the project's end mill bores 8 mm pins (no drill of that size).
    let mill = pin_tool(&made, 8.0).unwrap();
    assert_eq!(
        made.tools.iter().find(|t| t.id == mill).unwrap().kind,
        ToolType::Endmill
    );
    assert_eq!(made.operations[at].tool(), mill);
    // Changed: turned top to bottom, 6 mm pins; the holes are made again, not doubled.
    let request = TwoSidedRequest {
        turn: Turn::TopBottom,
        pins: Some(PinsRequest {
            diameter: 6.0,
            ..pins
        }),
        ..request
    };
    let layout = two_sided_layout(&made, &request).unwrap();
    assert!(layout.fits(), "{layout:?}");
    let (changed, what) = make_two_sided(&made, &request, lang).unwrap();
    assert!(!what.added);
    let link = changed.setup(what.back).unwrap().back_of.unwrap();
    assert_eq!(link.turn, Turn::TopBottom);
    assert_eq!(
        changed
            .operations
            .iter()
            .filter(|op| op.is_dowels())
            .count(),
        1
    );
    let radii: Vec<f64> = changed
        .drawing
        .shapes
        .iter()
        .filter(|s| s.layer == "dowels")
        .map(|s| match s.geometry {
            Geometry::Circle { radius, .. } => radius,
            _ => panic!("{s:?}"),
        })
        .collect();
    assert_eq!(radii, [3.0, 3.0]);
    // On the turn axis, now horizontal.
    let y_mid = (layout.board.0.1 + layout.board.1.1) / 2.0;
    assert!(layout.centres.iter().all(|c| (c.1 - y_mid).abs() < 1e-9));
    // A 6 mm drill is chosen over the end mill when there is one.
    let mut drilled = changed.clone();
    let mut drill = drilled.tools[0].clone();
    drill.id = 9;
    drill.kind = ToolType::Drill;
    drill.diameter = 6.0;
    drilled.tools.push(drill);
    assert_eq!(pin_tool(&drilled, 6.0), Some(9));
    let Operation::Holes(_) = &changed.operations[what.dowels.unwrap()] else {
        panic!("a holes operation");
    };
}

/// The user, 2026-11-12: "we should not be so strict" about the pins. A fitted board grows
/// round them; on a board of its own size they are made anyway, with a warning
/// (docs/specs/two-sided.md §6).
#[test]
fn wide_pins_grow_a_fitted_board_and_only_warn_on_one_of_its_own_size() {
    use chipload_pipeline::dowels::{dowel_layout, remove_dowels};
    use chipload_pipeline::two_sided::{
        PinsRequest, TwoSidedRequest, make_two_sided, two_sided_layout,
    };
    let lang = chipload_i18n::Language::En;
    let job = remove_dowels(&sample_two_sided(), 2).unwrap();
    let wide = TwoSidedRequest {
        front: 1,
        turn: Turn::LeftRight,
        register: Register::Pins,
        pins: Some(PinsRequest {
            diameter: 20.0,
            depth: 8.0,
            zero: false,
            tool: None,
        }),
    };
    let layout = two_sided_layout(&job, &wide).unwrap();
    assert!(layout.fits() && layout.tight().is_none());
    let (made, _) = make_two_sided(&job, &wide, lang).unwrap();
    // The board grew round the pins, keeping its margin beyond them, along the axis only.
    let before = job.for_setup(1).stock_bounds();
    let ((x0, y0), (x1, y1)) = made.for_setup(1).stock_bounds();
    assert_eq!(
        (x0, x1),
        (before.0.0, before.1.0),
        "across the axis: as it was"
    );
    let margin = job.setup(1).unwrap().stock.fit.unwrap();
    let pins: Vec<(f64, f64)> = made
        .drawing
        .shapes
        .iter()
        .filter(|s| s.layer == "dowels")
        .filter_map(|s| match s.geometry {
            chipload_job::Geometry::Circle { center, radius } => Some((center[1], radius)),
            _ => None,
        })
        .collect();
    assert_eq!(pins.len(), 2);
    for (y, r) in pins {
        assert!(
            y - r - margin >= y0 - 1e-9 && y + r + margin <= y1 + 1e-9,
            "{y} in {y0}..{y1}"
        );
    }
    assert_eq!(
        dowel_layout(&made, 2, 20.0).unwrap().board,
        ((x0, y0), (x1, y1))
    );
    // A board of its own size: made where they go, with a warning, not refused.
    let mut own = job.clone();
    for setup in &mut own.setups {
        setup.stock.fit = None;
    }
    own.follow_thickness();
    let layout = two_sided_layout(&own, &wide).unwrap();
    assert!(!layout.fits());
    assert!(
        layout
            .tight()
            .unwrap()
            .english()
            .contains("made there anyway")
    );
    assert!(make_two_sided(&own, &wide, lang).is_ok());
}

/// The sample without its dowel holes.
fn remove_dowels_of(job: &Job) -> Job {
    chipload_pipeline::dowels::remove_dowels(job, 2).unwrap()
}

#[test]
fn dowel_holes_are_placed_again_only_when_asked() {
    use chipload_job::Geometry;
    use chipload_pipeline::dowels::{dowel_centres, place_dowels_again};
    // docs/specs/two-sided-wizard.md §5, decision 10
    let job = sample_two_sided();
    let centres = |job: &Job| -> Vec<[f64; 2]> {
        job.drawing
            .shapes
            .iter()
            .filter(|s| s.layer == "dowels")
            .map(|s| match s.geometry {
                Geometry::Circle { center, .. } => center,
                _ => panic!("{s:?}"),
            })
            .collect()
    };
    assert_eq!(centres(&job), [[150.0, 39.0], [150.0, 181.0]]);
    // The outline cut on its line: the cuts reach less far, the holes stay where they were.
    let mut wider = job.clone();
    let outline = wider.operations.len() - 1;
    if let chipload_job::Operation::Profile(p) = &mut wider.operations[outline] {
        p.side = chipload_job::Side::On;
    }
    wider.follow_thickness();
    assert_eq!(centres(&wider), centres(&job));
    // Placed again: just beyond the cuts as they are now, the zero at the first pin with
    // them, the operation as it was.
    let placed = place_dowels_again(&wider, 2).unwrap();
    let now = dowel_centres(&wider, 2, 8.0).unwrap();
    assert_eq!(
        centres(&placed),
        [[now[0].0, now[0].1], [now[1].0, now[1].1]]
    );
    assert_ne!(centres(&placed), centres(&job));
    assert_eq!(
        placed.setup(1).unwrap().work_zero,
        WorkZero::Point {
            x: now[0].0,
            y: now[0].1
        }
    );
    let at = job.dowels_of(1).unwrap();
    assert_eq!(placed.operations[at], wider.operations[at]);
    // A zero elsewhere stays where it is.
    let mut elsewhere = wider.clone();
    elsewhere.setups[0].work_zero = WorkZero::Point { x: 1.0, y: 2.0 };
    elsewhere.follow_thickness();
    let placed = place_dowels_again(&elsewhere, 2).unwrap();
    assert_eq!(
        placed.setup(1).unwrap().work_zero,
        WorkZero::Point { x: 1.0, y: 2.0 }
    );
    // Without dowel holes there is nothing to place.
    let none = chipload_pipeline::dowels::remove_dowels(&job, 2).unwrap();
    assert!(place_dowels_again(&none, 2).is_err());
}

/// The user, 2026-11-10: with the back active, moving the front's dowel circles left the
/// window's run as it was, so the back's simulation kept the front's old holes. A back's
/// run reads its front's selections (docs/specs/edit-and-simulate.md §5).
#[test]
fn a_back_runs_again_when_its_fronts_cuts_change() {
    use chipload_job::Geometry;
    use chipload_pipeline::cut_inputs;
    let job = chipload_pipeline::setups::activate_setup(&sample_two_sided(), 2).unwrap();
    let before = cut_inputs(&job).unwrap();
    let mut moved = job.clone();
    for shape in moved
        .drawing
        .shapes
        .iter_mut()
        .filter(|s| s.layer == "dowels")
    {
        if let Geometry::Circle { center, .. } = &mut shape.geometry {
            center[1] += 1.0;
        }
    }
    assert_ne!(
        cut_inputs(&moved).unwrap(),
        before,
        "the front's holes moved"
    );
    // A shape no operation of either side selects still needs no new run.
    let mut drawn = job.clone();
    let mut extra = drawn
        .drawing
        .shapes
        .iter()
        .find(|s| s.layer == "dowels")
        .unwrap()
        .clone();
    extra.id = drawn.drawing.next_shape_id;
    extra.layer = "nothing cuts this".into();
    extra.geometry = Geometry::Circle {
        center: [400.0, 400.0],
        radius: 4.0,
    };
    drawn.drawing.next_shape_id += 1;
    drawn.drawing.shapes.push(extra);
    assert_eq!(cut_inputs(&drawn).unwrap(), before);
}
