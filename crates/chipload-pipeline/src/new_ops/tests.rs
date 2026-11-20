// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use chipload_job::library::LibraryTool;
use chipload_job::{MaterialRef, Source};
use std::path::PathBuf;

/// simple_part: contour 1 the outline (layer CUT_OUTSIDE), 2-4 its holes (HOLES).
fn part() -> (Job, Vec<Contour>) {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/dxf/simple_part.dxf");
    let job = crate::new_project(chipload_dxf::shapes_from_dxf(&path, None).unwrap());
    let contours = crate::load_drawing(&job).unwrap().contours;
    (job, contours)
}

fn layer(name: &str) -> Selector {
    Selector::on_layer(name)
}

fn tool(kind: ToolType, diameter: f64) -> Tool {
    Tool {
        id: 1,
        name: None,
        library_uid: None,
        kind,
        diameter,
        flutes: None,
        center_cutting: None,
        rpm: None,
        angle: None,
        tip_diameter: None,
        corner_radius: None,
    }
}

/// docs/specs/stl-relief.md §15: ridges of 0.02 mm, to the whole percent.
#[test]
fn a_ball_noses_relief_stepover_comes_from_its_ridge() {
    let cases = [
        (0.5, 0.39),
        (1.0, 0.28),
        (2.0, 0.2),
        (6.0, 0.12),
        (12.7, 0.08),
    ];
    for (diameter, expected) in cases {
        let ball = tool(ToolType::Ballnose, diameter);
        let got = relief_stepover(Some(&ball));
        assert!((got - expected).abs() < 1e-9, "{diameter} mm: {got}");
        let ridge = relief_ridge(&ball, got).unwrap();
        assert!(
            (ridge - RELIEF_RIDGE_MM).abs() < 0.003,
            "{diameter} mm: {ridge}"
        );
    }
    // Tiny and huge balls stay within the range.
    assert_eq!(relief_stepover(Some(&tool(ToolType::Ballnose, 0.1))), 0.4);
    assert_eq!(relief_stepover(Some(&tool(ToolType::Ballnose, 50.0))), 0.05);
}

#[test]
fn other_tools_keep_the_plain_relief_stepover() {
    assert_eq!(relief_stepover(None), DEFAULT_RELIEF_STEPOVER);
    for kind in [ToolType::Endmill, ToolType::Vbit, ToolType::Drill] {
        let t = tool(kind, 6.0);
        assert_eq!(relief_stepover(Some(&t)), DEFAULT_RELIEF_STEPOVER);
        assert_eq!(relief_ridge(&t, 0.2), None);
    }
    // The ridge of a 2 mm ball at 10 %: half a hundredth.
    let ridge = relief_ridge(&tool(ToolType::Ballnose, 2.0), 0.1).unwrap();
    assert!((ridge - 0.005).abs() < 0.0001, "{ridge}");
}

fn add(job: &Job, contours: &[Contour], kind: OpKind, select: Selector) -> NewOps {
    let request = NewRequest {
        kind,
        select,
        tool: None,
    };
    new_operations(job, &request, contours, None, Language::En)
}

fn profile(op: &Operation) -> &ProfileOp {
    match op {
        Operation::Profile(p) => p,
        _ => panic!("a profile"),
    }
}

#[test]
fn an_outline_goes_outside_and_through_with_tabs_and_holes_inside() {
    let (job, contours) = part();
    let stock = job.stock().clone();
    let made = add(&job, &contours, OpKind::Profile, layer("CUT_OUTSIDE"));
    assert_eq!(made.ops.len(), 1);
    let p = profile(&made.ops[0].op);
    assert_eq!((p.side, made.ops[0].side_chosen), (Side::Outside, true));
    assert_eq!(p.through, Some(stock.max_overcut));
    assert_eq!(p.end_depth, stock.thickness + stock.max_overcut);
    assert_eq!(p.tabs.as_ref().map(|t| t.height), Some(3.0));
    assert_eq!(p.select, layer("CUT_OUTSIDE"), "a layer stays a layer");
    let made = add(&job, &contours, OpKind::Profile, layer("HOLES"));
    let p = profile(&made.ops[0].op);
    assert_eq!((p.side, p.tabs.is_none()), (Side::Inside, true));
    assert_eq!(p.through, Some(stock.max_overcut));
    assert!(made.messages.is_empty());
}

#[test]
fn an_outline_with_its_holes_is_two_profiles_holes_first_by_shapes() {
    let (job, contours) = part();
    let made = add(
        &job,
        &contours,
        OpKind::Profile,
        Selector::of(Source::Dxf(1)),
    );
    let ops: Vec<&ProfileOp> = made.ops.iter().map(|n| profile(&n.op)).collect();
    assert_eq!(
        ops.iter().map(|p| p.side).collect::<Vec<_>>(),
        [Side::Inside, Side::Outside]
    );
    assert_eq!(
        ops.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
        ["Profile 1 (holes)", "Profile 2 (outline)"]
    );
    let shapes = |p: &ProfileOp| p.select.shapes.as_ref().map_or(0, Vec::len);
    assert!(shapes(ops[0]) > 0 && shapes(ops[1]) > 0);
    assert!(ops[0].tabs.is_none() && ops[1].tabs.is_some());
    // A whole DXF split into shapes: the report says new shapes won't be cut.
    assert_eq!(made.messages.len(), 1);
    assert!(
        made.messages[0]
            .english()
            .contains("added to the drawing later")
    );
    // Picked by shapes: nothing to say.
    let mut all = Vec::new();
    for p in &ops {
        all.extend(p.select.shapes.clone().unwrap());
    }
    let by_shapes = Selector {
        shapes: Some(all),
        ..Selector::default()
    };
    let made = add(&job, &contours, OpKind::Profile, by_shapes);
    assert_eq!(made.ops.len(), 2);
    assert!(made.messages.is_empty());
}

#[test]
fn nothing_selected_leaves_the_side_to_choose() {
    let (job, contours) = part();
    let made = add(&job, &contours, OpKind::Profile, Selector::default());
    assert_eq!(made.ops.len(), 1);
    assert!(!made.ops[0].side_chosen);
    let p = profile(&made.ops[0].op);
    assert_eq!((p.through, p.end_depth), (None, DEFAULT_DEPTH));
}

#[test]
fn tabs_are_at_most_half_the_thickness() {
    let (mut job, contours) = part();
    job.stock_mut().thickness = 4.0;
    let made = add(&job, &contours, OpKind::Profile, layer("CUT_OUTSIDE"));
    let p = profile(&made.ops[0].op);
    assert_eq!(p.tabs.as_ref().map(|t| t.height), Some(2.0));
}

#[test]
fn depths_come_from_the_last_of_the_kind_and_holes_go_through() {
    let (mut job, contours) = part();
    let made = add(&job, &contours, OpKind::Pocket, layer("HOLES"));
    assert_eq!(made.ops[0].op.depths(&[]).1, DEFAULT_DEPTH);
    insert(&mut job, made);
    job.operations[0].set_end_depth(6.0);
    let made = add(&job, &contours, OpKind::Pocket, layer("HOLES"));
    assert_eq!(made.ops[0].op.depths(&[]).1, 6.0);
    assert_eq!(made.ops[0].op.name(), "Pocket 2");
    let made = add(&job, &contours, OpKind::Holes, layer("HOLES"));
    let stock = job.stock();
    assert_eq!(made.ops[0].op.through(), Some(stock.max_overcut));
    assert_eq!(
        made.ops[0].op.depths(&[]).1,
        stock.thickness + stock.max_overcut
    );
}

#[test]
fn a_new_pocket_helixes_and_a_path_ramps_whatever_the_tool() {
    let (mut job, contours) = part();
    let pocket = add(&job, &contours, OpKind::Pocket, layer("HOLES"));
    let diameter = job.tools[0].diameter;
    assert_eq!(pocket.ops[0].op.entry(), Entry::default_helix(diameter));
    let path = add(&job, &contours, OpKind::Path, layer("HOLES"));
    assert!(matches!(path.ops[0].op.entry(), Entry::Ramp { .. }));
    // Marked centre-cutting: the same, gentler on the tool (styles.md §2.4, 2026-11-20).
    job.tools[0].center_cutting = Some(true);
    let pocket = add(&job, &contours, OpKind::Pocket, layer("HOLES"));
    assert_eq!(pocket.ops[0].op.entry(), Entry::default_helix(diameter));
    let path = add(&job, &contours, OpKind::Path, layer("HOLES"));
    assert!(matches!(path.ops[0].op.entry(), Entry::Ramp { .. }));
}

#[test]
fn cutting_values_come_from_the_tool_not_from_another_tools_operation() {
    let (mut job, contours) = part();
    // An operation above with tool 1 and its own feed.
    let made = add(&job, &contours, OpKind::Pocket, layer("HOLES"));
    insert(&mut job, made);
    if let Operation::Pocket(p) = &mut job.operations[0] {
        p.feed = 900.0;
    }
    // Without cutting data, the same tool takes the feed above.
    let made = add(&job, &contours, OpKind::Profile, layer("CUT_OUTSIDE"));
    assert!(matches!(&made.ops[0].op, Operation::Profile(p) if p.feed == 900.0));
    // A second tool from the library with a row for the setup's material.
    let mut tool = job.tools[0].clone();
    tool.id = 2;
    tool.library_uid = Some("c000000000000002".into());
    job.tools.push(tool.clone());
    let mut entry = LibraryTool::from_tool("6 mm", "c000000000000002", &tool, Vec::new());
    let ply = Cutting::uniform(2000.0, 600.0, 2.5, 0.45);
    entry.set_row(Some("m1"), ply);
    entry.set_row(None, Cutting::uniform(1000.0, 300.0, 1.0, 0.3));
    let mut library = ToolLibrary::default();
    library.tools.push(entry);
    job.stock_mut().material = Some(MaterialRef {
        name: "Plywood".into(),
        library_uid: "m1".into(),
    });
    let request = NewRequest {
        kind: OpKind::Pocket,
        select: layer("HOLES"),
        tool: Some(2),
    };
    let made = new_operations(&job, &request, &contours, Some(&library), Language::En);
    let op = &made.ops[0].op;
    assert!(op.has_cutting(&ply), "{op:?}");
    assert_eq!(op.material().map(|m| m.name.as_str()), Some("Plywood"));
}

#[test]
fn a_helix_made_for_a_larger_tool_fits_the_smaller_one() {
    let (mut job, contours) = part();
    let made = add(&job, &contours, OpKind::Pocket, layer("HOLES"));
    insert(&mut job, made);
    assert_eq!(job.operations[0].entry(), Entry::default_helix(6.0));
    job.tools[0].diameter = 3.0;
    job.fit_helixes();
    assert_eq!(job.operations[0].entry(), Entry::default_helix(3.0));
    // One that fits stays.
    job.tools[0].diameter = 10.0;
    job.fit_helixes();
    assert_eq!(job.operations[0].entry(), Entry::default_helix(3.0));
}

/// A big rectangle (shape 1) and a small one inside it (shape 2), in mm.
fn nested_rectangles() -> Job {
    let rect = |center: [f64; 2], size: f64| chipload_job::Geometry::Rectangle {
        center,
        length: size,
        width: size,
        angle: 0.0,
        corner_radius: 0.0,
        radii: Vec::new(),
    };
    let job = crate::empty_project();
    let (job, _) = crate::shapes::add_shape(&job, "0", rect([100.0, 100.0], 120.0)).unwrap();
    let (job, _) = crate::shapes::add_shape(&job, "0", rect([100.0, 100.0], 40.0)).unwrap();
    job
}

fn shapes(ids: &[u64]) -> Selector {
    Selector {
        shapes: Some(ids.to_vec()),
        ..Selector::default()
    }
}

fn side_and_tabs(made: &NewOps) -> (Side, bool) {
    match &made.ops[0].op {
        Operation::Profile(p) => (p.side, p.tabs.is_some()),
        other => panic!("{other:?}"),
    }
}

/// The user, 2026-11-17: a part drawn inside another part's outline (each to be cut from
/// its own board) is an outline of its own: outside, with tabs. Only another part's
/// outline is passed over; one no operation cuts, or the part's own, still makes a hole.
#[test]
fn a_shape_inside_another_parts_outline_is_an_outline_of_its_own() {
    let job = nested_rectangles();
    let contours = crate::load_drawing(&job).unwrap().contours;
    // Nothing cuts the big one yet: the small one is its hole.
    let made = add(&job, &contours, OpKind::Profile, shapes(&[2]));
    assert_eq!(side_and_tabs(&made), (Side::Inside, false));
    // Part 1 cuts the big one out.
    let made = add(&job, &contours, OpKind::Profile, shapes(&[1]));
    assert_eq!(side_and_tabs(&made), (Side::Outside, true));
    let mut job = job.clone();
    job.operations.push(made.ops[0].op.clone());
    // Part 2 (active): the small one is its own outline.
    let (two, part) = crate::parts::add_part(&job, None).unwrap();
    let made = add(&two, &contours, OpKind::Profile, shapes(&[2]));
    assert_eq!(made.ops[0].op.part(), part);
    assert_eq!(side_and_tabs(&made), (Side::Outside, true));
    // In part 1, the big one's own part, it is still a hole.
    let made = add(&job, &contours, OpKind::Profile, shapes(&[2]));
    assert_eq!(side_and_tabs(&made), (Side::Inside, false));
    // Both selected in part 2: the big one is selected, so the small one is its hole.
    let made = add(&two, &contours, OpKind::Profile, shapes(&[1, 2]));
    let sides: Vec<Side> = made
        .ops
        .iter()
        .filter_map(|n| match &n.op {
            Operation::Profile(p) => Some(p.side),
            _ => None,
        })
        .collect();
    assert_eq!(sides, [Side::Inside, Side::Outside]);
}
