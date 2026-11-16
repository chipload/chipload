// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[test]
fn the_parts_are_found_where_they_are_drawn() {
    let c = egui::pos2(100.0, 100.0);
    assert_eq!(part_at(c, 0.0, c), Some(CompassPart::Centre));
    assert_eq!(
        part_at(c, 0.0, c + egui::vec2(RING_PX, 0.0)),
        Some(CompassPart::Ring)
    );
    assert_eq!(
        part_at(c, 0.0, c + egui::vec2(50.0, 0.0)),
        Some(CompassPart::X)
    );
    // Y up on screen is minus.
    assert_eq!(
        part_at(c, 0.0, c + egui::vec2(0.0, -50.0)),
        Some(CompassPart::Y)
    );
    // Turned 90°: X points up.
    assert_eq!(
        part_at(c, 90.0, c + egui::vec2(0.0, -50.0)),
        Some(CompassPart::X)
    );
    assert_eq!(part_at(c, 0.0, c + egui::vec2(-50.0, 0.0)), None);
}

#[test]
fn turns_step_and_stay_within_half_a_turn() {
    assert_eq!(normalised(190.0), -170.0);
    assert_eq!(normalised(-190.0), 170.0);
    assert_eq!(stepped(22.0, false), 15.0);
    assert!((stepped(22.04, true) - 22.0).abs() < 1e-9);
    // A line at 10° seen from 170° is the line's other way.
    assert!((nearest_turn(10.0, 170.0) - 190.0).abs() < 1e-9);
    assert!((nearest_turn(10.0, 20.0) - 10.0).abs() < 1e-9);
}

#[test]
fn a_turned_box_goes_round_the_centre() {
    let start = PlacedBox {
        corner: (10.0, -5.0),
        size: (20.0, 10.0),
        rotation: 0.0,
    };
    let now = moved_box(&start, (0.0, 0.0), (0.0, 0.0), 90.0);
    let c = now.center();
    assert!(
        (c.0 - 0.0).abs() < 1e-9 && (c.1 - 20.0).abs() < 1e-9,
        "{c:?}"
    );
    assert_eq!(now.rotation, 90.0);
}

/// An instance turned about a point that isn't its own centre lands where its ghost's
/// outlines, turned about that point, show it (docs/specs/part-placement.md §5).
#[test]
fn an_instance_turns_about_the_compass_wherever_it_is() {
    let mut app = ChiploadApp::default();
    let project = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus/jobs/v5/drawn_part.chipload");
    assert!(app.open(&project));
    app.add_instance(1, false);
    let draw = |app: &mut ChiploadApp| {
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| app.canvas(ui));
        output.textures_delta.clear();
    };
    draw(&mut app);
    let before = app.scene.instances[0].outlines.clone();
    let pivot = (40.0, -25.0);
    let target = Target::Instance(1, 2);
    app.compass_apply(&target, pivot, 0.0, Motion::Turn(30.0));
    draw(&mut app);
    let after = app.scene.instances[0].outlines.clone();
    let (s, c) = math::sin_cos(30f64.to_radians());
    for (a, b) in before.iter().flatten().zip(after.iter().flatten()) {
        let (x, y) = (a.0 - pivot.0, a.1 - pivot.1);
        let want = (pivot.0 + x * c - y * s, pivot.1 + x * s + y * c);
        assert!(
            (want.0 - b.0).abs() < 1e-6 && (want.1 - b.1).abs() < 1e-6,
            "{want:?} vs {b:?}"
        );
    }
    // And moved: by as much.
    app.compass_apply(&target, pivot, 0.0, Motion::Move((5.0, -7.0)));
    draw(&mut app);
    let moved = &app.scene.instances[0].outlines;
    for (a, b) in after.iter().flatten().zip(moved.iter().flatten()) {
        assert!((a.0 + 5.0 - b.0).abs() < 1e-6 && (a.1 - 7.0 - b.1).abs() < 1e-6);
    }
}
