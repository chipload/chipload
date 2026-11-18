// SPDX-License-Identifier: GPL-3.0-or-later
//! Layer rules: styles with a layer pattern, applied to a drawing's layers
//! (docs/specs/fewer-clicks.md §4).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chipload_core::chain::Contour;
use chipload_i18n::Language;
use chipload_job::library::{LibraryTool, ToolLibrary, uid_for_name};
use chipload_job::styles::{Style, layer_matches, style_of};
use chipload_job::{Geometry, Job, Operation, Selector};
use chipload_pipeline::new_ops::{NewRequest, OpKind, insert, new_operations};
use chipload_pipeline::styles::apply_layer_rules;
use chipload_pipeline::{empty_project, load_drawing};

fn rect(x: f64, y: f64, length: f64, width: f64) -> Geometry {
    Geometry::Rectangle {
        center: [x, y],
        length,
        width,
        angle: 0.0,
        corner_radius: 0.0,
        radii: Vec::new(),
    }
}

/// A part drawn on layers CUT_OUTER, pocket and DRILL_5, its tool in the library.
fn drawn() -> (Job, Vec<Contour>, ToolLibrary) {
    let mut job = empty_project();
    for (layer, geometry) in [
        ("CUT_OUTER", rect(50.0, 50.0, 90.0, 90.0)),
        ("pocket", rect(40.0, 40.0, 20.0, 20.0)),
        (
            "DRILL_5",
            Geometry::Circle {
                center: [70.0, 70.0],
                radius: 2.5,
            },
        ),
        (
            "DRILL_6",
            Geometry::Circle {
                center: [70.0, 30.0],
                radius: 3.0,
            },
        ),
    ] {
        job = chipload_pipeline::shapes::add_shape(&job, layer, geometry)
            .unwrap()
            .0;
    }
    let entry = LibraryTool::from_tool("6 mm", &uid_for_name("6 mm"), &job.tools[0], Vec::new());
    job.tools[0].library_uid = Some(entry.uid.clone());
    job.tools[0].name = Some("6 mm".into());
    let library = ToolLibrary {
        tools: vec![entry],
        ..ToolLibrary::default()
    };
    let contours = load_drawing(&job).unwrap().contours;
    (job, contours, library)
}

/// A style of `kind` made from a new operation of that kind, with layer pattern `layer`.
fn style(
    job: &Job,
    contours: &[Contour],
    library: &ToolLibrary,
    kind: OpKind,
    name: &str,
    layer: &str,
) -> Style {
    let mut with = job.clone();
    let request = NewRequest {
        kind,
        select: Selector {
            layer: Some(layer.trim_end_matches('*').into()),
            ..Selector::default()
        },
        tool: None,
    };
    let made = new_operations(&with, &request, contours, None, Language::En);
    let index = insert(&mut with, made);
    let mut style = style_of(&with, index, name, library).unwrap();
    style.uid = uid_for_name(name);
    style.layer = Some(layer.into());
    style
}

fn names(job: &Job) -> Vec<(String, Option<String>)> {
    job.operations
        .iter()
        .map(|op| (op.name().to_string(), op.select().layer.clone()))
        .collect()
}

#[test]
fn patterns_match_as_layer_selection_does() {
    assert!(layer_matches("CUT*", "cut_outer"));
    assert!(layer_matches("cut*", "CUT"));
    assert!(layer_matches("DRILL_?", "DRILL_5"));
    assert!(!layer_matches("DRILL_?", "DRILL_10"));
    assert!(layer_matches("*_?", "DRILL_5"));
    assert!(layer_matches("pocket", "Pocket"));
    assert!(!layer_matches("pocket", "pockets"));
    assert!(layer_matches("*", "anything"));
    assert!(layer_matches("a*b*c", "aXXbYYc"));
    assert!(!layer_matches("a*b*c", "aXXbYY"));
    // No regular expressions: a dot is a dot.
    assert!(!layer_matches("a.c", "abc"));
}

#[test]
fn rules_add_by_style_order_then_layer_name_and_skip_what_is_cut() {
    let (job, contours, library) = drawn();
    let styles = [
        style(&job, &contours, &library, OpKind::Holes, "Drill", "DRILL_?"),
        style(&job, &contours, &library, OpKind::Pocket, "Clear", "pocket"),
        style(
            &job,
            &contours,
            &library,
            OpKind::Profile,
            "Cut out",
            "CUT*",
        ),
    ];
    let rules = apply_layer_rules(&job, &contours, &styles, &library, Language::En);
    assert_eq!(rules.added, 4);
    assert_eq!(
        names(&rules.job),
        [
            ("Drill".to_string(), Some("DRILL_5".to_string())),
            ("Drill 2".to_string(), Some("DRILL_6".to_string())),
            ("Clear".to_string(), Some("pocket".to_string())),
            ("Cut out".to_string(), Some("CUT_OUTER".to_string())),
        ]
    );
    let report: Vec<String> = rules.messages.iter().map(|m| m.english()).collect();
    assert_eq!(report[0], "style \"Drill\" on layer DRILL_5: Drill");
    assert_eq!(report.len(), 4, "{report:?}");
    // The profile is outside, as a new one on an outline is.
    assert!(matches!(
        &rules.job.operations[3],
        Operation::Profile(p) if p.side == chipload_job::Side::Outside
    ));
    // Applying again adds nothing: every layer is cut now.
    let again = apply_layer_rules(&rules.job, &contours, &styles, &library, Language::En);
    assert_eq!(again.added, 0);
    assert_eq!(again.job, rules.job);
    assert!(
        again
            .messages
            .iter()
            .all(|m| m.english().contains("already cuts it"))
    );
}

#[test]
fn two_styles_on_one_layer_both_apply_and_unmatched_layers_are_named() {
    let (job, contours, library) = drawn();
    let styles = [
        style(&job, &contours, &library, OpKind::Pocket, "Rough", "pocket"),
        style(
            &job,
            &contours,
            &library,
            OpKind::Pocket,
            "Finish",
            "POCKET",
        ),
    ];
    let rules = apply_layer_rules(&job, &contours, &styles, &library, Language::En);
    assert_eq!(rules.added, 2);
    let report: Vec<String> = rules.messages.iter().map(|m| m.english()).collect();
    assert_eq!(
        report.last().unwrap(),
        "no layer rule matches layers CUT_OUTER, DRILL_5, DRILL_6"
    );
}

#[test]
fn a_layer_cut_by_one_of_its_shapes_is_skipped_and_no_rules_says_so() {
    let (mut job, contours, library) = drawn();
    // The pocket shape (S2) is already cut by its shape.
    let request = NewRequest {
        kind: OpKind::Pocket,
        select: Selector {
            shapes: Some(vec![2]),
            ..Selector::default()
        },
        tool: None,
    };
    let made = new_operations(&job, &request, &contours, None, Language::En);
    insert(&mut job, made);
    let styles = [style(
        &job,
        &contours,
        &library,
        OpKind::Pocket,
        "Clear",
        "pocket",
    )];
    let rules = apply_layer_rules(&job, &contours, &styles, &library, Language::En);
    assert_eq!(rules.added, 0);
    assert_eq!(
        rules.messages[0].english(),
        "layer pocket skipped for style \"Clear\": an operation already cuts it"
    );
    // No style with a pattern: nothing to do, and it says how.
    let mut plain = styles[0].clone();
    plain.layer = None;
    let rules = apply_layer_rules(&job, &contours, &[plain], &library, Language::En);
    assert_eq!(rules.job, job);
    assert!(
        rules.messages[0]
            .english()
            .contains("no style has a layer rule")
    );
}
