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
