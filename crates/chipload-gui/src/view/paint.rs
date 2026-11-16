// SPDX-License-Identifier: GPL-3.0-or-later
//! Painting helpers: 3D paths, shading, the turn axis, background, markers.

use chipload_core::Point;
use chipload_core::math;
use eframe::egui::{Color32, Mesh, Painter, Pos2, Rect, Shape, Stroke, Vec2};

use super::scene::{
    COMPASS_ARM_END_PX, COMPASS_ARM_START_PX, COMPASS_CENTRE_PX, COMPASS_RING_PX, CompassShape,
    CornerMarker, TabMarker,
};
use super::shapes::ToolpathShape;
use super::{DASH_PX, GAP_PX, Palette, TURN_AXIS_OVERHANG_PX, View};

/// The toolpath in 3D: rapids dashed, cutting moves (plunges included) shaded from light at
/// the highest cut to the plain colour at the deepest.
pub(super) fn paint_path_3d(
    painter: &Painter,
    view: &View,
    path: &ToolpathShape,
    feed: Color32,
    pal: &Palette,
) {
    let (high, low) = path.cut_z_range().unwrap_or((0.0, 0.0));
    let span = (high - low).max(f64::EPSILON);
    for &(a, b, cutting) in &path.lines3 {
        let ends = [view.to_screen3(a), view.to_screen3(b)];
        if cutting {
            let depth = ((high - (a.2 + b.2) / 2.0) / span).clamp(0.0, 1.0) as f32;
            painter.line_segment(ends, Stroke::new(2.0, shade(feed, depth, pal.shade_toward)));
        } else {
            painter.extend(Shape::dashed_line(
                &ends,
                Stroke::new(1.0, pal.rapid),
                DASH_PX,
                GAP_PX,
            ));
        }
    }
}

/// How far towards white the highest cut in 3D is lightened.
const SHADE_LIGHTEN: f32 = 0.5;

/// `color` moved towards `toward` (white on the dark canvas, black on the light one) at
/// depth 0, the plain colour at depth 1: the deepest cut stays as easy to see as a cut in
/// top view.
pub(super) fn shade(color: Color32, depth: f32, toward: Color32) -> Color32 {
    let k = SHADE_LIGHTEN * (1.0 - depth);
    let mix = |c: u8, t: u8| (f32::from(c) + (f32::from(t) - f32::from(c)) * k).round() as u8;
    Color32::from_rgb(
        mix(color.r(), toward.r()),
        mix(color.g(), toward.g()),
        mix(color.b(), toward.b()),
    )
}

/// The background gradient, from `background_top` down to `background`.
/// The turn axis: a long-dashed line from `a` to `b`, a little past the board at each end,
/// and its label beyond `b`, in the stock's colour.
pub(super) fn paint_turn_axis(
    painter: &Painter,
    view: &View,
    (a, b): (Point, Point),
    label: &str,
    pal: &Palette,
) {
    let (sa, sb) = (view.to_screen(a), view.to_screen(b));
    let along = (sb - sa).normalized();
    if !along.x.is_finite() || !along.y.is_finite() {
        return;
    }
    let (from, to) = (
        sa - along * TURN_AXIS_OVERHANG_PX,
        sb + along * TURN_AXIS_OVERHANG_PX,
    );
    let stroke = Stroke::new(1.5, pal.stock);
    painter.extend(Shape::dashed_line(
        &[from, to],
        stroke,
        3.0 * DASH_PX,
        GAP_PX,
    ));
    // The label past the end, away from the board.
    let anchor = if along.y.abs() > along.x.abs() {
        if along.y < 0.0 {
            eframe::egui::Align2::CENTER_BOTTOM
        } else {
            eframe::egui::Align2::CENTER_TOP
        }
    } else if along.x > 0.0 {
        eframe::egui::Align2::LEFT_CENTER
    } else {
        eframe::egui::Align2::RIGHT_CENTER
    };
    painter.text(
        to + along * 4.0,
        anchor,
        label,
        eframe::egui::FontId::proportional(12.0),
        pal.stock,
    );
}

pub(super) fn paint_background(painter: &Painter, pal: &Palette) {
    let rect = painter.clip_rect();
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), pal.background_top);
    mesh.colored_vertex(rect.right_top(), pal.background_top);
    mesh.colored_vertex(rect.right_bottom(), pal.background);
    mesh.colored_vertex(rect.left_bottom(), pal.background);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(mesh));
}

/// A ">" pointing along `angle` (world, CCW from +X); screen Y is down, so it flips.
pub(super) fn chevron(at: Pos2, angle: f64, color: Color32) -> Shape {
    let (s, c) = math::sin_cos(-angle);
    let rotate = |x: f64, y: f64| at + Vec2::new((x * c - y * s) as f32, (x * s + y * c) as f32);
    Shape::line(
        vec![rotate(-7.0, -5.0), rotate(0.0, 0.0), rotate(-7.0, 5.0)],
        Stroke::new(2.0, color),
    )
}

pub(super) fn segment_distance(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let len2 = ab.length_sq();
    let t = if len2 == 0.0 {
        0.0
    } else {
        ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
    };
    (p - (a + ab * t)).length()
}

/// A tab marker: a small square in the tab colour at the tab's centre, filled when the tab
/// was placed by hand.
pub(super) fn paint_tab_marker(painter: &Painter, view: &View, marker: &TabMarker, pal: &Palette) {
    let at = view.to_screen(marker.at);
    let square = Rect::from_center_size(at, Vec2::splat(10.0));
    let fill = if marker.tab.by_hand {
        pal.tab
    } else {
        pal.background
    };
    painter.rect_filled(square, 1.0, fill);
    painter.rect_stroke(
        square,
        1.0,
        Stroke::new(2.0, pal.tab),
        eframe::egui::StrokeKind::Middle,
    );
}

/// A corner marker: a small diamond, filled where a bone is cut (with the tool's circle at
/// the bone's tip), hollow where the corner stays round (amber where the tool can't reach
/// into it), and a ring round it where the corner has its own choice.
/// The Compass (docs/specs/compass.md §3): its arrows in the axes' colours, its ring and
/// its centre; all grey while unlocked, the part under the pointer drawn bolder. Sizes
/// match the hit areas in the app's compass module.
pub(super) fn paint_compass(painter: &Painter, view: &View, compass: &CompassShape, pal: &Palette) {
    const HEAD: f32 = 7.0;
    let (arm_start, arm_end, ring_r, centre_half) = (
        COMPASS_ARM_START_PX,
        COMPASS_ARM_END_PX,
        COMPASS_RING_PX,
        COMPASS_CENTRE_PX,
    );
    let grey = Color32::from_gray(140);
    let at = view.to_screen(compass.centre);
    let width = |k: usize| if compass.hot == Some(k) { 3.5 } else { 2.0 };
    for (k, degrees, colour) in [
        (0, compass.angle_deg, pal.axis_x),
        (1, compass.angle_deg + 90.0, pal.axis_y),
    ] {
        let colour = if compass.unlocked { grey } else { colour };
        let (sin, cos) = math::sin_cos(degrees.to_radians());
        let u = Vec2::new(cos as f32, -(sin as f32));
        let side = Vec2::new(-u.y, u.x);
        let (start, end) = (at + u * arm_start, at + u * arm_end);
        painter.line_segment([start, end - u * HEAD], Stroke::new(width(k), colour));
        painter.add(Shape::convex_polygon(
            vec![
                end,
                end - u * HEAD * 1.6 + side * HEAD * 0.7,
                end - u * HEAD * 1.6 - side * HEAD * 0.7,
            ],
            colour,
            Stroke::NONE,
        ));
    }
    let ring = if compass.unlocked { grey } else { pal.selected };
    painter.circle_stroke(at, ring_r, Stroke::new(width(2), ring));
    let square = Rect::from_center_size(at, Vec2::splat(2.0 * centre_half));
    painter.rect_filled(square, 0.0, pal.handle_fill);
    painter.rect_stroke(
        square,
        0.0,
        Stroke::new(width(3), ring),
        eframe::egui::StrokeKind::Middle,
    );
}

pub(super) fn paint_corner_marker(
    painter: &Painter,
    view: &View,
    marker: &CornerMarker,
    pal: &Palette,
) {
    let at = view.to_screen(marker.at);
    if let Some((centre, radius)) = marker.notch {
        let r = (radius * view.scale) as f32;
        painter.circle_stroke(view.to_screen(centre), r, Stroke::new(1.0, pal.operation));
    }
    let d = 5.0;
    let diamond = vec![
        at + Vec2::new(0.0, -d),
        at + Vec2::new(d, 0.0),
        at + Vec2::new(0.0, d),
        at + Vec2::new(-d, 0.0),
    ];
    let fill = if marker.notch.is_some() {
        pal.operation
    } else {
        pal.background
    };
    // A corner the tool can't reach: hollow, in the colour of what isn't cut.
    let edge = if marker.unreached {
        pal.open
    } else {
        pal.operation
    };
    painter.add(Shape::convex_polygon(diamond, fill, Stroke::new(1.5, edge)));
    if marker.chosen {
        painter.circle_stroke(at, 9.0, Stroke::new(1.5, pal.operation));
    }
}
