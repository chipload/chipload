// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[test]
fn tools_dxfs_and_pictures_have_properties_of_their_own() {
    let mut app = two_parts();
    for focus in [
        Focus::Job,
        Focus::Tools,
        Focus::Tool(1),
        Focus::Tool(2),
        Focus::Dxf(1),
        Focus::Image(1),
    ] {
        app.focus = focus;
        draw_properties(&mut app);
        draw_sidebar(&mut app);
    }
    // A DXF's row selects it and shows its properties.
    app.focus = Focus::Job;
    app.select_group("dxf:2");
    assert_eq!(
        app.selection,
        CanvasSelection::Source(chipload_core::chain::ShapeFrom::Dxf(2))
    );
}

#[test]
fn a_pictures_new_size_and_place_trace_it_as_the_pipeline_does() {
    let mut app = two_parts();
    let place = super::forms::ImagePlace {
        width: 40.0,
        x: 20.0,
        y: 150.0,
        rotation: 0.0,
        layer: "badge".into(),
    };
    assert!(app.set_image_place(1, &place));
    let job = app.edit.as_ref().unwrap().to_job().unwrap();
    let image = &job.drawing.images[0];
    assert_eq!((image.width, image.x, image.y), (40.0, 20.0, 150.0));
    assert_eq!(image.layer, "badge");
    // The same as setting the values and tracing again in the pipeline.
    let mut by_hand = chipload_job::load_job(&corpus("jobs/v22/two_parts.chipload")).unwrap();
    let wanted = &mut by_hand.drawing.images[0];
    (wanted.width, wanted.x, wanted.y) = (40.0, 20.0, 150.0);
    wanted.layer = "badge".into();
    let (expected, _) = chipload_pipeline::images::retrace(&by_hand, 1).unwrap();
    assert_eq!(job.drawing.shapes, expected.drawing.shapes);
    // One undo step.
    app.undo();
    let back = app.edit.as_ref().unwrap().settings.drawing.images[0].width;
    assert!(back != 40.0, "{back}");
}

#[test]
fn a_tools_problem_belongs_to_the_tool_not_the_job_settings() {
    let mut app = two_parts();
    app.edit.as_mut().unwrap().settings.tools[1].rpm = None;
    app.commit();
    let fields = app.form_problem_map(Focus::Tool(2));
    assert!(
        fields.keys().any(|f| f.starts_with("tools.1.")),
        "{fields:?}"
    );
    assert!(app.form_problem_map(Focus::Tool(1)).is_empty());
    assert_eq!(app.settings_problems().count(), 0);
    assert!(!app.form_problem_map(Focus::Tools).is_empty());
}

#[test]
fn the_selection_never_pushes_an_open_operation_down() {
    use chipload_core::chain::ShapeFrom;
    let mut app = two_parts();
    app.focus = Focus::Job;
    app.selection = CanvasSelection::Source(ShapeFrom::Dxf(2));
    assert!(
        app.selection_shown(),
        "with the project open it comes first"
    );
    app.focus = Focus::Op(0);
    assert!(!app.selection_shown(), "Use selection counts it instead");
    draw_properties(&mut app);
    app.focus = Focus::Dxf(2);
    assert!(!app.selection_shown(), "the open DXF is what is selected");
    app.focus = Focus::Dxf(1);
    assert!(app.selection_shown(), "another DXF's shapes are selected");
    app.selection = CanvasSelection::Nothing;
    app.focus = Focus::Job;
    assert!(!app.selection_shown());
}
