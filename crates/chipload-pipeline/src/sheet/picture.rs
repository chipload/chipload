// SPDX-License-Identifier: GPL-3.0-or-later
//! The sheet's picture: the stock, the zero, and each tool's path in its colour.

use std::fmt::Write as _;

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_i18n::Message;
use chipload_job::{Job, Operation, Turn};
use chipload_toolpath::ToolGroup;
use chipload_toolpath::model::{MoveKind, Program};
use chipload_toolpath::select::{select_contours, select_paths};

use super::{Words, svg_num};
use crate::RunResult;

/// Colours and dashes of the files (or of the tools in one file), told apart in black and
/// white too (§5, decision 4).
const COLOURS: [&str; 6] = [
    "#1f5fbf", "#c0392b", "#1e8449", "#7d3c98", "#b9770e", "#117a8b",
];
const DASHES: [&str; 4] = ["", "6 3", "2 2", "9 3 2 3"];

/// The top view: stock, parts, each file's cutting moves, tabs and the X0 Y0 mark (§2.4).
pub(super) fn picture(html: &mut String, job: &Job, result: &RunResult, w: Words) {
    let ((sx0, sy0), (sx1, sy1)) = job.stock_bounds();
    // A back setup is drawn as its board lies on the machine, back face up: the shapes, the
    // toolpath and the tabs mirrored as its program is (docs/specs/two-sided.md §8). The
    // board maps onto itself, and the zero is where it is on the machine.
    let turned = |p: Point| match job.back_turn() {
        Some(Turn::LeftRight) => (sx0 + sx1 - p.0, p.1),
        Some(Turn::TopBottom) => (p.0, sy0 + sy1 - p.1),
        None => p,
    };
    let program = crate::on_the_board(job, &result.program);
    let mut bounds = Bounds::new((sx0, sy0));
    bounds.add((sx1, sy1));
    let zero = job.work_zero_xy();
    bounds.add(zero);
    for m in &program.moves {
        if let (Some(x), Some(y)) = (m.x, m.y) {
            bounds.add((x, y));
        }
    }
    let size = (bounds.x1 - bounds.x0).max(bounds.y1 - bounds.y0).max(1.0);
    let margin = size * 0.06;
    let _ = writeln!(
        html,
        "<figure>\n<svg class=\"picture\" xmlns=\"http://www.w3.org/2000/svg\" \
         viewBox=\"{} {} {} {}\" preserveAspectRatio=\"xMidYMid meet\">",
        svg_num(bounds.x0 - margin),
        svg_num(-(bounds.y1 + margin)),
        svg_num(bounds.x1 - bounds.x0 + 2.0 * margin),
        svg_num(bounds.y1 - bounds.y0 + 2.0 * margin)
    );
    let _ = writeln!(
        html,
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#f4efe6\" stroke=\"#555\" \
         stroke-width=\"1\" vector-effect=\"non-scaling-stroke\"/>",
        svg_num(sx0),
        svg_num(-sy1),
        svg_num(sx1 - sx0),
        svg_num(sy1 - sy0)
    );
    // The drawing, at every copy of every part: with one part the whole drawing as
    // before; with several, each part's copies of the contours its operations cut.
    let mut parts = String::new();
    for (contour, shifts) in contour_copies(job, &result.contours) {
        let points = contour.points();
        for (dx, dy) in shifts {
            let moved: Vec<Point> = points
                .iter()
                .map(|p| turned((p.0 + dx, p.1 + dy)))
                .collect();
            path_data(&mut parts, &moved, contour.closed);
        }
    }
    // Turned and mirrored instances, where they are cut (docs/specs/instances.md §3).
    for (points, closed) in crate::parts::placed_outlines(job, &result.contours) {
        let placed: Vec<Point> = points.into_iter().map(turned).collect();
        path_data(&mut parts, &placed, closed);
    }
    let _ = writeln!(
        html,
        "<path d=\"{}\" fill=\"none\" stroke=\"#999\" stroke-width=\"1\" \
         vector-effect=\"non-scaling-stroke\"/>",
        parts.trim_end()
    );
    for (i, group) in result.groups.iter().enumerate() {
        let d = cutting_path(&program, group);
        let dash = match DASHES[i % DASHES.len()] {
            "" => String::new(),
            dash => format!(" stroke-dasharray=\"{dash}\""),
        };
        let _ = writeln!(
            html,
            "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.3\"{dash} \
             stroke-linejoin=\"round\" vector-effect=\"non-scaling-stroke\"/>",
            d.trim_end(),
            COLOURS[i % COLOURS.len()]
        );
    }
    let mut tabs = String::new();
    for tab in &result.tabs {
        let tab: Vec<Point> = tab.iter().map(|p| turned(*p)).collect();
        path_data(&mut tabs, &tab, false);
    }
    if !tabs.is_empty() {
        let _ = writeln!(
            html,
            "<path d=\"{}\" fill=\"none\" stroke=\"#e67e22\" stroke-width=\"5\" \
             stroke-linecap=\"round\" vector-effect=\"non-scaling-stroke\"/>",
            tabs.trim_end()
        );
    }
    zero_mark(html, zero, size);
    html.push_str("</svg>\n<figcaption class=\"legend\">");
    let several = result.files.len() > 1;
    for (i, group) in result.groups.iter().enumerate() {
        let label = if several {
            w.text(
                &Message::new("sheet-legend-file")
                    .arg("number", i + 1)
                    .arg("tool", group.tool.id),
            )
        } else {
            format!("T{}", group.tool.id)
        };
        let dash = match DASHES[i % DASHES.len()] {
            "" => String::new(),
            dash => format!(" stroke-dasharray=\"{dash}\""),
        };
        let _ = write!(
            html,
            "<span><svg width=\"36\" height=\"10\"><line x1=\"0\" y1=\"5\" x2=\"36\" y2=\"5\" \
             stroke=\"{}\" stroke-width=\"2\"{dash}/></svg> {label}</span>",
            COLOURS[i % COLOURS.len()]
        );
    }
    if !result.tabs.is_empty() {
        let _ = write!(
            html,
            "<span><svg width=\"36\" height=\"10\"><line x1=\"4\" y1=\"5\" x2=\"32\" y2=\"5\" \
             stroke=\"#e67e22\" stroke-width=\"5\" stroke-linecap=\"round\"/></svg> {}</span>",
            w.text(&Message::new("sheet-legend-tabs"))
        );
    }
    let _ = writeln!(
        html,
        "<span>{}</span></figcaption>\n</figure>",
        w.text(&Message::new("sheet-legend-grey"))
    );
}

/// The X0 Y0 mark: a circle with arrows along +X and +Y.
fn zero_mark(html: &mut String, (zx, zy): (f64, f64), size: f64) {
    let arrow = size * 0.08;
    let head = arrow * 0.25;
    let font = size * 0.03;
    let (x, y) = (zx, -zy);
    let _ = writeln!(
        html,
        "<g stroke=\"#000\" stroke-width=\"2\" vector-effect=\"non-scaling-stroke\" fill=\"none\">\
         <circle cx=\"{x0}\" cy=\"{y0}\" r=\"{r}\"/>\
         <path d=\"M{x0} {y0} H{xe} M{xh} {yu} L{xe} {y0} L{xh} {yd} \
         M{x0} {y0} V{ye} M{xl} {yh} L{x0} {ye} L{xr} {yh}\"/></g>\n\
         <g font-size=\"{font}\" font-family=\"sans-serif\" fill=\"#000\">\
         <text x=\"{tx}\" y=\"{ty}\">X+</text><text x=\"{ux}\" y=\"{uy}\">Y+</text>\
         <text x=\"{zx_}\" y=\"{zy_}\">X0 Y0</text></g>",
        x0 = svg_num(x),
        y0 = svg_num(y),
        r = svg_num(head * 0.6),
        xe = svg_num(x + arrow),
        xh = svg_num(x + arrow - head),
        yu = svg_num(y - head * 0.5),
        yd = svg_num(y + head * 0.5),
        ye = svg_num(y - arrow),
        yh = svg_num(y - arrow + head),
        xl = svg_num(x - head * 0.5),
        xr = svg_num(x + head * 0.5),
        font = svg_num(font),
        tx = svg_num(x + arrow + font * 0.3),
        ty = svg_num(y + font * 0.35),
        ux = svg_num(x - font * 0.5),
        uy = svg_num(y - arrow - font * 0.4),
        zx_ = svg_num(x + font * 0.4),
        zy_ = svg_num(y + font * 1.3),
    );
}

/// A group's cutting moves as path data: a new subpath after each rapid.
fn cutting_path(program: &Program, group: &ToolGroup) -> String {
    let mut d = String::new();
    let (mut x, mut y) = (None, None);
    let mut drawing = false;
    for (i, m) in program.moves.iter().enumerate() {
        let from = (x, y);
        x = m.x.or(x);
        y = m.y.or(y);
        if !group.moves.contains(&i) || m.kind == MoveKind::Rapid {
            drawing = false;
            continue;
        }
        let (Some(tx), Some(ty)) = (x, y) else {
            continue;
        };
        if (Some(tx), Some(ty)) == (from.0, from.1) {
            continue; // straight down or up: nothing to see from above
        }
        if !drawing {
            if let (Some(fx), Some(fy)) = from {
                let _ = write!(d, "M{} {} ", svg_num(fx), svg_num(-fy));
            } else {
                let _ = write!(d, "M{} {} ", svg_num(tx), svg_num(-ty));
            }
            drawing = true;
        }
        let _ = write!(d, "L{} {} ", svg_num(tx), svg_num(-ty));
    }
    d
}

/// Each contour with the shifts it is drawn at: every copy of the parts that cut it
/// (docs/specs/parts.md §6), the part itself and its instances only moved. With one part, every contour at every copy, as before parts.
/// A contour no part cuts is drawn once, where it is.
pub fn contour_copies<'a>(job: &Job, contours: &'a [Contour]) -> Vec<(&'a Contour, Vec<Point>)> {
    // Turned and mirrored instances are drawn by crate::parts::placed_outlines.
    let shifts_of = |part: &chipload_job::Part| -> Vec<Point> {
        job.spots(part.id)
            .into_iter()
            .filter(|(_, spot)| spot.is_move())
            .map(|(_, spot)| (spot.at[0], spot.at[1]))
            .collect()
    };
    if let [part] = job.parts.as_slice() {
        let shifts = shifts_of(part);
        return contours.iter().map(|c| (c, shifts.clone())).collect();
    }
    let mut cut_by: Vec<Vec<Point>> = vec![Vec::new(); contours.len()];
    for part in &job.parts {
        let mut ids: Vec<usize> = Vec::new();
        for op in job
            .operations
            .iter()
            .filter(|op| op.part() == part.id && op.selects_shapes())
        {
            let picked = match op {
                Operation::Path(_) | Operation::Holes(_) => select_paths(
                    contours,
                    &op.select().resolved(&job.drawing),
                    &op.label_message(0),
                ),
                _ => select_contours(
                    contours,
                    &op.select().resolved(&job.drawing),
                    &op.label_message(0),
                ),
            };
            if let Ok(selection) = picked {
                ids.extend(selection.contours.iter().map(|c| c.id));
            }
        }
        for (k, contour) in contours.iter().enumerate() {
            if ids.contains(&contour.id) {
                for shift in shifts_of(part) {
                    if !cut_by[k].contains(&shift) {
                        cut_by[k].push(shift);
                    }
                }
            }
        }
    }
    contours
        .iter()
        .zip(cut_by)
        .map(|(c, shifts)| {
            if shifts.is_empty() {
                (c, vec![(0.0, 0.0)])
            } else {
                (c, shifts)
            }
        })
        .collect()
}

pub(super) fn path_data(d: &mut String, points: &[Point], closed: bool) {
    for (i, p) in points.iter().enumerate() {
        let _ = write!(
            d,
            "{}{} {} ",
            if i == 0 { "M" } else { "L" },
            svg_num(p.0),
            svg_num(-p.1)
        );
    }
    if closed && points.len() > 2 {
        d.push_str("Z ");
    }
}

pub(super) struct Bounds {
    pub(super) x0: f64,
    pub(super) y0: f64,
    pub(super) x1: f64,
    pub(super) y1: f64,
}

impl Bounds {
    pub(super) fn new((x, y): Point) -> Bounds {
        Bounds {
            x0: x,
            y0: y,
            x1: x,
            y1: y,
        }
    }

    pub(super) fn add(&mut self, (x, y): Point) {
        if x.is_finite() && y.is_finite() {
            self.x0 = self.x0.min(x);
            self.y0 = self.y0.min(y);
            self.x1 = self.x1.max(x);
            self.y1 = self.y1.max(y);
        }
    }
}
