// SPDX-License-Identifier: GPL-3.0-or-later
//! Helpers shared by the tests that a direction survives a mirror: two-sided work
//! (docs/specs/two-sided.md §4) and mirrored parts (docs/specs/mirror-part.md §3).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, dead_code)]

use chipload_core::math;
use std::path::Path;

use chipload_job::{Job, Turn};
use serde_json::json;

pub fn corpus_value(rel: &str) -> serde_json::Value {
    let text = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/corpus")
            .join(rel),
    )
    .unwrap();
    chipload_job::migrations::migrate(serde_json::from_str(&text).unwrap()).unwrap()
}

/// A pocket, an inside and an outside profile (op_add), plus a helical bore of the round
/// hole and a follow path round the slot, climb and conventional both: every operation
/// with a direction. Its shapes mirrored about `axis` (x: the line X = at) when given.
pub fn every_direction(mirror: Option<(Turn, f64)>) -> Job {
    use chipload_pipeline::new_ops::{NewRequest, OpKind, insert, new_operations};
    let mut data = corpus_value("jobs/v35/op_add.chipload");
    if let Some((turn, at)) = mirror {
        let flip = |p: &mut serde_json::Value| {
            let k = match turn {
                Turn::LeftRight => 0,
                Turn::TopBottom => 1,
            };
            p[k] = (2.0 * at - p[k].as_f64().unwrap()).into();
        };
        for shape in data["drawing"]["shapes"].as_array_mut().unwrap() {
            for segment in shape["segments"].as_array_mut().unwrap() {
                if let Some(line) = segment.get_mut("line") {
                    flip(&mut line["start"]);
                    flip(&mut line["end"]);
                } else {
                    let arc = &mut segment["arc"];
                    flip(&mut arc["center"]);
                    let (a, s) = (
                        arc["start_angle"].as_f64().unwrap(),
                        arc["sweep"].as_f64().unwrap(),
                    );
                    // x -> -x turns an angle t into pi - t, y -> -y into -t; the arc keeps
                    // its place in the path, so it runs the other way round.
                    arc["start_angle"] = match turn {
                        Turn::LeftRight => std::f64::consts::PI - a,
                        Turn::TopBottom => -a,
                    }
                    .into();
                    arc["sweep"] = (-s).into();
                }
            }
        }
    }
    // Conventional milling on the pocket, climb on the profiles.
    data["operations"][0]["direction"] = "conventional".into();
    let mut job = chipload_job::parse_job(data, "test").unwrap();
    let contours = chipload_pipeline::run_job(&job).unwrap().contours;
    for (kind, shape) in [(OpKind::Holes, 2), (OpKind::Path, 3)] {
        let request = NewRequest {
            kind,
            select: chipload_job::Selector {
                shapes: Some(vec![shape]),
                ..Default::default()
            },
            tool: None,
        };
        let made = new_operations(&job, &request, &contours, None, chipload_i18n::Language::En);
        insert(&mut job, made);
    }
    job.follow_thickness();
    assert!(job.validate().is_empty(), "{:?}", job.validate());
    job
}

/// Each run of cutting moves between rapids, by operation, as the signed area of the loop
/// it makes when closed by a straight line back to its start (mm², counter-clockwise
/// positive; arcs counted along their curve).
pub fn loops(
    program: &chipload_toolpath::model::Program,
    spans: &[chipload_toolpath::OpSpan],
) -> Vec<(usize, Vec<f64>)> {
    use chipload_toolpath::model::MoveKind;
    let mut out = Vec::new();
    for span in spans {
        let mut runs = Vec::new();
        let mut area = 0.0;
        let mut at: Option<(f64, f64)> = None;
        // Where the run being measured began.
        let mut from: Option<(f64, f64)> = None;
        let close = |area: f64, from: Option<(f64, f64)>, to: Option<(f64, f64)>| match (from, to) {
            (Some(a), Some(b)) => area + (b.0 * a.1 - a.0 * b.1) / 2.0,
            _ => area,
        };
        // Where the tool is before the span: the last XY set before it.
        for m in &program.moves[..span.moves.start] {
            if let (Some(x), Some(y)) = (m.x, m.y) {
                at = Some((x, y));
            }
        }
        for m in &program.moves[span.moves.clone()] {
            let to = match (at, m.x, m.y) {
                (Some(p), x, y) => (x.unwrap_or(p.0), y.unwrap_or(p.1)),
                (None, Some(x), Some(y)) => (x, y),
                _ => continue,
            };
            match (m.kind, at) {
                (MoveKind::Rapid, _) => {
                    let closed = close(area, from, at);
                    if closed != 0.0 {
                        runs.push(closed);
                    }
                    area = 0.0;
                    from = None;
                }
                (kind, Some(p)) => {
                    from = from.or(Some(p));
                    area += (p.0 * to.1 - to.0 * p.1) / 2.0;
                    if let (MoveKind::ArcCw | MoveKind::ArcCcw, Some(c)) = (kind, m.circle) {
                        let angle = |q: (f64, f64)| math::atan2(q.1 - c.center.1, q.0 - c.center.0);
                        let tau = std::f64::consts::TAU;
                        let mut sweep = (angle(to) - angle(p)).rem_euclid(tau);
                        if sweep == 0.0 {
                            sweep = tau;
                        }
                        if kind == MoveKind::ArcCw {
                            sweep -= tau;
                        }
                        area += c.radius * c.radius / 2.0 * (sweep - math::sin(sweep));
                    }
                }
                _ => {}
            }
            at = Some(to);
        }
        let closed = close(area, from, at);
        if closed != 0.0 {
            runs.push(closed);
        }
        out.push((span.operation, runs));
    }
    out
}

/// The area cut each way by each operation: (counter-clockwise, clockwise) totals.
pub fn turning(loops: &[(usize, Vec<f64>)]) -> Vec<(usize, f64, f64)> {
    let mut ops: Vec<usize> = loops.iter().map(|(op, _)| *op).collect();
    ops.dedup();
    ops.into_iter()
        .map(|op| {
            let runs: Vec<f64> = loops
                .iter()
                .filter(|(o, _)| *o == op)
                .flat_map(|(_, r)| r.clone())
                .collect();
            let ccw: f64 = runs.iter().filter(|a| **a > 0.0).sum();
            let cw: f64 = runs.iter().filter(|a| **a < 0.0).sum();
            (op, ccw, cw)
        })
        .collect()
}

/// A 40 × 20 hole cut with dogbones in part 1, a 10 mm square far up and right in part 1
/// too (so the part's box is 0 … 110 wide), and a circle 0.6 mm across at `circle_at`, cut
/// by part 2: at (-0.6, 1.5) it is beside the hole's bottom-left corner.
pub fn bones_beside_another_part(circle_at: [f64; 2]) -> Job {
    let line = |a: [f64; 2], b: [f64; 2]| json!({"line": {"start": a, "end": b}});
    let rect = |id: u64, layer: &str, x0: f64, y0: f64, x1: f64, y1: f64| {
        json!({"id": id, "layer": layer, "kind": "path", "closed": true, "segments": [
            line([x0, y0], [x1, y0]), line([x1, y0], [x1, y1]),
            line([x1, y1], [x0, y1]), line([x0, y1], [x0, y0])]})
    };
    let circle = json!({"id": 3, "layer": "B", "kind": "path", "closed": true, "segments": [
        {"arc": {"center": circle_at, "radius": 0.3, "start_angle": 0.0,
                 "sweep": std::f64::consts::TAU}}]});
    let mut data = corpus_value("jobs/v35/op_add.chipload");
    data["drawing"]["shapes"] = json!([
        rect(1, "A", 0.0, 0.0, 40.0, 20.0),
        rect(2, "FAR", 100.0, 100.0, 110.0, 110.0),
        circle
    ]);
    data["drawing"]["next_shape_id"] = json!(4);
    data["drawing"]["dxfs"] = json!([]);
    data["parts"] = json!([
        {"id": 1, "name": "Box", "enabled": true, "instances": [], "order": "by_operation", "setup": 1,
         "angle": 0.0, "at": [0.0, 0.0]},
        {"id": 2, "name": "Dot", "enabled": true, "instances": [], "order": "by_operation", "setup": 1,
         "angle": 0.0, "at": [0.0, 0.0]}]);
    let template = data["operations"][1].clone();
    let profile = |part: u64, layer: &str, side: &str, corners: serde_json::Value| {
        let mut op = template.clone();
        op["part"] = json!(part);
        op["name"] = json!(layer);
        op["select"] = json!({"layer": layer});
        op["side"] = json!(side);
        op["corners"] = corners;
        op["end_depth"] = json!(4.0);
        op["through"] = json!(null);
        op
    };
    data["operations"] = json!([
        profile(
            1,
            "A",
            "inside",
            json!({"type": "dogbone", "max_angle": 1.75})
        ),
        profile(1, "FAR", "outside", json!(null)),
        profile(2, "B", "on", json!(null)),
    ]);
    let job = chipload_job::parse_job(data, "test").unwrap();
    assert!(job.validate().is_empty(), "{:?}", job.validate());
    job
}
