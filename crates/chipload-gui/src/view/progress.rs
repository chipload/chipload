// SPDX-License-Identifier: GPL-3.0-or-later
//! Playback progress: the cut so far, the tool, what is still to come.

use chipload_core::math;
use eframe::egui::{Color32, Painter, Pos2, Shape, Stroke};

use super::paint::shade;
use super::shapes::ToolpathShape;
use super::{DASH_PX, GAP_PX, Palette, Point3, View};

/// Where playback is (docs/specs/playback.md §1): the moves done, the one under way and
/// the tool on it.
#[derive(Debug, Clone, PartialEq)]
pub struct Progress {
    /// Moves `0..done` are done.
    pub done: usize,
    /// The tool's point on move `done` (it is under way), if known.
    pub tool_at: Option<Point3>,
    pub tool: ToolLook,
}

/// The tool's outline for the playback marker.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToolLook {
    /// The cutting end's shape, out to the tool's radius.
    pub cutter: chipload_core::cutter::Cutter,
    /// A back setup's tool: it cuts the board from below, its shank down
    /// (docs/specs/two-sided.md §10.2).
    pub from_below: bool,
}

/// Points along the drawn tool's profile, from its axis out.
const TOOL_PROFILE_STEPS: usize = 8;

/// How faint the moves still to come are drawn.
pub(super) const TO_COME_FADE: f32 = 0.3;

impl Progress {
    /// The moves done over the faint toolpath, the move under way up to the tool, and the
    /// tool.
    pub(super) fn paint(
        &self,
        painter: &Painter,
        view: &View,
        path: &ToolpathShape,
        pal: &Palette,
    ) {
        let (high, low) = path.cut_z_range().unwrap_or((0.0, 0.0));
        let span = (high - low).max(f64::EPSILON);
        let flat = view.is_top();
        for (k, &(a, b, cutting)) in path.lines3.iter().enumerate() {
            let index = path.line_moves.get(k).copied().unwrap_or(usize::MAX);
            let b = if index < self.done {
                b
            } else if index == self.done
                && let Some(t) = self.tool_at
            {
                t
            } else {
                continue;
            };
            let ends = [view.to_screen3(a), view.to_screen3(b)];
            if cutting {
                let color = if flat {
                    pal.feed
                } else {
                    let depth = ((high - (a.2 + b.2) / 2.0) / span).clamp(0.0, 1.0) as f32;
                    shade(pal.feed, depth, pal.shade_toward)
                };
                painter.line_segment(ends, Stroke::new(2.0, color));
            } else if !flat || (a.0, a.1) != (b.0, b.1) {
                painter.extend(Shape::dashed_line(
                    &ends,
                    Stroke::new(1.0, pal.rapid),
                    DASH_PX,
                    GAP_PX,
                ));
            }
        }
        if let Some(at) = self.tool_at {
            self.paint_tool(painter, view, at, pal.tool_mark);
        }
    }

    /// A circle of the tool's size at its point; in 3D also its outline: a cylinder, or a
    /// cone on a cylinder.
    pub(super) fn paint_tool(
        &self,
        painter: &Painter,
        view: &View,
        (x, y, z): Point3,
        mark: Color32,
    ) {
        let r = self.tool.cutter.radius();
        let stroke = Stroke::new(2.0, mark);
        let ring = |z: f64, radius: f64| -> Vec<Pos2> {
            (0..=32)
                .map(|k| {
                    let a = k as f64 / 32.0 * std::f64::consts::TAU;
                    view.to_screen3((x + radius * math::cos(a), y + radius * math::sin(a), z))
                })
                .collect()
        };
        if view.is_top() {
            painter.add(Shape::line(ring(z, r), stroke));
            painter.circle_filled(view.to_screen3((x, y, z)), 2.5, mark);
            return;
        }
        // The cutting end as its profile: the height above the tip at each distance out
        // (flat, a cone, a ball or a tapered ball), then the shank.
        let up = if self.tool.from_below { -1.0 } else { 1.0 };
        let cutter = self.tool.cutter;
        let profile: Vec<(f64, f64)> = (0..=TOOL_PROFILE_STEPS)
            .map(|k| {
                let d = r * k as f64 / TOOL_PROFILE_STEPS as f64;
                (d, cutter.height_at(d).unwrap_or(0.0))
            })
            .collect();
        // Where the end is flat: its rim is drawn as a ring.
        let tip = profile
            .iter()
            .take_while(|(_, h)| *h <= chipload_core::tolerances::Z_NOISE_MM)
            .last()
            .map_or(0.0, |(d, _)| *d);
        let rise = profile.last().map_or(0.0, |(_, h)| *h);
        let shank = (z + up * rise, z + up * (rise + 3.0 * r));
        if tip > 0.0 {
            painter.add(Shape::line(ring(z, tip), stroke));
        }
        painter.add(Shape::line(ring(shank.0, r), stroke));
        painter.add(Shape::line(ring(shank.1, r), stroke));
        for k in 0..4 {
            let a = k as f64 * std::f64::consts::FRAC_PI_2 + view.turn;
            let (c, s) = (math::cos(a), math::sin(a));
            let at = |radius: f64, z: f64| view.to_screen3((x + radius * c, y + radius * s, z));
            let side: Vec<Pos2> = profile
                .iter()
                .map(|&(d, h)| at(d, z + up * h))
                .chain(std::iter::once(at(r, shank.1)))
                .collect();
            painter.add(Shape::line(side, stroke));
        }
    }
}
