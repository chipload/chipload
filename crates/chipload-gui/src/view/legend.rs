// SPDX-License-Identifier: GPL-3.0-or-later
//! The legend: a swatch and a name for each kind of line shown.

use eframe::egui::{Color32, Painter, Rect, Shape, Stroke, Vec2};

use super::Palette;
use super::scene::Scene;
use crate::i18n::{Lang, t};

/// What a legend entry looks like: drawn the way the canvas draws that thing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Swatch {
    Line(Color32, f32),
    Dashed(Color32),
    /// A plunge: the ring the canvas draws where the tool goes down.
    Plunge,
    /// A ring's start marker.
    Start,
    /// A corner marker with its bone.
    Corner,
}

/// What the canvas shows, explained: contours always; the toolpath and what goes with it once
/// generated; copies when there are some.
pub fn legend(scene: &Scene, lang: &impl Lang) -> Vec<(Swatch, String)> {
    let pal = &scene.palette;
    let mut items = vec![
        (
            Swatch::Line(pal.outer, 1.5),
            t!(lang, "view-legend-outline"),
        ),
        (Swatch::Line(pal.hole, 1.5), t!(lang, "view-legend-hole")),
        (Swatch::Dashed(pal.open), t!(lang, "view-legend-open")),
        (
            Swatch::Line(pal.selected, 3.0),
            t!(lang, "view-legend-selected"),
        ),
        (
            Swatch::Line(pal.operation, 6.0),
            t!(lang, "view-legend-operation"),
        ),
        (Swatch::Dashed(pal.stock), t!(lang, "view-legend-stock")),
    ];
    // A setup's space draws its instances, not the shapes: those two, and the board
    // (docs/specs/drawing-and-setup-spaces.md §4).
    if scene.shapes_hidden {
        items = vec![
            (
                Swatch::Line(pal.outer, 1.5),
                t!(lang, "view-legend-instances"),
            ),
            (
                Swatch::Line(pal.selected, 1.5),
                t!(lang, "view-legend-selected"),
            ),
            (Swatch::Dashed(pal.stock), t!(lang, "view-legend-stock")),
        ];
    }
    if !scene.mirrored.is_empty() {
        items.push((Swatch::Dashed(pal.ghost), t!(lang, "view-legend-mirrored")));
    }
    if let Some((color, _)) = &scene.rim {
        items.push((Swatch::Dashed(*color), t!(lang, "view-legend-rim")));
    }
    if !scene.corner_markers.is_empty() {
        items.push((Swatch::Corner, t!(lang, "view-legend-corner")));
    }
    if let Some(path) = scene.toolpath.as_ref().filter(|_| !scene.hide_toolpath) {
        let feed = if path.rejected {
            pal.feed_rejected
        } else {
            pal.feed
        };
        items.push((Swatch::Line(feed, 2.0), t!(lang, "view-legend-cutting")));
        items.push((Swatch::Dashed(pal.rapid), t!(lang, "view-legend-rapid")));
        items.push((Swatch::Plunge, t!(lang, "view-legend-plunge")));
        if !scene.tabs.is_empty() {
            items.push((Swatch::Line(pal.tab, 6.0), t!(lang, "view-legend-tab")));
        }
        if !scene.start_markers.is_empty() {
            items.push((Swatch::Start, t!(lang, "view-legend-ring-start")));
        }
    }
    items
}

/// Paint a swatch in `rect`, on the canvas background.
pub fn paint_swatch(painter: &Painter, rect: Rect, swatch: Swatch, pal: &Palette) {
    painter.rect_filled(rect, 2.0, pal.background);
    let (left, right) = (rect.left_center(), rect.right_center());
    let (a, b) = (left + Vec2::new(3.0, 0.0), right - Vec2::new(3.0, 0.0));
    match swatch {
        Swatch::Line(color, width) => {
            painter.line_segment([a, b], Stroke::new(width.min(rect.height() - 4.0), color));
        }
        Swatch::Dashed(color) => {
            painter.extend(Shape::dashed_line(
                &[a, b],
                Stroke::new(1.5, color),
                4.0,
                3.0,
            ));
        }
        Swatch::Plunge => {
            painter.circle_stroke(rect.center(), 5.0, Stroke::new(2.0, pal.label));
        }
        Swatch::Start => {
            painter.circle_stroke(rect.center(), 5.0, Stroke::new(2.0, pal.operation));
        }
        Swatch::Corner => {
            let c = rect.center();
            let d = 5.0;
            painter.add(Shape::convex_polygon(
                vec![
                    c + Vec2::new(0.0, -d),
                    c + Vec2::new(d, 0.0),
                    c + Vec2::new(0.0, d),
                    c + Vec2::new(-d, 0.0),
                ],
                pal.operation,
                Stroke::new(1.5, pal.operation),
            ));
        }
    }
}
