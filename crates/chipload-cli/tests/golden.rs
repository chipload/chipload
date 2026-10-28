// SPDX-License-Identifier: GPL-3.0-or-later
//! Golden G-code: `chipload run` on the frozen corpus projects (tests/corpus/jobs/v4 on)
//! must reproduce tests/corpus/expected/ byte for byte, twice in a row. The same projects
//! without arcs must reproduce tests/corpus/expected/lines/ (docs/specs/arc-output.md).
//!
//! When a change is intended, regenerate with `CHIPLOAD_UPDATE_GOLDEN=1 cargo test -p
//! chipload-cli --test golden`, review the diff, and explain it in the commit message.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

/// (golden name, frozen project). The names say which schema version the job first had;
/// the projects are frozen at the version in their path and migrate on load.
const GOLDEN: [(&str, &str); 51] = [
    ("v1_example_job", "jobs/v4/example_job.chipload"),
    ("v2_simple_part_full", "jobs/v4/simple_part_full.chipload"),
    ("v3_messy_2_lower", "jobs/v4/messy_2_lower.chipload"),
    ("v3_simple_part_tabs", "jobs/v4/simple_part_tabs.chipload"),
    (
        "v3_messy_2_lower_tabs",
        "jobs/v4/messy_2_lower_tabs.chipload",
    ),
    ("v5_drawn_part", "jobs/v5/drawn_part.chipload"),
    // The same project frozen at v6: must give the same G-code after migrating to v7.
    ("v6_drawn_part", "jobs/v6/drawn_part.chipload"),
    // Start depth, clearance (both signs) and a cut into the spoilboard.
    (
        "v7_depths_and_clearance",
        "jobs/v7/depths_and_clearance.chipload",
    ),
    ("v7_ramp_entry", "jobs/v7/ramp_entry.chipload"),
    // The ramp-entry part cut six times, operation by operation (docs/specs/copies-grid.md).
    ("v7_copies", "jobs/v7/copies.chipload"),
    // The ramp-entry part with leads and moved starts (docs/specs/helix-and-leads.md).
    ("v7_leads", "jobs/v7/leads.chipload"),
    // The same, its pockets going down round a helix (helical G2/G3).
    ("v7_helix", "jobs/v7/helix.chipload"),
    // The simple part frozen at v7 (an imported drawing): the same G-code as
    // v2_simple_part_full after migrating to v8 (docs/specs/drawing-placement.md).
    ("v7_simple_part_full", "jobs/v7/simple_part_full.chipload"),
    // The same, its drawing moved by (-20, 30) with a start point on the outline.
    ("v8_placed_part", "jobs/v8/placed_part.chipload"),
    // The same, turned 90° and put in the stock's corner (docs/specs/drawing-rotation.md).
    ("v9_turned_part", "jobs/v9/turned_part.chipload"),
    // Serif letters V-carved with a 60° bit in two passes (docs/specs/v-carve.md).
    ("v10_vcarve_letters", "jobs/v10/vcarve_letters.chipload"),
    ("v11_scaled_part", "jobs/v11/scaled_part.chipload"),
    // A pocket with a 3 mm tool, the outline with 6 mm: one file per tool
    // (docs/specs/tool-per-operation.md), v11_two_tools-1-T2.nc and v11_two_tools-2-T1.nc.
    ("v11_two_tools", "jobs/v11/two_tools.chipload"),
    // The same in one file, T M6 at the tool change (machine tool_change: program).
    (
        "v12_two_tools_program",
        "jobs/v12/two_tools_program.chipload",
    ),
    // v7_helix frozen at v13, a rings pocket: v14 keeps its corners sharp, the same G-code
    // as v7_helix (docs/specs/pocket-round-corners.md).
    ("v13_helix_rings", "jobs/v13/helix_rings.chipload"),
    // A rings pocket round two islands with rounded ring corners
    // (docs/specs/pocket-round-corners.md).
    ("v14_round_corners", "jobs/v14/round_corners.chipload"),
    // v7_helix (helix, leads, tabs, arcs) with the LinuxCNC and Mach3/4 posts
    // (docs/specs/user-posts.md §5): the same moves in their dialects.
    ("v15_linuxcnc_helix", "jobs/v15/linuxcnc_helix.chipload"),
    ("v15_mach3_helix", "jobs/v15/mach3_helix.chipload"),
    ("v15_straight_tabs", "jobs/v15/straight_tabs.chipload"),
    ("v16_profile_spiral", "jobs/v16/profile_spiral.chipload"),
    ("v17_follow_path", "jobs/v17/follow_path.chipload"),
    // Finishing laps and an onion skin, with ramped tabs (docs/specs/finish-and-skin.md).
    ("v17_finish_skin", "jobs/v17/finish_skin.chipload"),
    // Drilled holes in pecks and helical bores, one through with a core (docs/specs/holes.md).
    ("v18_holes", "jobs/v18/holes.chipload"),
    // Dogbones in a mortise pocket and a notched outline (with tabs), T-bones in a slot
    // (docs/specs/dogbones.md).
    ("v18_dogbones", "jobs/v18/dogbones.chipload"),
    // A 12 mm pocket round an island with a slot too narrow for it, then a 4 mm rest pocket
    // (docs/specs/rest-machining.md).
    ("v18_rest", "jobs/v18/rest.chipload"),
    // A picture traced into shapes and V-carved (docs/specs/image-trace.md §9).
    ("v19_image_vcarve", "jobs/v19/image_vcarve.chipload"),
    // A picture of strokes traced into centre lines with arcs and followed with a 3 mm end
    // mill (docs/specs/image-arcs-centrelines.md §8).
    (
        "v20_image_centre_line",
        "jobs/v20/image_centre_line.chipload",
    ),
    // v11_scaled_part frozen at v20: v21 makes its DXF the first of a list; the same G-code
    // as v11_scaled_part (docs/specs/drawing-sources.md §8).
    ("v20_scaled_part", "jobs/v20/scaled_part.chipload"),
    // Two DXFs (the second added, scaled and moved), a profile of a layer both have, an
    // outline limited to each, and a traced logo V-carved (docs/specs/drawing-sources.md §8).
    ("v21_two_dxfs", "jobs/v21/two_dxfs.chipload"),
    // v7_copies cut copy by copy, frozen by Chipload 0.16.0 with `order: by_part` and its
    // G-code from that binary: v22 renames the order `by_copy` and moves the copies into
    // the one part; the G-code stays the same (docs/specs/parts.md §4).
    ("v21_copies_by_part", "jobs/v21/copies_by_part.chipload"),
    // Two parts (docs/specs/parts.md): DXF 1 with its holes, outline and a V-carved logo,
    // then DXF 2's outline in a 1 × 2 grid. Part by part: T1, T2, then T1 again, and a
    // comment where each part starts.
    ("v22_two_parts", "jobs/v22/two_parts.chipload"),
    // v22_two_parts with part B's DXF turned 90° too (both DXFs quarter-turned) and its
    // copies side by side, frozen at v23 with G-code from that code: v24 makes the quarter
    // turns an angle in degrees; the G-code stays the same (docs/specs/direct-placement.md).
    ("v23_turned_parts", "jobs/v23/turned_parts.chipload"),
    // v14_round_corners (a turned rectangle, pocketed and cut round) frozen at v24: v25
    // gives rectangles a corner radius, 0 for these; the G-code stays the same
    // (docs/specs/rectangle-corner-radius.md).
    ("v24_rectangle_part", "jobs/v24/rectangle_part.chipload"),
    // The same with the rectangle's corners rounded at 15 mm: rings and the outline follow
    // the corner arcs (docs/specs/rectangle-corner-radius.md).
    (
        "v25_rounded_rectangle",
        "jobs/v25/rounded_rectangle.chipload",
    ),
    // v15_straight_tabs frozen at v26: a profile that plunged, with straight tabs and a
    // lead; since v27 every profile spirals down, at 3° here, and a profile has no lead
    // (docs/specs/profile-always-spiral.md).
    ("v26_straight_tabs", "jobs/v26/straight_tabs.chipload"),
    // Text (docs/specs/text.md): two centred lines in FreeSans Bold, turned 8°, V-carved,
    // and a FreeSerif A pocketed round its counter.
    ("v27_text", "jobs/v27/text.chipload"),
    // Pockets that used the removed trochoidal and adaptive clearing (ADR 0017): since v26
    // they are rings pockets like any other (docs/specs/pocket.md as built 2026-10-15).
    ("v7_trochoidal", "jobs/v7/trochoidal.chipload"),
    ("v7_adaptive", "jobs/v7/adaptive.chipload"),
    ("v12_adaptive", "jobs/v12/adaptive.chipload"),
    (
        "v13_adaptive_two_islands",
        "jobs/v13/adaptive_two_islands.chipload",
    ),
    // Two pockets with a 12 mm tool, each with a 4 mm rest pass, then the outline: the
    // passes wait for both pockets, and the outline comes last (docs/specs/rest-passes.md
    // §3): files T1 (1, 2), T2 (1.1, 2.1), T1 (3).
    ("v28_rest_passes", "jobs/v28/rest_passes.chipload"),
    // A surface of the whole stock with a 25 mm endmill, first in part 1, before two parts
    // (part B with two copies): two depths back and forth, the second coming back the
    // other way, and the stock faced once (docs/specs/surfacing.md).
    ("v29_surface", "jobs/v29/surface.chipload"),
    // A part drawn and edited only with the editing commands (docs/specs/drawing-editing.md
    // §9): a pentagon pocketed, its sharp offset filleted and scalloped by a polar array
    // of circles subtracted, cut round. `cli.rs` draws it again from the commands.
    ("v30_drawn_edited", "jobs/v30/drawn_edited.chipload"),
    // An SVG (tests/corpus/svg/badge_layers.svg, Inkscape layers in mm, Bézier curves
    // fitted with arcs): layer Pocket pocketed round its island, layer Outline cut round
    // (docs/specs/svg-import.md §11). `tests/svg.rs` in the pipeline imports it again.
    ("v31_svg_badge", "jobs/v31/svg_badge.chipload"),
    // Built-in fonts beyond FreeFont (docs/specs/more-fonts.md): two lines of joined
    // Satisfy script, merged where the letters overlap, V-carved; an Ultra B pocketed.
    ("v29_script_text", "jobs/v29/script_text.chipload"),
    // v25_rounded_rectangle with a hexagon (a drawn polygon, schema 30) in place of the
    // triangle: rings round the circle and the hexagon, then the rounded outline
    // (docs/specs/drawing-input.md §5).
    ("v30_polygon", "jobs/v30/polygon.chipload"),
];

fn corpus(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus")
        .join(rel)
}

fn chipload(args: &[&OsStr]) {
    // An empty settings folder: tests never read the user's tool library.
    let settings =
        std::env::temp_dir().join(format!("chipload-golden-settings-{}", std::process::id()));
    let out = Command::new(env!("CARGO_BIN_EXE_chipload"))
        .env("CHIPLOAD_CONFIG_DIR", settings)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Run the project to `dir/<name>.nc`: the files written, as (file name, G-code), in name
/// order. One file, or one per tool group (`<name>-1-T2.nc`, …).
fn run(project: &Path, dir: &Path, name: &str) -> Vec<(String, String)> {
    let out = dir.join(format!("{name}.nc"));
    chipload(&[
        "run".as_ref(),
        project.as_os_str(),
        "-o".as_ref(),
        out.as_os_str(),
        // The sheets have golden files of their own (chipload-pipeline tests/sheet.rs).
        "--no-sheet".as_ref(),
    ]);
    let mut files: Vec<(String, String)> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|f| f == &format!("{name}.nc") || f.starts_with(&format!("{name}-")))
        .map(|f| {
            let text = std::fs::read_to_string(dir.join(&f)).unwrap();
            (f, text)
        })
        .collect();
    files.sort();
    files
}

#[test]
fn golden_gcode() {
    let base = std::env::temp_dir().join(format!("chipload-golden-{}", std::process::id()));
    let update = std::env::var_os("CHIPLOAD_UPDATE_GOLDEN").is_some();
    let mut mismatches = Vec::new();
    for (name, project) in GOLDEN {
        let project = corpus(project);
        let (one, two) = (
            base.join(format!("{name}-a")),
            base.join(format!("{name}-b")),
        );
        std::fs::create_dir_all(&one).unwrap();
        std::fs::create_dir_all(&two).unwrap();
        let first = run(&project, &one, name);
        let second = run(&project, &two, name);
        assert_eq!(first, second, "{name}: output differs between runs");
        assert!(!first.is_empty(), "{name}: no G-code written");
        for (file, text) in &first {
            let expected = corpus(&format!("expected/{file}"));
            if update {
                std::fs::write(&expected, text).unwrap();
            } else if std::fs::read_to_string(&expected).ok().as_ref() != Some(text) {
                mismatches.push(file.clone());
            }
        }
    }
    std::fs::remove_dir_all(&base).unwrap();
    assert!(
        mismatches.is_empty(),
        "golden G-code differs for {mismatches:?}; if intended, regenerate with \
         CHIPLOAD_UPDATE_GOLDEN=1 and explain the change in the commit message"
    );
}

/// The same project rendered with its post but `arcs = false`: what run_job does, with
/// that one setting changed. (file name, G-code) for each file, as `run` names them.
fn render_without_arcs(project: &Path, name: &str) -> Vec<(String, String)> {
    let job = chipload_job::load_job(project).unwrap();
    let drawing = chipload_pipeline::load_drawing(&job).unwrap();
    let plan = chipload_toolpath::plan_job(&job, &drawing.contours).unwrap();
    let mut post = chipload_pipeline::posts::job_post(&job).unwrap();
    post.arcs = false;
    let (files, _) =
        chipload_pipeline::post_and_check(&job, &post, &plan.program, &plan.groups).unwrap();
    files
        .into_iter()
        .map(|f| {
            let file = format!("{name}{}.nc", f.suffix.unwrap_or_default());
            (file, f.gcode)
        })
        .collect()
}

#[test]
fn without_arcs_the_output_is_the_line_only_golden() {
    // docs/specs/arc-output.md: `arcs = false` must give exactly the G-code from before arcs
    // (tests/corpus/expected/lines/, frozen when arcs were added).
    let update = std::env::var_os("CHIPLOAD_UPDATE_GOLDEN").is_some();
    for (name, project) in GOLDEN {
        for (file, lines) in render_without_arcs(&corpus(project), name) {
            let path = corpus(&format!("expected/lines/{file}"));
            if update {
                std::fs::write(&path, &lines).unwrap();
            }
            let golden = std::fs::read_to_string(&path).unwrap();
            assert!(lines == golden, "{file}: arcs = false changed the G-code");
            assert!(
                !lines
                    .lines()
                    .any(|l| l.starts_with("G2 ") || l.starts_with("G3 "))
            );
        }
    }
}

/// Every golden file of the GOLDEN projects (`expected/<name>.nc` or `<name>-…nc`).
fn golden_files() -> Vec<String> {
    let mut files: Vec<String> = std::fs::read_dir(corpus("expected"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|f| {
            GOLDEN.iter().any(|(name, _)| {
                f == &format!("{name}.nc")
                    || (f.starts_with(&format!("{name}-")) && f.ends_with(".nc"))
            })
        })
        .collect();
    files.sort();
    files
}

type Xy = (f64, f64);

/// (start, end, centre) of every G2/G3 line, as written.
fn arcs_in(gcode: &str) -> Vec<(Xy, Xy, Xy)> {
    let word = |line: &str, letter: char| {
        line.split_whitespace()
            .find_map(|w| w.strip_prefix(letter).and_then(|v| v.parse::<f64>().ok()))
    };
    let (mut x, mut y) = (0.0, 0.0);
    let mut arcs = Vec::new();
    for line in gcode.lines() {
        let (nx, ny) = (word(line, 'X').unwrap_or(x), word(line, 'Y').unwrap_or(y));
        if line.starts_with("G2 ") || line.starts_with("G3 ") {
            let (i, j) = (word(line, 'I').unwrap(), word(line, 'J').unwrap());
            arcs.push(((x, y), (nx, ny), (x + i, y + j)));
        }
        (x, y) = (nx, ny);
    }
    arcs
}

#[test]
fn every_golden_arc_is_consistent_as_written() {
    // After rounding to 3 decimals, start and end radius agree within 0.001 mm (GRBL checks
    // 0.005 mm, the independent checker 0.01 mm), and no arc is a full circle.
    let mut count = 0;
    for name in golden_files() {
        let gcode = std::fs::read_to_string(corpus(&format!("expected/{name}"))).unwrap();
        for (start, end, centre) in arcs_in(&gcode) {
            let r0 = (start.0 - centre.0).hypot(start.1 - centre.1);
            let r1 = (end.0 - centre.0).hypot(end.1 - centre.1);
            assert!(
                (r0 - r1).abs() <= 0.001,
                "{name}: {start:?} -> {end:?} around {centre:?}"
            );
            assert!(start != end, "{name}: full circle in one move");
            count += 1;
        }
    }
    assert!(count > 50, "the corpus should exercise arcs, found {count}");
}

// -- inch G-code (docs/specs/inch-units.md §4.4)

/// (golden name, project) rendered with the machine's G-code in inches: the three posts,
/// arcs and helices, tabs, holes, and both ways of changing tools.
const INCH: [(&str, &str); 6] = [
    ("v7_helix", "jobs/v7/helix.chipload"),
    ("v15_linuxcnc_helix", "jobs/v15/linuxcnc_helix.chipload"),
    ("v15_mach3_helix", "jobs/v15/mach3_helix.chipload"),
    ("v18_holes", "jobs/v18/holes.chipload"),
    ("v11_two_tools", "jobs/v11/two_tools.chipload"),
    (
        "v12_two_tools_program",
        "jobs/v12/two_tools_program.chipload",
    ),
];

/// The project's G-code in `units`, as run_job writes it (checked by the independent
/// checker against the machine's units): (file name, G-code) as `run` names them.
fn render_in(project: &Path, name: &str, units: chipload_job::Units) -> Vec<(String, String)> {
    let mut job = chipload_job::load_job(project).unwrap();
    job.machine.gcode_units = units;
    let drawing = chipload_pipeline::load_drawing(&job).unwrap();
    let plan = chipload_toolpath::plan_job(&job, &drawing.contours).unwrap();
    let post = chipload_pipeline::posts::job_post(&job).unwrap();
    let (files, _) =
        chipload_pipeline::post_and_check(&job, &post, &plan.program, &plan.groups).unwrap();
    files
        .into_iter()
        .map(|f| {
            let file = format!("{name}{}.nc", f.suffix.unwrap_or_default());
            (file, f.gcode)
        })
        .collect()
}

#[test]
fn inch_golden_gcode() {
    let update = std::env::var_os("CHIPLOAD_UPDATE_GOLDEN").is_some();
    let mut mismatches = Vec::new();
    for (name, project) in INCH {
        let files = render_in(&corpus(project), name, chipload_job::Units::Inch);
        for (file, text) in &files {
            assert!(text.contains("G20 G90"), "{file}: no G20");
            assert!(!text.contains("G21"), "{file}: G21 in an inch program");
            let expected = corpus(&format!("expected/inch/{file}"));
            if update {
                std::fs::write(&expected, text).unwrap();
            } else if std::fs::read_to_string(&expected).ok().as_ref() != Some(text) {
                mismatches.push(file.clone());
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "inch G-code differs for {mismatches:?}; if intended, regenerate with \
         CHIPLOAD_UPDATE_GOLDEN=1 and explain the change in the commit message"
    );
}

/// A line's words: its letter and number, as written.
fn words(line: &str) -> Vec<(char, f64)> {
    line.split_whitespace()
        .filter_map(|w| {
            let mut chars = w.chars();
            let letter = chars.next()?;
            Some((letter, chars.as_str().parse::<f64>().ok()?))
        })
        .collect()
}

/// One motion as a controller sees it: its G code, where it ends, its feed, and for an arc
/// its centre (all in mm).
#[derive(Debug, Clone, Copy)]
struct Motion {
    code: i64,
    end: [f64; 3],
    feed: f64,
    centre: Option<(f64, f64)>,
}

/// The program's motions, every length times `scale` (25.4 for inches); its other lines
/// (codes without motion) as text, comments left out.
fn motions(gcode: &str, scale: f64) -> (Vec<Motion>, Vec<String>) {
    let mut at = [f64::NAN; 3];
    let (mut code, mut feed) = (0, 0.0);
    let (mut moves, mut other) = (Vec::new(), Vec::new());
    for line in gcode.lines().filter(|l| !l.starts_with('(')) {
        let ws = words(line);
        let moving = ws.iter().any(|(l, _)| "XYZ".contains(*l));
        if !moving {
            other.push(line.replace("G20", "G21"));
            continue;
        }
        let start = at;
        let (mut i, mut j) = (None, None);
        for (letter, value) in ws {
            match letter {
                'G' => code = value as i64,
                'X' => at[0] = value * scale,
                'Y' => at[1] = value * scale,
                'Z' => at[2] = value * scale,
                'F' => feed = value * scale,
                'I' => i = Some(value * scale),
                'J' => j = Some(value * scale),
                _ => {}
            }
        }
        let centre = i.zip(j).map(|(i, j)| (start[0] + i, start[1] + j));
        moves.push(Motion {
            code,
            end: at,
            feed: if code == 0 { 0.0 } else { feed },
            centre,
        });
    }
    (moves, other)
}

#[test]
fn every_golden_project_moves_the_same_in_inches() {
    // The inch program is the mm program: the same codes, and every motion ends where the mm
    // one does, at the same feed, within both roundings (half a step of each; an arc's
    // centre is measured from its start as rounded and may take a neighbouring step to keep
    // its radii equal, so two steps of each). A move shorter than an inch step can vanish
    // in inches, where its end rounds to its start.
    let inch_step = 0.0001 * chipload_core::MM_PER_INCH;
    let mm_step = 0.001;
    let tolerance = (inch_step + mm_step) / 2.0 + 1e-9;
    let centre_tolerance = 2.0 * (inch_step + mm_step) + 1e-9;
    for (name, project) in GOLDEN {
        let project = corpus(project);
        let mm = render_in(&project, name, chipload_job::Units::Mm);
        let inch = render_in(&project, name, chipload_job::Units::Inch);
        assert_eq!(mm.len(), inch.len(), "{name}: number of files");
        for ((file, mm), (_, inch)) in mm.iter().zip(&inch) {
            let (a, other_a) = motions(mm, 1.0);
            let (b, other_b) = motions(inch, chipload_core::MM_PER_INCH);
            assert_eq!(other_a, other_b, "{file}: lines without motion");
            let close = |p: [f64; 3], q: [f64; 3], tol: f64| {
                p.iter()
                    .zip(&q)
                    .all(|(u, v)| (u.is_nan() && v.is_nan()) || (u - v).abs() <= tol)
            };
            let (mut i, mut j, mut skipped) = (0, 0, 0);
            while i < a.len() && j < b.len() {
                let (m, n) = (a[i], b[j]);
                let same = m.code == n.code
                    && close(m.end, n.end, tolerance)
                    && (m.feed - n.feed).abs() <= tolerance
                    && match (m.centre, n.centre) {
                        (Some(c), Some(d)) => {
                            (c.0 - d.0).abs() <= centre_tolerance
                                && (c.1 - d.1).abs() <= centre_tolerance
                        }
                        (None, None) => true,
                        _ => false,
                    };
                if same {
                    (i, j) = (i + 1, j + 1);
                    continue;
                }
                // Only a move too short to show may be missing: its end within an inch step
                // (plus the mm rounding) of the one before. The two grids don't nest, so it
                // may be missing from either program: two points can round together in mm
                // and apart in inches, or the other way round.
                let tiny = inch_step + mm_step;
                let before = |list: &[Motion], k: usize| {
                    if k > 0 {
                        list[k - 1].end
                    } else {
                        [f64::NAN; 3]
                    }
                };
                if close(m.end, before(&a, i), tiny) {
                    i += 1;
                } else if close(n.end, before(&b, j), tiny) {
                    j += 1;
                } else {
                    panic!("{file}: motion {} {m:?} vs {n:?}", i + 1);
                }
                skipped += 1;
            }
            // What's left at the end can only be such tiny moves.
            for k in i..a.len() {
                assert!(
                    close(a[k].end, a[k - 1].end, inch_step + mm_step),
                    "{file}: left over"
                );
                skipped += 1;
            }
            for k in j..b.len() {
                assert!(
                    close(b[k].end, b[k - 1].end, inch_step + mm_step),
                    "{file}: left over"
                );
                skipped += 1;
            }
            // At most 1 % of the moves (and 10 in any file): a long V-carve has thousands of
            // short moves, a few of them shorter than a rounding step.
            let most = (a.len() / 100).max(10);
            assert!(
                skipped <= most,
                "{file}: {skipped} moves vanished in inches"
            );
        }
    }
}
