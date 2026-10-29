// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[test]
fn a_profiles_ramp_angle_draws_in_its_cut_settings() {
    let mut app = lower_part();
    let i = app
        .edit
        .as_ref()
        .unwrap()
        .ops
        .iter()
        .position(|e| matches!(e.op, Operation::Profile(_)))
        .unwrap();
    app.focus = Focus::Op(i);
    for angle in [chipload_job::DEFAULT_RAMP_ANGLE_RAD, 0.2] {
        if let Operation::Profile(p) = &mut app.edit.as_mut().unwrap().ops[i].op {
            p.ramp_angle = angle;
        }
        app.commit();
        draw_properties(&mut app);
        let Operation::Profile(p) = &app.edit.as_ref().unwrap().ops[i].op else {
            unreachable!()
        };
        assert_eq!(p.ramp_angle, angle);
    }
    // A ramp angle problem marks the Cut section.
    let fields = super::problems::section_fields("op-profile-cut");
    assert!(fields.contains(&"ramp_angle"));
}

#[test]
fn a_profile_cut_through_follows_the_stock_thickness() {
    // docs/specs/job-settings-layout.md §5.
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1200.0, 900.0))
        .build_eframe(|_| {
            let mut app = lower_part();
            let i = app
                .edit
                .as_ref()
                .unwrap()
                .ops
                .iter()
                .position(|e| matches!(e.op, Operation::Profile(_)))
                .unwrap();
            app.focus = Focus::Op(i);
            app
        });
    harness.run_steps(4);
    let profile = |h: &Harness<'_, ChiploadApp>| {
        let edit = h.state().edit.as_ref().unwrap();
        let i = edit
            .ops
            .iter()
            .position(|e| matches!(e.op, Operation::Profile(_)))
            .unwrap();
        (
            edit.to_job().unwrap().operations[i].clone(),
            edit.settings.stock.clone(),
        )
    };
    assert_eq!(profile(&harness).0.through(), None);
    harness.get_by_label("Through").click();
    harness.run_steps(4);
    let (op, stock) = profile(&harness);
    assert_eq!(op.through(), Some(stock.max_overcut));
    assert_eq!(op.depths().1, stock.thickness + stock.max_overcut);
    assert!(harness.query_by_label("Below stock").is_some());
    // A new thickness moves the end depth with it.
    harness
        .state_mut()
        .edit
        .as_mut()
        .unwrap()
        .settings
        .stock
        .thickness = 12.0;
    harness.state_mut().commit();
    harness.run_steps(4);
    let (op, stock) = profile(&harness);
    assert_eq!(op.depths().1, 12.0 + stock.max_overcut);
    // Unticked, the depth stays where it was.
    harness.get_by_label("Through").click();
    harness.run_steps(4);
    let (op, stock) = profile(&harness);
    assert_eq!(op.through(), None);
    assert_eq!(op.depths().1, 12.0 + stock.max_overcut);
}
