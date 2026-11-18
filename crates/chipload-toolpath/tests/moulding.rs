// SPDX-License-Identifier: GPL-3.0-or-later
//! Moulding (docs/specs/moulding.md §11): the 1D drop of the tool onto the section, passes
//! as the path's offsets at that depth, tapered and round ends, closed paths, levels; what
//! it refuses.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use chipload_core::chain::Contour;
use chipload_core::cutter::Cutter;
use chipload_core::math;
use chipload_core::primitives::{Arc, Line, Primitive};
use chipload_job::migrations::migrate;
use chipload_job::{Job, MouldProfile, parse_job};
use chipload_toolpath::model::{Move, MoveKind};
use chipload_toolpath::moulding::Section;
use chipload_toolpath::plan::job_problems;
use chipload_toolpath::plan_job;
use proptest::prelude::*;
use serde_json::{Value, json};

const TOL: f64 = 0.02;

/// An open contour through `(x, y, bulge)` vertices.
fn open(id: usize, vertices: &[(f64, f64, f64)]) -> Contour {
    let parts = vertices
        .windows(2)
        .map(|w| {
            let ((x1, y1, b), (x2, y2, _)) = (w[0], w[1]);
            common::segment(match Arc::from_bulge((x1, y1), (x2, y2), b) {
                Some(arc) => Primitive::Arc(arc),
                None => Primitive::Line(Line {
                    start: (x1, y1),
                    end: (x2, y2),
                }),
            })
        })
        .collect();
    Contour {
        id,
        parts,
        closed: false,
        invalid_reason: None,
        nesting_depth: None,
    }
}

/// A project with ball nose 1 (6 mm), end mill 2 (6 mm, centre-cutting), V-bit 3, and one
/// moulding on layer L with `extra` over: a 6 mm cove left of the path, square ends, one
/// level, 1 mm stepover.
fn job(extra: Value) -> Job {
    let mut data = migrate(json!({
        "schema_version": 21,
        "drawing": {"shapes": [], "next_shape_id": 1, "dxfs": []},
        "stock": {"x": 300, "y": 200, "thickness": 18, "origin": "bottom_left",
                  "max_overcut": 0.5},
        "machine": {"post": "grbl", "safe_z": 5, "spindle": "cw", "tool_change": "files"},
        "tools": [
            {"id": 1, "type": "ballnose", "diameter": 6, "rpm": 18000},
            {"id": 2, "diameter": 6, "rpm": 18000, "center_cutting": true},
            {"id": 3, "type": "vbit", "diameter": 12.7, "rpm": 18000,
             "angle": 90f64.to_radians()}
        ],
        "operations": [],
    }))
    .unwrap();
    let mut op = json!({"type": "moulding", "part": 1, "select": {"layer": "L"}, "tool": 1,
        "profile": {"kind": "cove", "radius": 6.0}, "side": "left",
        "ends": {"kind": "square"}, "start_depth": 0, "stepover": 1.0, "step_down": null,
        "direction": "climb", "feed": 1500, "plunge_feed": 500});
    for (k, v) in extra.as_object().unwrap() {
        op[k] = v.clone();
    }
    data["operations"] = json!([op]);
    parse_job(data, "test.json").unwrap()
}

/// A 100 mm line along X at y = 20, drawn from x = 10.
fn line() -> Vec<Contour> {
    vec![open(1, &[(10.0, 20.0, 0.0), (110.0, 20.0, 0.0)])]
}

/// Every feed move's end below the top, as (x, y, z).
fn cuts(moves: &[Move]) -> Vec<(f64, f64, f64)> {
    let (mut x, mut y, mut z) = (0.0, 0.0, 0.0);
    let mut out = Vec::new();
    for m in moves {
        (x, y, z) = (m.x.unwrap_or(x), m.y.unwrap_or(y), m.z.unwrap_or(z));
        if m.kind != MoveKind::Rapid && z < -1e-6 {
            out.push((x, y, z));
        }
    }
    out
}

fn section(profile: MouldProfile) -> Section {
    Section::of(&profile, &job(json!({}))).unwrap()
}

#[test]
fn a_ball_in_a_cove_rides_round_it() {
    // A 6 mm cove (centre at the path's top edge), a 3 mm ball: the ball's centre stays
    // 3 mm inside the cove's circle, so its tip goes to −3 − √(9 − c²).
    let s = section(MouldProfile::Cove { radius: 6.0 });
    let ball = Cutter::Ball { radius: 3.0 };
    for k in 0..=30 {
        let c = k as f64 * 0.1;
        let want = -3.0 - (9.0 - c * c).sqrt();
        let got = s.drop(ball, c, 1.0);
        assert!((got - want).abs() < TOL, "at {c}: {got}, not {want}");
    }
}

#[test]
fn a_flat_end_mill_steps_down_to_the_profile_under_its_far_edge() {
    let s = section(MouldProfile::Cove { radius: 6.0 });
    let flat = Cutter::Flat { radius: 3.0 };
    for k in 0..=30 {
        let c = k as f64 * 0.1;
        let want = -(36.0 - (c + 3.0).powi(2)).max(0.0).sqrt();
        assert!((s.drop(flat, c, 1.0) - want).abs() < TOL, "at {c}");
    }
}

#[test]
fn passes_are_the_path_offset_at_the_drops_depth() {
    let plan = plan_job(&job(json!({})), &line()).unwrap();
    let s = section(MouldProfile::Cove { radius: 6.0 });
    let ball = Cutter::Ball { radius: 3.0 };
    let points = cuts(&plan.program.moves);
    assert!(!points.is_empty());
    for (x, y, z) in points {
        // Left of the line: y = 20 + c.
        let c = y - 20.0;
        assert!((-TOL..=6.0 + TOL).contains(&c), "off the side at y {y}");
        assert!(
            (10.0 - TOL..=110.0 + TOL).contains(&x),
            "past the ends at x {x}"
        );
        assert!((z - s.drop(ball, c, 1.0)).abs() < TOL, "at c {c}: {z}");
    }
}

#[test]
fn tapered_ends_fade_to_the_surface() {
    let plan = plan_job(
        &job(json!({"ends": {"kind": "taper", "length": 30.0, "shape": "straight"}})),
        &line(),
    )
    .unwrap();
    let points = cuts(&plan.program.moves);
    // On the path's own pass (c = 0): full depth only from 30 mm in, shallower before.
    let on_path: Vec<&(f64, f64, f64)> = points
        .iter()
        .filter(|p| (p.1 - 20.0).abs() < 1e-6)
        .collect();
    let deepest = on_path.iter().map(|p| p.2).fold(0.0, f64::min);
    assert!(deepest < -5.0, "{deepest}");
    let full_from = on_path
        .iter()
        .filter(|p| p.2 <= deepest + 0.05)
        .map(|p| p.0)
        .fold(f64::INFINITY, f64::min);
    assert!(
        (full_from - 40.0).abs() < 1.0,
        "full depth from x {full_from}"
    );
    assert!(
        points
            .iter()
            .all(|p| p.0 >= 10.0 - TOL && p.0 <= 110.0 + TOL)
    );
}

#[test]
fn round_ends_turn_a_quarter_round_the_end_points() {
    let plan = plan_job(&job(json!({"ends": {"kind": "round"}})), &line()).unwrap();
    let points = cuts(&plan.program.moves);
    // Beyond the line's ends, on the profile's side, a quarter circle about each end.
    let beyond: Vec<&(f64, f64, f64)> = points.iter().filter(|p| p.0 > 110.0 + 0.1).collect();
    assert!(!beyond.is_empty());
    for p in &beyond {
        assert!(p.1 >= 20.0 - TOL, "on the profile's side: {p:?}");
        let r = math::hypot(p.0 - 110.0, p.1 - 20.0);
        assert!(r <= 6.0 + TOL, "within the profile's width: {r}");
    }
    assert!(
        points.iter().any(|p| p.0 < 10.0 - 0.1),
        "round at the start too"
    );
}

#[test]
fn outside_a_rectangle_the_corners_turn_round() {
    let contours = vec![common::rect(1, 50.0, 50.0, 150.0, 110.0)];
    let plan = plan_job(&job(json!({"side": "outside"})), &contours).unwrap();
    let points = cuts(&plan.program.moves);
    // Nothing inside; past a corner, at the pass's distance from the corner itself.
    for p in &points {
        let inside = p.0 > 50.0 + TOL && p.0 < 150.0 - TOL && p.1 > 50.0 + TOL && p.1 < 110.0 - TOL;
        assert!(!inside, "inside the rectangle: {p:?}");
    }
    assert!(
        points.iter().any(|p| p.0 > 150.5 && p.1 > 110.5),
        "round the corner"
    );
}

#[test]
fn levels_never_take_more_than_a_step() {
    let plan = plan_job(&job(json!({"step_down": 2.0})), &line()).unwrap();
    // The deepest pass (about 5.9 mm) comes in three levels: no Z below −2 before −4 …
    let z: Vec<f64> = cuts(&plan.program.moves).iter().map(|p| p.2).collect();
    let first_below_2 = z.iter().position(|&v| v < -2.0 - TOL).unwrap();
    assert!(z[..first_below_2].iter().all(|&v| v >= -2.0 - TOL));
    let first_below_4 = z.iter().position(|&v| v < -4.0 - TOL).unwrap();
    assert!(first_below_4 > first_below_2);
}

#[test]
fn a_bead_leaves_its_top_at_the_surface() {
    let s = section(MouldProfile::Bead { radius: 4.0 });
    // The bead's crest is at the surface: the ball over it doesn't cut.
    assert!(s.drop(Cutter::Ball { radius: 3.0 }, 4.0, 1.0) > -TOL);
    assert!((s.deepest() - 4.0).abs() < TOL);
}

#[test]
fn what_it_cant_do_is_refused() {
    let problem = |job: &Job| {
        job_problems(job)
            .into_iter()
            .map(|p| p.path)
            .collect::<Vec<_>>()
    };
    assert!(problem(&job(json!({"tool": 3}))).contains(&"operations.0.tool".to_string()));
    let wide = plan_job(&job(json!({"stepover": 7.0})), &line())
        .unwrap_err()
        .to_string();
    assert!(wide.contains("stepover"), "{wide}");
    let missing = plan_job(
        &job(json!({"profile": {"kind": "shape", "shape": 99}})),
        &line(),
    )
    .unwrap_err()
    .to_string();
    assert!(missing.contains("S99"), "{missing}");
    let huge = plan_job(
        &job(json!({"profile": {"kind": "ogee", "width": 250.0, "depth": 5.0}})),
        &line(),
    )
    .unwrap_err()
    .to_string();
    assert!(huge.contains("at most"), "{huge}");
    let base = job(json!({}));
    let mut data = serde_json::to_value(&base).unwrap();
    data["operations"][0]["stepover"] = json!(0.0);
    assert!(parse_job(data, "test.json").is_err());
}

proptest! {
    #![proptest_config(common::proptest_config(48))]

    /// Random profiles and tools: the dropped tool touches the section and is nowhere
    /// below it (the 1D drop's promise), sampled across the tool.
    #[test]
    fn the_dropped_tool_is_never_below_the_section(
        kind in 0usize..5,
        size in 2.0f64..15.0,
        r in 1.0f64..6.0,
        ball in any::<bool>(),
        c in -2.0f64..20.0,
    ) {
        let profile = match kind {
            0 => MouldProfile::Cove { radius: size },
            1 => MouldProfile::Roundover { radius: size },
            2 => MouldProfile::Ogee { width: 2.0 * size, depth: size },
            3 => MouldProfile::Bead { radius: size / 2.0 },
            _ => MouldProfile::Flute { radius: size / 2.0 },
        };
        let s = section(profile);
        let cutter = if ball { Cutter::Ball { radius: r } } else { Cutter::Flat { radius: r } };
        let z = s.drop(cutter, c, 1.0);
        prop_assume!(z < 0.0);
        let mut touches = false;
        let step = chipload_toolpath::moulding::SECTION_STEP_MM;
        let first = ((c - r) / step).floor() as i64;
        let last = ((c + r) / step).ceil() as i64;
        // The top of the wall where the section ends: the board beyond it is at 0.
        let wall = s.width;
        let points = (first..=last)
            .map(|i| (i as f64 * step, None))
            .chain([(c - r, Some(-r)), (c + r, Some(r)), (wall, Some(wall - c))]);
        for (d, rim) in points {
            let Some(h) = cutter.height_at(rim.unwrap_or(d - c)) else { continue };
            // Grid points within the section are its samples exactly.
            let surface = if d < -1e-9 {
                if s.waste_before { continue } else { 0.0 }
            } else if d > s.width - 1e-12 && rim == Some(wall - c) || d > s.width + 1e-9 {
                0.0
            } else {
                // Straight between samples, as the section reads; its last sample lies at
                // its width, which may come before the grid's next step.
                let x = (d / step).clamp(0.0, (s.z.len() - 1) as f64);
                let i = (x.floor() as usize).min(s.z.len() - 2);
                let start = i as f64 * step;
                let end = ((i + 1) as f64 * step).min(s.width);
                let t = if end > start { ((d - start) / (end - start)).clamp(0.0, 1.0) } else { 0.0 };
                s.z[i] + (s.z[i + 1] - s.z[i]) * t
            };
            let tool = z + h;
            prop_assert!(tool >= surface - 1e-9, "below at d {d}: {tool} < {surface}");
            if (tool - surface).abs() < TOL {
                touches = true;
            }
        }
        prop_assert!(touches, "never touches");
    }
}

#[test]
fn the_preview_is_the_section_for_the_window() {
    use chipload_toolpath::moulding::preview;
    // A cove: from its full depth at the path up to the surface its radius across.
    let cove = preview(&MouldProfile::Cove { radius: 6.0 }, &[]).unwrap();
    assert!(
        cove.len() <= 200,
        "a few points for a small drawing: {}",
        cove.len()
    );
    let (first, last) = (cove[0], cove[cove.len() - 1]);
    assert!(
        (first.0 - 0.0).abs() < 1e-9 && (first.1 + 6.0).abs() < TOL,
        "{first:?}"
    );
    assert!(
        (last.0 - 6.0).abs() < 1e-9 && last.1.abs() < TOL,
        "{last:?}"
    );
    // A drawn profile from its shape among the drawing's contours; missing: none.
    let mut drawn = open(1, &[(0.0, 10.0, 0.0), (4.0, 6.0, 0.0), (8.0, 10.0, 0.0)]);
    for part in &mut drawn.parts {
        part.segment.shape = Some(7);
    }
    let v = preview(&MouldProfile::Shape { shape: 7 }, &[drawn]).unwrap();
    let deepest = v.iter().map(|p| p.1).fold(0.0, f64::min);
    // Sampled for a small drawing: its tip within a few samples.
    assert!((deepest + 4.0).abs() < 0.1, "{deepest}");
    assert!(preview(&MouldProfile::Shape { shape: 8 }, &[]).is_none());
}

#[test]
fn the_preview_shows_the_tool_at_each_pass() {
    use chipload_toolpath::moulding::preview_tool;
    let ball = Cutter::Ball { radius: 3.0 };
    let shown = preview_tool(&MouldProfile::Cove { radius: 6.0 }, &[], ball, 1.0).unwrap();
    // A tip every stepover up to the profile's far edge, where the ball doesn't cut.
    let s = section(MouldProfile::Cove { radius: 6.0 });
    assert!(shown.tips.len() >= 5, "{:?}", shown.tips);
    for &(c, z) in &shown.tips {
        assert!((z - s.drop(ball, c, 1.0)).abs() < 1e-9);
    }
    // The tool's outline at the deepest pass: its tip there, 6 mm across.
    let deepest = shown.tips.iter().map(|t| t.1).fold(0.0, f64::min);
    let low = shown.outline.iter().map(|p| p.1).fold(0.0, f64::min);
    assert!((low - deepest).abs() < 1e-9);
    let (lo, hi) = shown
        .outline
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), p| {
            (a.min(p.0), b.max(p.0))
        });
    assert!((hi - lo - 6.0).abs() < 1e-9);
    assert!(preview_tool(&MouldProfile::Cove { radius: 6.0 }, &[], ball, 0.0).is_none());
}

/// The two cases the property test found (TODO, 2026-11-16 and 11-17): a bead whose width
/// isn't a whole number of 0.01 mm steps, a ball nose reaching just past its end. The tool
/// must stay above the untouched board beyond the section, at its end wall too.
#[test]
fn the_tool_stays_above_the_board_just_past_the_sections_end() {
    for (size, r, c) in [
        (14.009968037188523, 2.709730661055996, 12.554779909949778),
        (11.659449711444985, 1.0, 10.956349851748659),
    ] {
        let s = section(MouldProfile::Bead { radius: size / 2.0 });
        let cutter = Cutter::Ball { radius: r };
        let z = s.drop(cutter, c, 1.0);
        // Every point of the tool past the section's end is above the board (Z 0).
        let mut d = s.width;
        while d <= c + r {
            if let Some(h) = cutter.height_at(d - c) {
                assert!(z + h >= -1e-9, "below the board at d {d}: {}", z + h);
            }
            d += 0.0005;
        }
    }
}
