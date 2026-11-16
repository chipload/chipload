// SPDX-License-Identifier: GPL-3.0-or-later
//! The pieces sheet: a puzzle of boards, each piece with its file (docs/specs/pieces.md).

use std::fmt::Write as _;
use std::path::Path;

use chipload_i18n::{Message, Value};
use chipload_job::Job;

use super::files::{file_name, file_section, watch_out};
use super::picture::{Bounds, path_data, picture};
use super::stock::setup;
use super::{SheetInfo, Words, escape, head, header, svg_num};
use crate::RunResult;
use crate::estimate::{Lengths, estimate_lines};

/// One piece's run, for the sheet of a setup in pieces: its name, the project as it is
/// cut (its board and zero) and its result.
pub struct PieceSheet<'a> {
    pub name: &'a str,
    pub job: &'a Job,
    pub result: &'a RunResult,
}

/// The sheet of a setup cut in pieces (docs/specs/pieces.md §8): a map of the pieces as they
/// go together, then for each piece its board, zero, picture and programs, then gluing.
pub fn pieces_sheet(
    job: &Job,
    map: &crate::pieces::PieceMap,
    pieces: &[PieceSheet],
    output: &Path,
    info: &SheetInfo,
) -> String {
    let w = Words {
        lang: info.language,
        units: info.units,
    };
    let mut html = String::new();
    head(&mut html, job, info, w);
    if let Some(first) = pieces.first() {
        header(&mut html, job, first.result, info, w);
    }
    pieces_map(&mut html, map, w);
    for piece in pieces {
        let _ = writeln!(
            html,
            "<section class=\"piece\">\n<h2>{}</h2>",
            w.text(&Message::new("sheet-piece").arg("name", piece.name))
        );
        let _ = writeln!(
            html,
            "<p class=\"do\">{}</p>",
            w.text(&Message::new("sheet-piece-mark").arg("name", piece.name))
        );
        setup(&mut html, piece.job, w);
        picture(&mut html, piece.job, piece.result, w);
        let names: Vec<String> = piece
            .result
            .files
            .iter()
            .map(|f| file_name(f, output))
            .collect();
        let several = piece.result.files.len() > 1;
        for (i, name) in names.iter().enumerate() {
            file_section(&mut html, piece.job, piece.result, i, name, several, w);
        }
        watch_out(&mut html, piece.result, w);
        let _ = writeln!(html, "<h3>{}</h3>", w.text(&Message::new("sheet-time")));
        for line in estimate_lines(piece.job, &piece.result.estimate, several, Lengths::Without) {
            let text = line.message.render(w.lang, w.units.into());
            let _ = writeln!(html, "<p>{}</p>", escape(text.trim_start()));
        }
        html.push_str("</section>\n");
    }
    let _ = writeln!(
        html,
        "<section class=\"total\">\n<h2>{}</h2>\n<p>{}</p>\n</section>",
        w.text(&Message::new("sheet-pieces-glue-title")),
        w.text(&Message::new("sheet-pieces-glue").arg(
            "clearance",
            Value::Length(map.clearance, chipload_i18n::Digits::Shortest)
        ))
    );
    html.push_str("</body>\n</html>\n");
    html
}

/// The pieces as they go together, each named in its middle.
fn pieces_map(html: &mut String, map: &crate::pieces::PieceMap, w: Words) {
    let joint = match map.joint {
        chipload_job::Joint::Knobs => "knobs",
        chipload_job::Joint::Fingers => "fingers",
    };
    let _ = writeln!(
        html,
        "<section class=\"setup\">\n<h2>{}</h2>\n<p>{}</p>",
        w.text(&Message::new("sheet-pieces")),
        w.text(
            &Message::new("sheet-pieces-summary")
                .arg("count", map.pieces.len())
                .arg("columns", map.columns)
                .arg("rows", map.rows)
                .arg("joint", joint)
                .arg(
                    "size",
                    Value::Length(map.size, chipload_i18n::Digits::Shortest)
                )
        )
    );
    let mut bounds: Option<Bounds> = None;
    for (_, rings) in &map.pieces {
        for p in rings.iter().flatten() {
            match &mut bounds {
                Some(b) => b.add(*p),
                None => bounds = Some(Bounds::new(*p)),
            }
        }
    }
    let Some(bounds) = bounds else {
        html.push_str("</section>\n");
        return;
    };
    let size = (bounds.x1 - bounds.x0).max(bounds.y1 - bounds.y0).max(1.0);
    let margin = size * 0.04;
    let _ = writeln!(
        html,
        "<figure>\n<svg class=\"picture\" xmlns=\"http://www.w3.org/2000/svg\" \
         viewBox=\"{} {} {} {}\" preserveAspectRatio=\"xMidYMid meet\">",
        svg_num(bounds.x0 - margin),
        svg_num(-(bounds.y1 + margin)),
        svg_num(bounds.x1 - bounds.x0 + 2.0 * margin),
        svg_num(bounds.y1 - bounds.y0 + 2.0 * margin)
    );
    for (name, rings) in &map.pieces {
        let mut d = String::new();
        let mut centre = Bounds::new(
            rings
                .first()
                .and_then(|r| r.first())
                .copied()
                .unwrap_or((0.0, 0.0)),
        );
        for ring in rings {
            path_data(&mut d, ring, true);
            for p in ring {
                centre.add(*p);
            }
        }
        let _ = writeln!(
            html,
            "<path d=\"{}\" fill=\"#f4efe6\" stroke=\"#555\" stroke-width=\"1\" \
             vector-effect=\"non-scaling-stroke\"/>",
            d.trim_end()
        );
        let _ = writeln!(
            html,
            "<text x=\"{}\" y=\"{}\" font-size=\"{}\" text-anchor=\"middle\" \
             dominant-baseline=\"middle\" fill=\"#111\">{}</text>",
            svg_num((centre.x0 + centre.x1) / 2.0),
            svg_num(-(centre.y0 + centre.y1) / 2.0),
            svg_num(size * 0.05),
            escape(name)
        );
    }
    html.push_str("</svg>\n</figure>\n</section>\n");
}
