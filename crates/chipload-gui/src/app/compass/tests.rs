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
