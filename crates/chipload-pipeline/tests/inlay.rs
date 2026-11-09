// SPDX-License-Identifier: GPL-3.0-or-later
//! V-carve inlays (docs/specs/vcarve-inlay.md): the plug, turned over and pushed in until
//! its outline level meets the base's surface, fits the pocket (§2.3, §9).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use chipload_job::{Job, Operation, Selector, Source};
use chipload_pipeline::inlay::{InlayMade, InlayRequest, add_inlay};
use chipload_pipeline::simulate::{Simulation, simulate};
use chipload_pipeline::{Cancel, load_drawing, run_setups};
use common::corpus_value;
use proptest::prelude::*;

const POCKET: f64 = 5.0;
const PLUG_START: f64 = 3.0;
const GAP: f64 = 1.5;

/// The text project, its operations gone, its V-bit's tip `tip` (a flat tip falls short at
/// sharp corners, §2.4), and an inlay of text `text` made with `floor` as the floor end
/// mill (1: the project's 6 mm one; another id: a 2 mm one). Text 2 reads "AO" in a serif
/// font 50 mm high: serifs, a counter, hairlines narrower than the pocket is deep and
/// stems wider, with floors.
fn inlay_of(text: u64, floor: i64, tip: f64) -> (Job, InlayMade) {
    let mut job = chipload_job::parse_job(corpus_value("jobs/v27/text.chipload"), "test").unwrap();
    job.operations.clear();
    job.drawing.texts[1].text = "AO".into();
    job.tools[1].tip_diameter = Some(tip);
    if floor != 1 {
        let mut mill = job.tools[0].clone();
        mill.id = floor;
        mill.diameter = 2.0;
        job.tools.push(mill);
    }
    let drawing = load_drawing(&job).unwrap();
    let request = InlayRequest {
        select: Selector {
            source: Some(Source::Text(text)),
            ..Selector::default()
        },
        tool: 2,
        depth: POCKET,
        plug_start: PLUG_START,
        gap: GAP,
        floor_tool: Some(floor),
        plug_thickness: 10.0,
    };
    add_inlay(
        &job,
        &request,
        &drawing.contours,
        None,
        chipload_i18n::Language::En,
    )
    .unwrap()
}

/// Each setup's material after its program, by setup id.
fn simulated(job: &Job, cell: f64) -> Vec<(u64, Simulation)> {
    run_setups(job, None)
        .unwrap()
        .into_iter()
        .map(|(id, result)| {
            let one = job.for_setup(id);
            let sim = simulate(
                &one,
                &result.program,
                &result.groups,
                Some(cell),
                &Cancel::new(),
            )
            .unwrap();
            (id, sim)
        })
        .collect()
}

/// How deep a simulated board is cut, cell by cell, with the needles taken out: material
/// a disk of `NEEDLE_CELLS` cells' radius doesn't fit in. A pocket's own uncut check
/// leaves such needles between its rings on purpose (REST_TOL_MM, 0.05 mm, a radius), and
/// they snap off. A closing of the depth map (the deepest within the disk, then the
/// shallowest of that within it) takes them out and leaves every straight wall where it is.
struct Depths {
    origin: (f64, f64),
    cell: f64,
    columns: usize,
    rows: usize,
    depth: Vec<f64>,
}

/// The needles' radius in cells: two cells of 0.05 mm, twice the pocket's tolerance.
const NEEDLE_CELLS: i64 = 2;

impl Depths {
    fn of(sim: &Simulation) -> Depths {
        let (columns, rows) = (sim.columns, sim.rows);
        let raw: Vec<f64> = (0..rows)
            .flat_map(|j| (0..columns).map(move |i| (i, j)))
            .map(|(i, j)| sim.top - sim.height(i, j))
            .collect();
        let over = |values: &[f64], pick: fn(f64, f64) -> f64| -> Vec<f64> {
            let mut out = values.to_vec();
            for j in 0..rows as i64 {
                for i in 0..columns as i64 {
                    let mut v = values[j as usize * columns + i as usize];
                    for dj in -NEEDLE_CELLS..=NEEDLE_CELLS {
                        for di in -NEEDLE_CELLS..=NEEDLE_CELLS {
                            let (a, b) = (i + di, j + dj);
                            if di * di + dj * dj > NEEDLE_CELLS * NEEDLE_CELLS
                                || a < 0
                                || b < 0
                                || a >= columns as i64
                                || b >= rows as i64
                            {
                                continue;
                            }
                            v = pick(v, values[b as usize * columns + a as usize]);
                        }
                    }
                    out[j as usize * columns + i as usize] = v;
                }
            }
            out
        };
        let deepest = over(&raw, f64::max);
        Depths {
            origin: sim.origin,
            cell: sim.cell,
            columns,
            rows,
            depth: over(&deepest, f64::min),
        }
    }

    fn at_cell(&self, i: usize, j: usize) -> f64 {
        self.depth[j * self.columns + i]
    }

    /// The depth at `p` (drawing coordinates), from the cell centres round it, bilinearly.
    /// None outside the simulated cells.
    fn at(&self, p: (f64, f64)) -> Option<f64> {
        let fx = (p.0 - self.origin.0) / self.cell - 0.5;
        let fy = (p.1 - self.origin.1) / self.cell - 0.5;
        if fx < 0.0 || fy < 0.0 {
            return None;
        }
        let (i, j) = (fx.floor() as usize, fy.floor() as usize);
        if i + 1 >= self.columns || j + 1 >= self.rows {
            return None;
        }
        let (tx, ty) = (fx - i as f64, fy - j as f64);
        let h = |a: usize, b: usize| self.at_cell(a, b);
        let low = h(i, j) * (1.0 - tx) + h(i + 1, j) * tx;
        let high = h(i, j + 1) * (1.0 - tx) + h(i + 1, j + 1) * tx;
        Some(low * (1.0 - ty) + high * ty)
    }
}

/// The worst misfits of the plug turned over in the pocket: the most the plug overlaps the
/// base, and the most it stands off a pocket wall it should touch, mm; and how many points
/// were compared.
struct Fit {
    overlap: f64,
    off_wall: f64,
    points: usize,
    deepest_plug: f64,
    shallowest_outside: f64,
}

/// The plug turned over (mirrored back about its part's centre line) and pushed in until
/// its outline level, `outline` down, meets the base's surface: at every plug cell
/// within the artwork's box grown by `margin`, its lowest material `e = D - plug depth`
/// against the pocket's depth `p` there. Overlap: e above min(p, D). Off a wall: inside
/// the pocket (p > 0), e short of min(p, D).
fn fit(job: &Job, made: &InlayMade, cell: f64, margin: f64, outline: f64) -> Fit {
    let sims = simulated(job, cell);
    let base = Depths::of(&sims.iter().find(|(id, _)| *id != made.setup).unwrap().1);
    let plug = Depths::of(&sims.iter().find(|(id, _)| *id == made.setup).unwrap().1);
    let ((x0, y0), (x1, y1)) = job.part_box(made.part).unwrap();
    let centre = (x0 + x1) / 2.0;
    let mut fit = Fit {
        overlap: f64::NEG_INFINITY,
        off_wall: f64::NEG_INFINITY,
        points: 0,
        deepest_plug: 0.0,
        shallowest_outside: f64::INFINITY,
    };
    for j in 0..plug.rows {
        for i in 0..plug.columns {
            let q = (
                plug.origin.0 + (i as f64 + 0.5) * plug.cell,
                plug.origin.1 + (j as f64 + 0.5) * plug.cell,
            );
            let inside = |v: f64, lo: f64, hi: f64| lo - margin <= v && v <= hi + margin;
            if !(inside(q.0, x0, x1) && inside(q.1, y0, y1)) {
                continue;
            }
            let p = (2.0 * centre - q.0, q.1);
            let Some(pocket) = base.at(p) else {
                continue;
            };
            let plug_depth = plug.at_cell(i, j);
            fit.deepest_plug = fit.deepest_plug.max(plug_depth);
            let e = outline - plug_depth;
            let meets = pocket.min(outline);
            fit.overlap = fit.overlap.max(e - meets);
            if pocket > cell {
                fit.off_wall = fit.off_wall.max(meets - e);
            } else {
                fit.shallowest_outside = fit.shallowest_outside.min(plug_depth);
            }
            fit.points += 1;
        }
    }
    fit
}

/// How far apart plug and pocket may be found on a wall of a bit of `angle`°: the V-carve
/// tolerance and a cell's worth of position as height, the simulation's cell (a 60° wall
/// rises 1.73 mm per mm). Off a wall, also the needles' disk: taking them out lowers a
/// sharp ridge (the top of a hairline's plug) that much.
fn tolerance(angle: f64, cell: f64) -> (f64, f64) {
    let slope = 1.0 / (angle.to_radians() / 2.0).tan();
    let overlap = chipload_core::tolerances::VCARVE_TOL_MM + slope * cell;
    (overlap, overlap + slope * NEEDLE_CELLS as f64 * cell)
}

#[test]
fn the_pair_is_a_pocket_and_a_mirrored_plug_on_a_board_of_its_own() {
    let (job, made) = inlay_of(2, 1, 0.0);
    let Operation::Vcarve(pocket) = &job.operations[made.pocket] else {
        panic!()
    };
    let Operation::Vcarve(plug) = &job.operations[made.plug] else {
        panic!()
    };
    assert_eq!(pocket.end_depth, POCKET);
    assert_eq!(pocket.ignore_kinks_deg, 0.0);
    assert!(pocket.plug.is_none() && pocket.floor.is_some());
    assert_eq!(plug.plug.map(|p| p.start_depth), Some(PLUG_START));
    assert_eq!(plug.end_depth, PLUG_START + GAP);
    assert_eq!(plug.select, pocket.select);
    let part = job.part(made.part).unwrap();
    assert_eq!(part.mirror, Some(chipload_job::Mirror::LeftRight));
    assert_eq!(part.setup, made.setup);
    let setup = job.setups.iter().find(|s| s.id == made.setup).unwrap();
    assert_eq!(setup.stock.thickness, 10.0);
    assert_eq!(setup.stock.fit, Some(10.0));
    // The base stays the active setup.
    assert_eq!(job.active_setup_id(), Some(job.setups[0].id));
    // Both programs pass the checker (run_setups refuses one that doesn't), and the plug
    // has none of the warnings of §6 but the tip's.
    let runs = run_setups(&job, None).unwrap();
    let plug_run = &runs.iter().find(|(id, _)| *id == made.setup).unwrap().1;
    let warned: Vec<String> = plug_run
        .notes
        .iter()
        .map(|n| n.message.english())
        .filter(|m| m.contains("inlay plug"))
        .collect();
    assert!(warned.is_empty(), "{warned:#?}");
}

#[test]
fn the_plug_turned_over_fits_the_pocket() {
    // A 2 mm floor end mill reaches the channel's corners; the text has an O, narrow
    // strokes and wide ones.
    let (job, made) = inlay_of(2, 3, 0.0);
    let cell = 0.05;
    let fit = fit(&job, &made, cell, 3.0, PLUG_START);
    let (tol, wall) = tolerance(60.0, cell);
    assert!(fit.points > 100_000, "{}", fit.points);
    assert!(
        fit.overlap <= tol,
        "the plug overlaps the base by {}",
        fit.overlap
    );
    assert!(
        fit.off_wall <= wall,
        "the plug stands {} off a wall",
        fit.off_wall
    );
    // Nothing of the plug board is carved below the channel floor, D + g.
    assert!(
        fit.deepest_plug <= PLUG_START + GAP + 1e-3,
        "{}",
        fit.deepest_plug
    );
    // Outside the artwork, nothing of the plug board stands above its outline level: the
    // base's surface once turned over. "Outside" is where the pocket is less than a cell
    // deep, which takes in a sliver inside the outline where the plug may be a cell higher.
    assert!(
        fit.shallowest_outside >= PLUG_START - cell - tol,
        "{}",
        fit.shallowest_outside
    );
}

/// The plug's problems' paths and short texts, the project's checks before planning.
fn problems(job: &Job) -> Vec<(String, String)> {
    chipload_pipeline::job_problems(job)
        .into_iter()
        .map(|p| (p.path, p.short))
        .collect()
}

/// The base board's warnings about the inlay's pocket, in English.
fn pocket_warnings(job: &Job) -> Vec<String> {
    run_setups(job, Some(job.setups[0].id))
        .unwrap()
        .remove(0)
        .1
        .notes
        .iter()
        .map(|n| n.message.english())
        .filter(|m| m.contains("inlay pocket"))
        .collect()
}

/// The plug board's warnings about the plug, in English.
fn plug_warnings(job: &Job, made: &InlayMade) -> Vec<String> {
    run_setups(job, Some(made.setup))
        .unwrap()
        .remove(0)
        .1
        .notes
        .iter()
        .map(|n| n.message.english())
        .filter(|m| m.contains("inlay plug"))
        .collect()
}

fn plug_mut<'a>(job: &'a mut Job, made: &InlayMade) -> &'a mut chipload_job::VcarveOp {
    let Operation::Vcarve(plug) = &mut job.operations[made.plug] else {
        panic!()
    };
    plug
}

#[test]
fn the_plug_start_lies_between_its_start_and_end_depths() {
    let (job, made) = inlay_of(2, 1, 0.0);
    let path = format!("operations.{}.plug.start_depth", made.plug);
    for (start, refused) in [
        (0.0, true),
        (0.001, true),
        (0.01, false),
        (4.49, false),
        (4.5, true),
    ] {
        let mut job = job.clone();
        plug_mut(&mut job, &made).plug.as_mut().unwrap().start_depth = start;
        let found = problems(&job).iter().any(|(p, _)| *p == path);
        assert_eq!(found, refused, "start depth {start}: {:?}", problems(&job));
    }
}

#[test]
fn a_plug_never_ignores_kinks() {
    let (mut job, made) = inlay_of(2, 1, 0.0);
    plug_mut(&mut job, &made).ignore_kinks_deg = 10.0;
    let path = format!("operations.{}.ignore_kinks_deg", made.plug);
    assert!(problems(&job).iter().any(|(p, _)| *p == path));
}

#[test]
fn a_plug_as_deep_as_its_v_bit_cuts_and_no_deeper() {
    let (mut job, made) = inlay_of(2, 1, 0.0);
    // A V-bit 5.196 mm across cuts 4.5 mm deep with its cone: exactly the channel's depth.
    let across = 2.0 * 4.5 * 30f64.to_radians().tan();
    job.tools[1].diameter = across;
    let path = format!("operations.{}.end_depth", made.plug);
    let deep = |job: &Job| {
        problems(job)
            .iter()
            .any(|(p, s)| *p == path && s.contains("V-bit"))
    };
    assert!(!deep(&job), "{:?}", problems(&job));
    plug_mut(&mut job, &made).end_depth = 4.51;
    assert!(deep(&job), "{:?}", problems(&job));
}

#[test]
fn a_plug_part_with_copies_is_refused() {
    let (mut job, made) = inlay_of(2, 1, 0.0);
    job.parts
        .iter_mut()
        .find(|p| p.id == made.part)
        .unwrap()
        .copies = Some(chipload_job::Copies {
        columns: 2,
        rows: 1,
        dx: 100.0,
        dy: 0.0,
        order: chipload_job::CopyOrder::ByCopy,
    });
    let path = format!("operations.{}.part", made.plug);
    assert!(
        problems(&job).iter().any(|(p, _)| *p == path),
        "{:?}",
        problems(&job)
    );
}

#[test]
fn the_plug_warns_about_what_would_spoil_the_fit() {
    let (job, made) = inlay_of(2, 3, 0.0);
    assert!(plug_warnings(&job, &made).is_empty());
    let warned = |job: &Job, what: &str| {
        let warnings = plug_warnings(job, &made);
        assert!(
            warnings.iter().any(|w| w.contains(what)),
            "{what}: {warnings:#?}"
        );
    };
    // Its pocket no deeper than the plug's face: it would sit on the pocket's floor.
    let mut shallow = job.clone();
    let Operation::Vcarve(pocket) = &mut shallow.operations[made.pocket] else {
        panic!()
    };
    pocket.end_depth = PLUG_START;
    warned(&shallow, "hit the pocket's floor");
    // Its pocket ignoring kinks.
    let mut kinks = job.clone();
    let Operation::Vcarve(pocket) = &mut kinks.operations[made.pocket] else {
        panic!()
    };
    pocket.ignore_kinks_deg = 10.0;
    warned(&kinks, "ignores kinks");
    // No pocket at all.
    let mut alone = job.clone();
    alone.operations[made.pocket].set_enabled(false);
    warned(&alone, "no pocket for it");
    // Its part not mirrored.
    let mut plain = job.clone();
    plain
        .parts
        .iter_mut()
        .find(|p| p.id == made.part)
        .unwrap()
        .mirror = None;
    warned(&plain, "isn't mirrored");
    // A small glue gap, a thin back, no floor pass.
    let mut gap = job.clone();
    plug_mut(&mut gap, &made).end_depth = PLUG_START + 0.4;
    warned(&gap, "glue gap");
    let mut thin = job.clone();
    thin.setups
        .iter_mut()
        .find(|s| s.id == made.setup)
        .unwrap()
        .stock
        .thickness = 6.0;
    warned(&thin, "left under the channel");
    let mut bare = job.clone();
    plug_mut(&mut bare, &made).floor = None;
    warned(&bare, "without a floor pass");
    // The pocket: its wide strokes' floor left.
    assert!(
        pocket_warnings(&job).is_empty(),
        "{:#?}",
        pocket_warnings(&job)
    );
    let mut open = job.clone();
    let Operation::Vcarve(pocket) = &mut open.operations[made.pocket] else {
        panic!()
    };
    pocket.floor = None;
    let warnings = pocket_warnings(&open);
    assert!(
        warnings.iter().any(|w| w.contains("without a floor pass")),
        "{warnings:#?}"
    );
    // A floor pass that leaves part of a narrow floor standing above the plug's face: the
    // case the property test below found, a 45° bit and a face 1 mm down in a pocket 1.5 mm
    // deep.
    let (star, _) = polygon_inlay(&[6.0, 6.0, 6.0, 20.07, 6.0], 0.0, true, 45.0, 1.0, 0.5, 1.5);
    let warnings = pocket_warnings(&star);
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("above the plug's face (1 mm)")),
        "{warnings:#?}"
    );
    // A flat tip: how far from a right-angled corner the fit falls short.
    let (flat, made) = inlay_of(2, 1, 0.2);
    let warnings = plug_warnings(&flat, &made);
    assert!(
        warnings.iter().any(|w| w.contains("within 0.14 mm")),
        "{warnings:#?}"
    );
}

/// A closed polygon of drawn lines on layer "art", star-shaped round (60, 50): `radii` at
/// even angles turned by `turn`; and, when `hole`, a small square in the middle as a hole.
/// An inlay of the layer with a `angle`° V-bit, pointed, a 2 mm floor end mill, the
/// plug's outline `outline` down, the glue gap `gap`, the pocket `pocket` deep.
fn polygon_inlay(
    radii: &[f64],
    turn: f64,
    hole: bool,
    angle: f64,
    outline: f64,
    gap: f64,
    pocket: f64,
) -> (Job, InlayMade) {
    let mut data = corpus_value("jobs/v27/text.chipload");
    let ring = |points: Vec<(f64, f64)>| {
        let n = points.len();
        (0..n)
            .map(|k| {
                let (a, b) = (points[k], points[(k + 1) % n]);
                serde_json::json!({"line": {"start": [a.0, a.1], "end": [b.0, b.1]}})
            })
            .collect::<Vec<_>>()
    };
    let n = radii.len();
    let star: Vec<(f64, f64)> = radii
        .iter()
        .enumerate()
        .map(|(k, r)| {
            let a = turn + std::f64::consts::TAU * k as f64 / n as f64;
            (60.0 + r * a.cos(), 50.0 + r * a.sin())
        })
        .collect();
    let mut shapes = vec![serde_json::json!({
        "id": 1, "layer": "art", "kind": "path", "dxf_handle": null, "dxf_type": null,
        "closed": true, "segments": ring(star)
    })];
    if hole {
        let square = vec![(58.0, 48.0), (62.0, 48.0), (62.0, 52.0), (58.0, 52.0)];
        shapes.push(serde_json::json!({
            "id": 2, "layer": "art", "kind": "path", "dxf_handle": null, "dxf_type": null,
            "closed": true, "segments": ring(square)
        }));
    }
    data["drawing"]["shapes"] = serde_json::Value::Array(shapes);
    data["drawing"]["next_shape_id"] = 3.into();
    data["drawing"]["texts"] = serde_json::json!([]);
    data["operations"] = serde_json::json!([]);
    let mut job = chipload_job::parse_job(data, "test").unwrap();
    job.tools[1].angle = Some(angle.to_radians());
    job.tools[1].tip_diameter = Some(0.0);
    job.tools[1].diameter = 25.0;
    let mut mill = job.tools[0].clone();
    mill.id = 3;
    mill.diameter = 2.0;
    job.tools.push(mill);
    job.stock_mut().thickness = 12.0;
    let drawing = load_drawing(&job).unwrap();
    let request = InlayRequest {
        select: Selector {
            layer: Some("art".into()),
            ..Selector::default()
        },
        tool: 2,
        depth: pocket,
        plug_start: outline,
        gap,
        floor_tool: Some(3),
        plug_thickness: outline + gap + 3.0,
    };
    add_inlay(
        &job,
        &request,
        &drawing.contours,
        None,
        chipload_i18n::Language::En,
    )
    .unwrap()
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 6, ..ProptestConfig::default() })]

    /// Random polygons, angles and depths: the plug turned over fits its pocket (§9).
    #[test]
    fn any_polygon_plug_fits_its_pocket(
        radii in prop::collection::vec(6.0f64..22.0, 5..9),
        turn in 0.0f64..1.0,
        hole in any::<bool>(),
        angle in 45.0f64..100.0,
        outline in 1.0f64..3.0,
        gap in 0.5f64..2.0,
        under in 0.5f64..2.0,
    ) {
        let (job, made) = polygon_inlay(&radii, turn, hole, angle, outline, gap, outline + under);
        // A pocket whose floor pass leaves part of its floor above the plug's face says so,
        // and then the plug may rest on it: the fit is promised only without that warning.
        prop_assume!(pocket_warnings(&job).is_empty());
        // The same for the plug's channel.
        prop_assume!(plug_warnings(&job, &made).is_empty());
        let cell = 0.05;
        let fit = fit(&job, &made, cell, 3.0, outline);
        let (tol, wall) = tolerance(angle, cell);
        prop_assert!(fit.points > 10_000);
        prop_assert!(fit.overlap <= tol, "overlap {} > {tol}", fit.overlap);
        prop_assert!(fit.off_wall <= wall, "off a wall {} > {wall}", fit.off_wall);
        prop_assert!(fit.deepest_plug <= outline + gap + 1e-3);
        prop_assert!(fit.shallowest_outside >= outline - cell - tol);
    }
}
