// SPDX-License-Identifier: GPL-3.0-or-later
//! The canvas's transform, shapes and markers.

use chipload_job::Spindle;
use chipload_toolpath::OpSpan;
use chipload_toolpath::model::{Move, Program};

use super::paint::shade;
use super::shapes::Plunge;
use super::*;

#[test]
fn a_turned_board_shows_its_points_mirrored_and_upside_down() {
    // docs/specs/two-sided.md §10.2: about X = 150, a 10 mm board from Z0 to Z-10.
    let turned = Turned {
        left_right: true,
        at: 150.0,
        z_sum: -10.0,
    };
    assert_eq!(turned.point((140.0, 30.0, -2.0)), (160.0, 30.0, -8.0));
    assert_eq!(
        turned.point(turned.point((1.0, 2.0, -3.0))),
        (1.0, 2.0, -3.0)
    );
    let mut view = View {
        turn: 0.4,
        tilt: 0.6,
        turned: Some(turned),
        ..View::default()
    };
    // 2D items sit on the face that is up: the old bottom.
    view.top_z = -10.0;
    for p in [(40.0, 30.0), (150.0, 75.0), (260.0, 170.0)] {
        let back = view.to_world(view.to_screen(p));
        // Screen positions are f32: a micrometre or so.
        assert!(
            (back.0 - p.0).abs() < 1e-3 && (back.1 - p.1).abs() < 1e-3,
            "{p:?} {back:?}"
        );
    }
    // The old bottom is up: a point on it shows above one on the old top.
    let bottom = view.to_screen3((100.0, 50.0, -10.0));
    let top = view.to_screen3((100.0, 50.0, 0.0));
    assert!(bottom.y < top.y, "{bottom:?} {top:?}");
    // A raw point isn't turned again.
    assert_eq!(
        view.to_screen3_raw((200.0, 50.0, 0.0)),
        view.to_screen3((100.0, 50.0, -10.0))
    );
}

#[test]
fn the_surface_leaves_holes_and_stands_on_walls() {
    // 4 × 4 cells, the middle one cut through.
    let mut surface = SimSurface {
        origin: (0.0, 0.0),
        cell: 1.0,
        columns: 4,
        rows: 4,
        heights: vec![0.0; 16],
        floors: vec![-10.0; 16],
        colors: vec![Color32::WHITE; 16],
        wall: Color32::GRAY,
        raw: false,
    };
    let view = View {
        turn: 0.3,
        tilt: 0.7,
        ..View::default()
    };
    let solid = surface.mesh(&view, false).indices.len() / 6;
    surface.floors[5] = 0.0;
    let holed = surface.mesh(&view, false).indices.len() / 6;
    // 9 quads, less the 4 round the hole's vertex, plus walls facing the viewer.
    assert!(holed < solid + 8, "{solid} {holed}");
    let top_quads = (0..3)
        .flat_map(|j| (0..3).map(move |i| (i, j)))
        .filter(|&(i, j)| surface.kept(i, j))
        .count();
    assert_eq!(top_quads, 5);
    // Seen from straight above: no walls, only the top.
    assert_eq!(
        surface.mesh(&View::default(), false).indices.len() / 6,
        top_quads
    );
}

/// Every line and mark on the canvas stands out from it by at least 3:1 (WCAG's minimum
/// for graphics), faded and 3D-shaded ones included; the measure text by 4.5:1. In both
/// palettes, against both ends of the background gradient.
#[test]
fn canvas_colours_stand_out_from_the_background() {
    use crate::style::contrast;
    let over = |c: Color32, bg: Color32| bg.blend(c);
    for (name, pal) in [("dark", Palette::DARK), ("light", Palette::LIGHT)] {
        for background in [pal.background_top, pal.background] {
            let faded = |c: Color32| over(c.gamma_multiply(GHOST_FADE), background);
            let lines = [
                ("outline", pal.outer),
                ("hole", pal.hole),
                ("open", pal.open),
                ("invalid", pal.invalid),
                ("stock", pal.stock),
                ("feed", pal.feed),
                ("rejected feed", pal.feed_rejected),
                ("rapid", pal.rapid),
                ("label", pal.label),
                ("axis x", pal.axis_x),
                ("axis y", pal.axis_y),
                ("axis z", pal.axis_z),
                ("selected", pal.selected),
                ("snap", pal.snap),
                ("measure", pal.measure),
                ("middle handle", pal.middle_handle),
                ("tab", pal.tab),
                ("copies", pal.ghost),
                ("operation", pal.operation),
                ("tool", pal.tool_mark),
                ("stale feed", faded(pal.feed)),
                ("stale rapid", faded(pal.rapid)),
                ("deepest cut in 3D", shade(pal.feed, 1.0, pal.shade_toward)),
                (
                    "deepest rejected cut in 3D",
                    shade(pal.feed_rejected, 1.0, pal.shade_toward),
                ),
                ("highest cut in 3D", shade(pal.feed, 0.0, pal.shade_toward)),
            ];
            for (what, color) in lines {
                let ratio = contrast(color, background);
                assert!(ratio >= 3.0, "{name}: {what} is {ratio:.2}:1 on the canvas");
            }
            let behind_measure = over(pal.measure_back, background);
            let ratio = contrast(pal.measure, behind_measure);
            assert!(ratio >= 4.5, "{name}: measure text {ratio:.2}:1");
        }
        // The grid stays in the background: fainter than any line drawn on it.
        let bottom = pal.background;
        let major = contrast(over(pal.grid_major, bottom), bottom);
        assert!(major < 1.6, "{name}: the grid shouts ({major:.2}:1)");
    }
}

#[test]
fn world_and_screen_round_trip_with_y_up() {
    let view = View {
        scale: 4.0,
        offset: (100.0, 300.0),
        ..View::default()
    };
    let p = view.to_screen((10.0, 20.0));
    assert_eq!(p, Pos2::new(140.0, 220.0)); // up in the world is up on screen
    let back = view.to_world(p);
    assert!((back.0 - 10.0).abs() < 1e-9 && (back.1 - 20.0).abs() < 1e-9);
}

#[test]
fn zoom_keeps_the_point_under_the_cursor() {
    let mut view = View::default();
    let anchor = Pos2::new(321.0, 123.0);
    let before = view.to_world(anchor);
    view.zoom_at(anchor, 3.0);
    let after = view.to_world(anchor);
    assert!((before.0 - after.0).abs() < 1e-9 && (before.1 - after.1).abs() < 1e-9);
}

/// docs/specs/canvas-resize.md: an area shown is the area visible, and in a narrower
/// screen all of it still shows, centred.
#[test]
fn an_area_shown_is_the_area_visible() {
    let mut view = View::default();
    let screen = Rect::from_min_size(Pos2::new(10.0, 20.0), Vec2::new(400.0, 200.0));
    let area = Area {
        across: (-50.0, 150.0),
        up: (0.0, 100.0),
    };
    view.show_area(area, screen);
    let seen = view.visible_area(screen);
    let close = |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9;
    assert!(
        close(seen.across, area.across) && close(seen.up, area.up),
        "{seen:?}"
    );
    let narrow = Rect::from_min_size(Pos2::new(10.0, 20.0), Vec2::new(200.0, 200.0));
    view.show_area(area, narrow);
    let seen = view.visible_area(narrow);
    assert!(close(seen.across, (-50.0, 150.0)), "{seen:?}");
    assert!(close(seen.up, (-50.0, 150.0)), "{seen:?}");
}

#[test]
fn fit_puts_the_bounds_inside_the_screen() {
    let mut view = View::default();
    let screen = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(800.0, 600.0));
    view.fit(&[(-50.0, 0.0, 0.0), (150.0, 100.0, 0.0)], screen);
    for corner in [(-50.0, 0.0), (150.0, 100.0)] {
        assert!(screen.contains(view.to_screen(corner)));
    }
}

// -- 3D (docs/specs/3d-view.md)

fn tilted() -> View {
    let mut view = View {
        scale: 4.0,
        offset: (100.0, 300.0),
        top_z: 0.0,
        ..View::default()
    };
    view.orient(-30f64.to_radians(), 55f64.to_radians(), (50.0, 50.0));
    view
}

#[test]
fn top_view_projects_exactly_as_the_2d_view_did() {
    let view = View {
        scale: 4.0,
        offset: (100.0, 300.0),
        top_z: -3.0,
        ..View::default()
    };
    for p in [(10.0, 20.0), (-7.25, 1e3), (0.1, -0.3)] {
        let old = Pos2::new((100.0 + p.0 * 4.0) as f32, (300.0 - p.1 * 4.0) as f32);
        assert_eq!(view.to_screen(p), old);
        // Z doesn't matter looking straight down.
        assert_eq!(view.to_screen3((p.0, p.1, -12.0)), old);
    }
}

#[test]
fn tilting_lifts_higher_points_and_picking_still_works() {
    let view = tilted();
    let low = view.to_screen3((40.0, 30.0, -10.0));
    let high = view.to_screen3((40.0, 30.0, 0.0));
    assert!(high.y < low.y, "higher is further up the screen");
    // A click lands on the stock top where that point is drawn.
    let p = (40.0, 30.0);
    let back = view.to_world(view.to_screen(p));
    assert!(
        (back.0 - p.0).abs() < 1e-3 && (back.1 - p.1).abs() < 1e-3,
        "{back:?}"
    );
    // Zoom keeps the stock-top point under the cursor, tilted too.
    let mut zoomed = view;
    let anchor = Pos2::new(321.0, 123.0);
    let before = zoomed.to_world(anchor);
    zoomed.zoom_at(anchor, 2.5);
    let after = zoomed.to_world(anchor);
    assert!((before.0 - after.0).abs() < 1e-6 && (before.1 - after.1).abs() < 1e-6);
}

#[test]
fn orbiting_keeps_the_pivot_and_clamps_the_tilt() {
    let mut view = tilted();
    let pivot = (50.0, 50.0);
    let at = view.to_screen(pivot);
    view.orbit(Vec2::new(40.0, -1000.0), pivot);
    assert_eq!(view.tilt, MAX_TILT_RAD);
    assert!(view.to_screen(pivot).distance(at) < 1e-3);
    view.orbit(Vec2::new(-10_000.0, 1000.0), pivot);
    assert_eq!(view.tilt, 0.0);
    assert!((0.0..std::f64::consts::TAU).contains(&view.turn));
}

/// Issue #16: the model turns with the mouse, as in many CAD programs.
#[test]
fn a_drag_turns_the_model_with_the_mouse() {
    let mut view = tilted();
    let pivot = (50.0, 50.0);
    let front = (50.0, 0.0); // the model's front, towards the viewer
    let before = view.to_screen(front);
    view.orbit(Vec2::new(10.0, 0.0), pivot);
    assert!(
        view.to_screen(front).x > before.x,
        "right: the front goes right"
    );
    let tilt = view.tilt;
    view.orbit(Vec2::new(0.0, 10.0), pivot);
    assert!(
        view.tilt < tilt,
        "down: the top tips towards you, nearer the top view"
    );
}

#[test]
fn fit_in_3d_keeps_the_stock_box_on_screen() {
    let mut view = tilted();
    let screen = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(800.0, 600.0));
    let mut corners = Vec::new();
    for z in [0.0, -18.0] {
        corners.extend([
            (0.0, 0.0, z),
            (300.0, 0.0, z),
            (300.0, 200.0, z),
            (0.0, 200.0, z),
        ]);
    }
    view.fit(&corners, screen);
    for &c in &corners {
        assert!(screen.contains(view.to_screen3(c)), "{c:?}");
    }
}

#[test]
fn the_toolpath_keeps_its_depths_for_3d() {
    let program = Program {
        tool_id: 1,
        tool_diameter: 6.0,
        rpm: 18000.0,
        spindle: Spindle::Cw,
        safe_z: 5.0,
        moves: vec![
            Move::rapid_z(5.0),
            Move::rapid_xy((0.0, 0.0)),
            Move::feed_z(-2.0, 300.0),
            Move::feed_xy((10.0, 0.0), 1000.0),
            Move::feed_z(-4.0, 300.0),
            Move::feed_xy((0.0, 0.0), 1000.0),
        ],
        comments: Vec::new(),
    };
    let shape = ToolpathShape::new(&program, false, &[]);
    assert!(
        shape
            .lines3
            .contains(&((0.0, 0.0, 5.0), (0.0, 0.0, -2.0), true)),
        "plunge"
    );
    assert!(
        shape
            .lines3
            .contains(&((0.0, 0.0, -2.0), (10.0, 0.0, -2.0), true))
    );
    assert!(
        shape
            .lines3
            .contains(&((10.0, 0.0, -4.0), (0.0, 0.0, -4.0), true))
    );
    assert_eq!(shape.cut_z_range(), Some((5.0, -4.0)));
}

fn square(id: usize, x: f64) -> ContourShape {
    ContourShape {
        id,
        kind: Kind::Outer,
        points: vec![
            (x, 0.0),
            (x + 10.0, 0.0),
            (x + 10.0, 10.0),
            (x, 10.0),
            (x, 0.0),
        ],
    }
}

#[test]
fn picking_takes_the_nearest_contour_within_tolerance() {
    let scene = Scene {
        contours: vec![square(1, 0.0), square(2, 12.0)],
        ..Default::default()
    };
    let view = View {
        scale: 10.0,
        offset: (0.0, 500.0),
        ..View::default()
    };
    // 0.3 mm = 3 px right of square 1's right edge, 1.7 mm from square 2.
    assert_eq!(scene.pick(&view, view.to_screen((10.3, 5.0))), Some(1));
    assert_eq!(scene.pick(&view, view.to_screen((11.8, 5.0))), Some(2));
    // In the middle of square 1: farther than 6 px from any line.
    assert_eq!(scene.pick(&view, view.to_screen((5.0, 5.0))), None);
}

#[test]
fn toolpath_pieces() {
    let program = Program {
        tool_id: 1,
        tool_diameter: 6.0,
        rpm: 18000.0,
        spindle: Spindle::Cw,
        safe_z: 5.0,
        moves: vec![
            Move::rapid_z(5.0),
            Move::rapid_xy((0.0, 0.0)),
            Move::feed_z(-1.0, 300.0),
            Move::feed_xy((100.0, 0.0), 1000.0),
            Move::feed_xy((100.0, 10.0), 1000.0),
            Move::rapid_z(5.0),
            Move::rapid_xy((50.0, 50.0)),
            Move::feed_z(-1.0, 300.0),
            Move::feed_xy((60.0, 50.0), 1000.0),
        ],
        comments: Vec::new(),
    };
    let spans = [
        OpSpan {
            operation: 0,
            pass: None,
            copy: 1,
            moves: 0..6,
            tool_radius: 3.0,
        },
        OpSpan {
            operation: 3,
            pass: None,
            copy: 1,
            moves: 6..9,
            tool_radius: 3.0,
        },
    ];
    let shape = ToolpathShape::new(&program, false, &spans);
    let plunge = |at, operation| Plunge { at, operation };
    assert_eq!(
        shape.starts,
        [plunge((0.0, 0.0), Some(0)), plunge((50.0, 50.0), Some(3))]
    );
    assert_eq!(shape.rapids, [((100.0, 10.0), (50.0, 50.0))]);
    assert_eq!(
        shape.feeds,
        [
            vec![(0.0, 0.0), (100.0, 0.0), (100.0, 10.0)],
            vec![(50.0, 50.0), (60.0, 50.0)]
        ]
    );
    // One arrow at the first cut of each ring; the 10 mm stretch comes 100 mm later,
    // past the spacing, so it gets one too.
    assert_eq!(shape.arrows.len(), 3);
    assert_eq!(shape.arrows[0], ((50.0, 0.0), 0.0));
}
