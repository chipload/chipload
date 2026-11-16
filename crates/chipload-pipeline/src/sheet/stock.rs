// SPDX-License-Identifier: GPL-3.0-or-later
//! The stock and its zero, and a board turned over (docs/specs/two-sided.md).

use std::fmt::Write as _;

use chipload_core::tolerances::CHAIN_TOL_MM;
use chipload_i18n::Message;
use chipload_job::{Job, Register, StockCorner, Turn, WorkZero, ZZero};

use super::{Words, fact, len, len_u};
use crate::copies;

pub(super) fn setup(html: &mut String, job: &Job, w: Words) {
    let stock = &job.stock();
    let _ = writeln!(
        html,
        "<section class=\"setup\">\n<h2>{}</h2>\n<table class=\"facts\">",
        w.text(&Message::new("sheet-stock-and-zero"))
    );
    fact(
        html,
        &w.text(&Message::new("sheet-stock")),
        &w.html(
            &Message::new("sheet-stock-size")
                .arg("width", len(job.stock_size().0))
                .arg("length", len_u(job.stock_size().1))
                .arg("thickness", len_u(stock.thickness)),
        ),
    );
    fact(html, "X0 Y0", &w.text(&xy_zero_message(job)));
    fact(html, "Z0", &w.text(&z_zero_message(job)));
    // A front: the board stays for its back (docs/specs/two-sided.md §8).
    if let Some(back) = job.active_setup_id().and_then(|id| job.back_of(id)) {
        fact(
            html,
            &w.text(&Message::new("sheet-back-side")),
            &w.text(&Message::new("sheet-back-side-value").arg("back", back.name.as_str())),
        );
    }
    if let Some(copies) = copies::copies_message(job, w.units) {
        fact(
            html,
            &w.text(&Message::new("sheet-copies")),
            &w.text(&copies),
        );
    }
    // Several parts: each with its copies (docs/specs/parts.md §6).
    for line in copies::parts_messages(job, w.units) {
        fact(html, &w.text(&Message::new("sheet-part")), &w.text(&line));
    }
    // Mirrored parts: cut as the drawing's mirror image (docs/specs/mirror-part.md §5).
    for part in job
        .parts
        .iter()
        .filter(|p| p.enabled && Some(p.setup) == job.active_setup_id())
    {
        let id = match part.mirror {
            Some(chipload_job::Mirror::LeftRight) => "sheet-mirrored-left-right",
            Some(chipload_job::Mirror::TopBottom) => "sheet-mirrored-top-bottom",
            None => continue,
        };
        fact(
            html,
            &w.text(&Message::new("sheet-mirrored")),
            &w.text(&Message::new(id).arg("part", part.name.as_str())),
        );
    }
    // An inlay's plug: what to do with it once cut (docs/specs/vcarve-inlay.md §7).
    for op in job.operations.iter().filter(|op| {
        op.enabled()
            && job
                .part_of(op)
                .is_some_and(|p| p.enabled && Some(p.setup) == job.active_setup_id())
    }) {
        let chipload_job::Operation::Vcarve(plug) = op else {
            continue;
        };
        if plug.plug.is_none() {
            continue;
        }
        let steps = match crate::inlay::pocket_setup(job, plug) {
            Some(setup) => Message::new("sheet-inlay-steps")
                .arg("setup", setup.id)
                .arg("name", setup.name.as_str()),
            None => Message::new("sheet-inlay-steps-no-pocket"),
        };
        fact(html, &w.text(&Message::new("sheet-inlay")), &w.text(&steps));
    }
    // Several drawings: each with where it was put (docs/specs/drawing-sources.md §7).
    let d = &job.drawing;
    if d.dxfs.len() + d.images.len() + d.texts.len() > 1 {
        for dxf in &job.drawing.dxfs {
            fact(
                html,
                &w.text(&Message::new("sheet-dxf").arg("id", dxf.id)),
                &w.text(
                    &Message::new("sheet-dxf-placed")
                        .arg("name", dxf.name.as_str())
                        .arg("file", dxf.file.as_str())
                        .arg(
                            "placement",
                            crate::placement::placement_message(dxf.placement),
                        ),
                ),
            );
        }
        for image in &job.drawing.images {
            fact(
                html,
                &w.text(&Message::new("sheet-image").arg("id", image.id)),
                &w.text(
                    &Message::new("sheet-image-placed")
                        .arg("file", image.file.as_str())
                        .arg("layer", image.layer.as_str())
                        .arg("width", len_u(image.width))
                        .arg("x", len(image.x))
                        .arg("y", len(image.y)),
                ),
            );
        }
        for text in &job.drawing.texts {
            fact(
                html,
                &w.text(&Message::new("sheet-text").arg("id", text.id)),
                &w.text(
                    &Message::new("sheet-text-placed")
                        .arg("text", crate::texts::text_label(text))
                        .arg("font", text.font.as_str())
                        .arg("layer", text.layer.as_str())
                        .arg("height", len_u(text.height))
                        .arg("x", len(text.x))
                        .arg("y", len(text.y)),
                ),
            );
        }
    }
    html.push_str("</table>\n</section>\n");
}

/// A back setup's first words: how to turn the board and put it back in place
/// (docs/specs/two-sided.md §8). Nothing for any other setup.
pub(super) fn turning(html: &mut String, job: &Job, w: Words) {
    let Some(id) = job.active_setup_id() else {
        return;
    };
    let (Some(back), Some(front)) = (job.setup(id).and_then(|s| s.back_of), job.front_of(id))
    else {
        return;
    };
    let _ = writeln!(
        html,
        "<section class=\"turn\">\n<h2>{}</h2>",
        w.text(&Message::new("sheet-turn-heading"))
    );
    let turn = match back.turn {
        Turn::LeftRight => "sheet-turn-left-right",
        Turn::TopBottom => "sheet-turn-top-bottom",
    };
    let mut lines = vec![Message::new(turn)];
    match back.register {
        Register::Pins => match dowel_diameter(job, front.id) {
            Some(diameter) => {
                lines.push(Message::new("sheet-turn-pins").arg("diameter", len_u(diameter)));
                // A surface pass over them: they go in below the back face (two-sided.md §6).
                if let Some((_, depth)) = job.surface_over_pins() {
                    lines.push(Message::new("sheet-turn-pins-down").arg("depth", len_u(depth)));
                }
            }
            None => {
                lines.push(Message::new("sheet-turn-no-dowels").arg("front", front.name.as_str()))
            }
        },
        Register::Corner => lines.push(
            Message::new("sheet-turn-corner")
                .arg("front", front.name.as_str())
                .arg("width", len(job.stock_size().0))
                .arg("length", len_u(job.stock_size().1)),
        ),
    }
    lines.push(Message::new("sheet-turn-picture"));
    for line in lines {
        let _ = writeln!(html, "<p class=\"do\">{}</p>", w.text(&line));
    }
    html.push_str("</section>\n");
}

/// The diameter of the dowel holes setup `front` cuts, if it has any.
fn dowel_diameter(job: &Job, front: u64) -> Option<f64> {
    job.operations
        .iter()
        .filter(|op| op.is_dowels() && job.setup_of(op).is_some_and(|s| s.id == front))
        .find_map(|op| {
            let select = op.select().resolved(&job.drawing);
            job.drawing
                .shapes
                .iter()
                .find_map(|shape| match shape.geometry {
                    chipload_job::Geometry::Circle { radius, .. } if select.may_pick(shape) => {
                        Some(2.0 * radius)
                    }
                    _ => None,
                })
        })
}

/// Where X0 Y0 is, in words measured on the stock (§2.3), in English and mm.
pub fn xy_zero_text(job: &Job) -> String {
    xy_zero_message(job).english()
}

/// [`xy_zero_text`] as a message.
pub fn xy_zero_message(job: &Job) -> Message {
    let corner = |corner: StockCorner| {
        Message::new(match corner {
            StockCorner::Center => "sheet-zero-centre",
            StockCorner::BottomLeft => "sheet-zero-bottom-left",
            StockCorner::BottomRight => "sheet-zero-bottom-right",
            StockCorner::TopLeft => "sheet-zero-top-left",
            StockCorner::TopRight => "sheet-zero-top-right",
        })
    };
    if let WorkZero::Stock { corner: at } = job.work_zero() {
        return corner(at);
    }
    let ((x0, y0), (x1, y1)) = job.stock_bounds();
    let (zx, zy) = job.work_zero_xy();
    let (dx, dy) = (zx - x0, zy - y0);
    let mut parts = Vec::new();
    if dx.abs() > CHAIN_TOL_MM {
        let side = if dx > 0.0 {
            "sheet-zero-right-of"
        } else {
            "sheet-zero-left-of"
        };
        parts.push(Message::new(side).arg("distance", len_u(dx.abs())));
    }
    if dy.abs() > CHAIN_TOL_MM {
        let side = if dy > 0.0 {
            "sheet-zero-above"
        } else {
            "sheet-zero-below"
        };
        parts.push(Message::new(side).arg("distance", len_u(dy.abs())));
    }
    let offset = parts.into_iter().reduce(|first, second| {
        Message::new("sheet-zero-and")
            .arg("first", first)
            .arg("second", second)
    });
    let text = match offset {
        None => corner(StockCorner::BottomLeft),
        Some(offset) => Message::new("sheet-zero-from-corner").arg("offset", offset),
    };
    let outside = zx < x0 - CHAIN_TOL_MM
        || zx > x1 + CHAIN_TOL_MM
        || zy < y0 - CHAIN_TOL_MM
        || zy > y1 + CHAIN_TOL_MM;
    if outside {
        Message::new("sheet-zero-outside").arg("zero", text)
    } else {
        text
    }
}

pub(super) fn z_zero_message(job: &Job) -> Message {
    Message::new(match job.stock().z_zero {
        ZZero::Top => "sheet-z-top",
        ZZero::Bottom => "sheet-z-bottom",
    })
}
