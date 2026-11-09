// SPDX-License-Identifier: GPL-3.0-or-later
//! A sign in one step (docs/specs/sign-maker.md §3, §6): what *New sign…* makes, that it
//! cuts, and that it keeps cutting when the words change.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use chipload_i18n::Language;
use chipload_job::{Job, Operation, Side, Source};
use chipload_pipeline::fonts::BuiltinOnly;
use chipload_pipeline::frames::follow_frames;
use chipload_pipeline::sign::{SignRequest, SignStyle, add_sign};
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

fn made(request: &SignRequest) -> (Job, chipload_pipeline::sign::SignMade) {
    add_sign(&project(), request, &BuiltinOnly, None, Language::En).unwrap()
}

fn names(job: &Job, ops: &[usize]) -> Vec<String> {
    ops.iter()
        .map(|&i| job.operations[i].name().to_string())
        .collect()
}

#[test]
fn an_engraved_sign_is_letters_borders_edge_and_cut_out_in_that_order() {
    let mut request = SignRequest::new("THE NÄSLUNDS", 2, 1);
    request.edge = Some(2.0);
    let (job, sign) = made(&request);
    assert_eq!(
        names(&job, &sign.operations),
        ["Letters", "Borders", "Edge chamfer", "Cut out"]
    );
    // In order, in the sign's own part, which is active.
    assert!(sign.operations.windows(2).all(|w| w[0] < w[1]));
    assert_eq!(job.active_part, Some(sign.part));
    assert_eq!(job.part(sign.part).unwrap().name, "THE NÄSLUNDS");
    // A new project's empty part became the sign's.
    assert_eq!(job.parts.len(), 1);
    let ops: Vec<&Operation> = sign
        .operations
        .iter()
        .map(|&i| &job.operations[i])
        .collect();
    assert!(ops.iter().all(|op| op.part() == sign.part));
    let Operation::Vcarve(letters) = ops[0] else {
        panic!("{:?}", ops[0]);
    };
    assert_eq!(
        letters.select.source,
        Some(Source::Text(sign.text.unwrap()))
    );
    let vbit = job.tool(2).unwrap().clone();
    let Operation::Path(borders) = ops[1] else {
        panic!();
    };
    assert_eq!(borders.select.shapes, Some(sign.borders.clone()));
    // A groove 2 mm wide: its depth from the V-bit.
    assert!((vbit.vbit_width_at(borders.end_depth).unwrap() - 2.0).abs() < 1e-9);
    let Operation::Path(edge) = ops[2] else {
        panic!();
    };
    assert_eq!(edge.select.shapes, Some(vec![sign.outline]));
    // A 2 mm chamfer: a 4 mm groove centred on the outline.
    assert!((vbit.vbit_width_at(edge.end_depth).unwrap() - 4.0).abs() < 1e-9);
    let Operation::Profile(cut) = ops[3] else {
        panic!();
    };
    assert_eq!(cut.side, Side::Outside);
    assert!(cut.through.is_some() && cut.tabs.is_some());
    assert_eq!(cut.tool, 1);
    // The board fitted round it; the sign grouped; the frame follows the words.
    let stock = job.stock();
    assert_eq!(stock.fit, Some(15.0));
    assert_eq!(stock.thickness, 18.0);
    let group = &job.drawing.groups[0];
    assert_eq!(group.texts, vec![sign.text.unwrap()]);
    assert!(group.shapes.contains(&sign.outline));
    assert_eq!(job.drawing.frames[0].outline, sign.outline);
    // It cuts: every program passes the checker.
    let runs = run_setups(&job, None).unwrap();
    assert_eq!(runs.len(), 1);
}

#[test]
fn new_words_keep_every_operation_cutting_the_sign() {
    let (job, sign) = made(&SignRequest::new("OPEN", 2, 1));
    let mut changed = job.clone();
    changed.drawing.texts[0].text = "OPEN ALL DAY".into();
    let (changed, _) =
        chipload_pipeline::texts::remake(&job, &changed, sign.text.unwrap(), &BuiltinOnly).unwrap();
    let (changed, notes) = follow_frames(changed);
    assert!(notes.is_empty(), "{notes:?}");
    assert!(changed.drawing.frames[0].width > job.drawing.frames[0].width + 50.0);
    assert_eq!(changed.drawing.frames[0].borders, sign.borders);
    run_setups(&changed, None).unwrap();
}

#[test]
fn a_raised_sign_clears_inside_its_border_round_the_letters() {
    let mut request = SignRequest::new("CAFE", 2, 1);
    request.style = SignStyle::Raised;
    request.borders = 2;
    let (job, sign) = made(&request);
    assert_eq!(
        names(&job, &sign.operations),
        ["Background", "Borders", "Cut out"]
    );
    let Operation::Pocket(pocket) = &job.operations[sign.operations[0]] else {
        panic!();
    };
    assert_eq!(pocket.select.source, Some(Source::Group(sign.group)));
    // The group without the outline and the outer border: the innermost line and the
    // letters.
    assert_eq!(pocket.select.exclude, vec![sign.outline, sign.borders[0]]);
    assert_eq!(pocket.end_depth, 3.0);
    assert_eq!(pocket.tool, 1);
    run_setups(&job, None).unwrap();

    // Simulated: a letter's middle stands at the top; between it and the border is 3 mm
    // down.
    let mut one = request.clone();
    one.words = "I".into();
    one.borders = 1;
    let (job, sign) = made(&one);
    let result = chipload_pipeline::run_job(&job).unwrap();
    let sim = chipload_pipeline::simulate::simulate(
        &job,
        &result.program,
        &result.groups,
        Some(0.25),
        &chipload_pipeline::Cancel::new(),
    )
    .unwrap();
    let text = &job.drawing.texts[0];
    let at = |x: f64, y: f64| {
        let i = ((x - sim.origin.0) / sim.cell) as usize;
        let j = ((y - sim.origin.1) / sim.cell) as usize;
        sim.top - sim.height(i, j)
    };
    assert!(at(text.x, text.y).abs() < 1e-6, "{}", at(text.x, text.y));
    let frame = job
        .drawing
        .frames
        .iter()
        .find(|f| f.outline == sign.outline)
        .unwrap();
    // Halfway between the letter's box (the margin is 10 mm) and the border's groove.
    let letters: Vec<u64> = job
        .drawing
        .shapes
        .iter()
        .filter(|s| s.text() == Some(text.id))
        .map(|s| s.id)
        .collect();
    let ((_, _), (x1, _)) = chipload_pipeline::shapes::shapes_bounds(&job, &letters).unwrap();
    assert!(
        (at(x1 + 5.0, text.y) - 3.0).abs() < 1e-6,
        "{}",
        at(x1 + 5.0, text.y)
    );
    assert!(frame.width > 0.0);

    request.borders = 0;
    let err = add_sign(&project(), &request, &BuiltinOnly, None, Language::En).unwrap_err();
    assert!(err.to_string().contains("needs a border"), "{err}");
}

#[test]
fn an_inlay_sign_makes_the_pocket_here_and_the_plug_in_its_own_board() {
    let mut request = SignRequest::new("JO", 2, 1);
    request.style = SignStyle::Inlay;
    request.height = 50.0;
    let (job, sign) = made(&request);
    assert_eq!(
        names(&job, &sign.operations),
        ["Inlay pocket", "Borders", "Cut out"]
    );
    let setup = sign.plug_setup.unwrap();
    assert_eq!(job.setups.len(), 2);
    let plug = job
        .operations
        .iter()
        .find_map(|op| match op {
            Operation::Vcarve(v) if v.plug.is_some() => Some(v),
            _ => None,
        })
        .unwrap();
    assert_eq!(job.part(plug.part).unwrap().setup, setup);
    assert_eq!(job.active_part, Some(sign.part));
    assert_eq!(run_setups(&job, None).unwrap().len(), 2);
}

#[test]
fn a_sign_refuses_the_wrong_tools_and_what_the_bit_cant_cut() {
    let request = SignRequest::new("NO", 1, 1);
    let err = add_sign(&project(), &request, &BuiltinOnly, None, Language::En).unwrap_err();
    assert!(err.to_string().contains("isn't a V-bit"), "{err}");
    let request = SignRequest::new("NO", 2, 2);
    let err = add_sign(&project(), &request, &BuiltinOnly, None, Language::En).unwrap_err();
    assert!(err.to_string().contains("isn't an end mill"), "{err}");
    let mut request = SignRequest::new("NO", 2, 1);
    request.edge = Some(8.0);
    let err = add_sign(&project(), &request, &BuiltinOnly, None, Language::En).unwrap_err();
    assert!(err.to_string().contains("chamfer 8 mm wide"), "{err}");
    let request = SignRequest::new("  \n ", 2, 1);
    assert!(add_sign(&project(), &request, &BuiltinOnly, None, Language::En).is_err());
}

#[test]
fn a_sign_in_a_project_with_a_drawing_can_leave_the_board_as_it_is() {
    let mut request = SignRequest::new("A", 2, 1);
    request.board = None;
    let before = project();
    let (job, first) = add_sign(&before, &request, &BuiltinOnly, None, Language::En).unwrap();
    assert_eq!(job.stock(), before.stock());
    // A second sign (on the first until moved): a part of its own, named after it.
    request.words = "B".into();
    let (job, second) = add_sign(&job, &request, &BuiltinOnly, None, Language::En).unwrap();
    assert_ne!(first.part, second.part);
    assert_eq!(job.parts.len(), 2);
    assert_eq!(job.drawing.frames.len(), 2);
}

/// The corpus's traced logo (picture 1, shapes 1-4) with the sign tools.
fn traced() -> Job {
    let mut job =
        chipload_job::parse_job(corpus_value("jobs/v19/image_vcarve.chipload"), "test").unwrap();
    job.operations.clear();
    job.tools = project().tools;
    job
}

#[test]
fn a_sign_round_a_traced_picture_follows_it_and_carves_it_as_the_picture() {
    let job = traced();
    let mut request = SignRequest::new("", 2, 1);
    request.around = vec![1, 2, 3, 4];
    request.edge = Some(1.5);
    let (made_job, sign) = add_sign(&job, &request, &BuiltinOnly, None, Language::En).unwrap();
    assert_eq!(sign.text, None);
    assert!(
        made_job.drawing.texts.len() == job.drawing.texts.len(),
        "no words added"
    );
    assert_eq!(
        names(&made_job, &sign.operations),
        ["Carving", "Borders", "Edge chamfer", "Cut out"]
    );
    let Operation::Vcarve(carve) = &made_job.operations[sign.operations[0]] else {
        panic!();
    };
    // Cut as the picture, so it still is when traced again.
    assert_eq!(carve.select.source, Some(Source::Image(1)));
    assert_eq!(made_job.part(sign.part).unwrap().name, "logo");
    let frame = &made_job.drawing.frames[0];
    assert_eq!(frame.around.as_ref().unwrap().images, vec![1]);
    // The group holds the frame only: a picture's shapes can't be grouped.
    let group = &made_job.drawing.groups[0];
    assert!(group.texts.is_empty() && group.shapes.contains(&sign.outline));
    run_setups(&made_job, None).unwrap();

    // Raised round a picture: refused, with why.
    request.style = SignStyle::Raised;
    let err = add_sign(&job, &request, &BuiltinOnly, None, Language::En).unwrap_err();
    assert!(err.to_string().contains("picture"), "{err}");
}

#[test]
fn a_sign_round_some_of_a_pictures_shapes_leaves_the_others_out_and_a_missing_shape_is_refused() {
    let job = traced();
    let mut request = SignRequest::new("", 2, 1);
    request.around = vec![1, 2];
    let (made_job, sign) = add_sign(&job, &request, &BuiltinOnly, None, Language::En).unwrap();
    let Operation::Vcarve(carve) = &made_job.operations[sign.operations[0]] else {
        panic!();
    };
    assert_eq!(carve.select.source, Some(Source::Image(1)));
    assert_eq!(carve.select.exclude, vec![3, 4]);
    request.around = vec![99];
    assert!(add_sign(&job, &request, &BuiltinOnly, None, Language::En).is_err());
}

#[test]
fn a_raised_sign_round_drawn_shapes_groups_them() {
    let job = project();
    let geometry = chipload_job::Geometry::Circle {
        center: [100.0, 100.0],
        radius: 20.0,
    };
    let (job, circle) = chipload_pipeline::shapes::add_shape(&job, "art", geometry).unwrap();
    let mut request = SignRequest::new("", 2, 1);
    request.around = vec![circle];
    request.style = SignStyle::Raised;
    let (made_job, sign) = add_sign(&job, &request, &BuiltinOnly, None, Language::En).unwrap();
    assert!(made_job.drawing.groups[0].shapes.contains(&circle));
    assert_eq!(made_job.part(sign.part).unwrap().name, "Sign");
    run_setups(&made_job, None).unwrap();
}
