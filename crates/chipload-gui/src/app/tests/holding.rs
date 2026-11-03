// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[test]
fn tabs_and_onion_skin_are_one_choice_and_switching_keeps_what_stays() {
    use super::forms::Holding;
    let mut app = lower_part();
    let i = app
        .edit
        .as_ref()
        .unwrap()
        .job
        .operations
        .iter()
        .position(|e| matches!(e, Operation::Profile(_)))
        .unwrap();
    let profile = |app: &mut ChiploadApp| -> chipload_job::ProfileOp {
        match &app.edit.as_ref().unwrap().job.operations[i] {
            Operation::Profile(p) => p.clone(),
            _ => unreachable!(),
        }
    };
    let mut p = profile(&mut app);
    p.tabs = None;
    p.skin = None;
    assert_eq!(Holding::of(&p), Holding::None);
    Holding::Tabs.apply(&mut p, None);
    assert!(p.tabs.is_some() && p.skin.is_none());
    // Tabs changed by hand stay when the skin is added.
    p.tabs.as_mut().unwrap().count = 6;
    Holding::SkinAndTabs.apply(&mut p, None);
    assert_eq!(p.tabs.as_ref().unwrap().count, 6);
    let skin = p.skin.unwrap().thickness;
    assert!(skin <= p.step_down && skin < p.tabs.as_ref().unwrap().height);
    Holding::Skin.apply(&mut p, None);
    assert!(p.tabs.is_none() && p.skin == Some(chipload_job::Skin { thickness: skin }));
    Holding::None.apply(&mut p, None);
    assert_eq!(Holding::of(&p), Holding::None);
    // Each choice draws, and a tab problem marks the Holding section.
    app.focus = Focus::Op(i);
    for holding in Holding::ALL {
        let mut p = profile(&mut app);
        holding.apply(&mut p, None);
        app.edit.as_mut().unwrap().job.operations[i] = Operation::Profile(p);
        app.commit();
        draw_properties(&mut app);
    }
    assert_eq!(
        super::problems::section_fields("op-holding"),
        ["tabs", "skin"]
    );
}

#[test]
fn clicking_a_traced_contour_selects_the_whole_picture() {
    // User report 2026-10-13: a V-carve made after clicking a traced contour selected its
    // shape by id, which is refused (it changes each time the picture is traced again).
    let mut app = two_parts();
    let traced = app
        .contours
        .iter()
        .find(|c| {
            c.parts
                .iter()
                .all(|p| p.segment.from == Some(chipload_core::chain::ShapeFrom::Image(1)))
        })
        .map(|c| c.id)
        .unwrap();
    app.click(Some(traced), false);
    assert_eq!(
        app.selection,
        CanvasSelection::whole(chipload_core::chain::ShapeFrom::Image(1))
    );
    let index = app.add_operation(OpKind::Vcarve).unwrap();
    let op = &app.edit.as_ref().unwrap().job.operations[index];
    assert_eq!(op.select().image(), Some(1));
    assert!(op.select().shapes.as_ref().is_none_or(|s| s.is_empty()));
    assert!(
        !app.problems
            .iter()
            .any(|p| problems::plain(&app, app.prefs.units, p).contains("traced from an image")),
        "{:?}",
        app.problems
    );
}

#[test]
fn use_selection_with_a_picture_selects_the_picture() {
    // User report 2026-10-14: "I still can't create a V-carve: select an image, click Use
    // selection…". Use selection set the traced shapes by id, which is refused.
    let mut app = two_parts();
    let traced: std::collections::BTreeSet<u64> = app
        .edit
        .as_ref()
        .unwrap()
        .job
        .drawing
        .shapes
        .iter()
        .filter(|s| s.image() == Some(1))
        .map(|s| s.id)
        .collect();
    assert!(!traced.is_empty());
    // A V-carve of something else first, then its geometry from the picture: chosen in the
    // Browser, and as the traced contours picked on the canvas.
    app.select_group("dxf:1");
    let index = app.add_operation(OpKind::Vcarve).unwrap();
    for pick in ["browser", "canvas"] {
        match pick {
            "browser" => app.select_group("image:1"),
            _ => app.selection = CanvasSelection::Shapes(traced.clone()),
        }
        app.focus = Focus::Op(index);
        app.use_selection();
        let select = app.edit.as_ref().unwrap().job.operations[index]
            .select()
            .clone();
        assert_eq!(select.image(), Some(1), "{pick}: {select:?}");
        assert!(
            select.shapes.as_ref().is_none_or(|s| s.is_empty()),
            "{pick}"
        );
        assert!(
            !app.problems
                .iter()
                .any(|p| problems::plain(&app, app.prefs.units, p).contains("traced from an image")),
            "{pick}: {:?}",
            app.problems
        );
        // Copy as selector gives the same.
        assert_eq!(
            app.selector_json().as_deref(),
            Some("{\"image\": 1}"),
            "{pick}"
        );
    }
}

#[test]
fn tabs_placed_along_x_sit_on_the_bottom_and_top_edges() {
    // docs/specs/tabs-along-grain.md: the Placement under Tabs, then the tabs it plans.
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .build_eframe(|cc| {
            open_sections(&cc.egui_ctx);
            let mut app = ChiploadApp::default();
            assert!(app.import_dxf(&corpus("dxf/simple_part.dxf"), None));
            select(&mut app, &[1]);
            let i = app.add_operation(OpKind::Profile).unwrap();
            app.focus = Focus::Op(i);
            app
        });
    harness.run_steps(4);
    assert!(harness.query_by_label_contains("placement").is_some());
    if let Operation::Profile(p) = &mut harness.state_mut().edit.as_mut().unwrap().job.operations[0]
    {
        p.tabs.as_mut().unwrap().along = Some(chipload_job::Axis::X);
    }
    harness.state_mut().commit();
    harness.run_steps(4);
    let app = harness.state_mut();
    assert!(app.generate_and_wait(), "{:?}", app.report);
    let tabs = app.scene.tabs.clone();
    assert_eq!(tabs.len(), 4);
    // Every tab lies on a horizontal edge: all its points at one Y.
    for tab in &tabs {
        let y = tab[0].1;
        assert!(tab.iter().all(|p| (p.1 - y).abs() < 1e-6), "{tab:?}");
    }
}
