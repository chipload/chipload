// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[test]
fn a_profiles_spiral_ramp_and_lead_draw_in_its_cut_settings() {
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
    let set = |app: &mut ChiploadApp, entry: chipload_job::Entry, lead: bool| {
        if let Operation::Profile(p) = &mut app.edit.as_mut().unwrap().ops[i].op {
            p.entry = entry;
            p.lead = lead.then_some(chipload_job::Lead { radius: 3.0 });
        }
        app.commit();
    };
    let ramp = chipload_job::Entry::Ramp {
        ramp_angle: chipload_job::DEFAULT_RAMP_ANGLE_RAD,
    };
    for (entry, lead) in [
        (chipload_job::Entry::Plunge {}, false),
        (chipload_job::Entry::Plunge {}, true),
        (ramp, false),
        // An older project with both: shown, and kept as it is.
        (ramp, true),
    ] {
        set(&mut app, entry, lead);
        draw_properties(&mut app);
        let Operation::Profile(p) = &app.edit.as_ref().unwrap().ops[i].op else {
            unreachable!()
        };
        assert_eq!((p.entry, p.lead.is_some()), (entry, lead));
    }
    // An entry problem marks the Cut section.
    let fields = super::problems::section_fields("op-profile-cut");
    assert!(fields.contains(&"entry") && fields.contains(&"lead"));
}
