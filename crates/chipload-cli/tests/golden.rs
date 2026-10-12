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
const GOLDEN: [(&str, &str); 25] = [
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
    // The same, its pocket cleared with trochoidal loops (docs/specs/trochoidal.md).
    ("v7_trochoidal", "jobs/v7/trochoidal.chipload"),
    ("v7_ramp_entry", "jobs/v7/ramp_entry.chipload"),
    // The same, its pocket cleared adaptively (docs/specs/adaptive.md).
    ("v7_adaptive", "jobs/v7/adaptive.chipload"),
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
    // v7_adaptive frozen at v12 (load 0.1): the load becomes the stepover in v13, the same
    // G-code as v7_adaptive (docs/specs/adaptive-fronts.md §5).
    ("v12_adaptive", "jobs/v12/adaptive.chipload"),
    // The pocket of the adaptive-fronts request: a rectangle round two round islands,
    // cleared by fronts at stepover 0.4 (docs/specs/adaptive-fronts.md).
    (
        "v13_adaptive_two_islands",
        "jobs/v13/adaptive_two_islands.chipload",
    ),
    // v7_helix frozen at v13, a rings pocket: v14 keeps its corners sharp, the same G-code
    // as v7_helix (docs/specs/pocket-round-corners.md).
    ("v13_helix_rings", "jobs/v13/helix_rings.chipload"),
    // A rings pocket round two islands with rounded ring corners
    // (docs/specs/pocket-round-corners.md).
    ("v14_round_corners", "jobs/v14/round_corners.chipload"),
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

/// The same project rendered with the GRBL post but `arcs = false`: what run_job does, with
/// that one setting changed. (file name, G-code) for each file, as `run` names them.
fn render_without_arcs(project: &Path, name: &str) -> Vec<(String, String)> {
    let job = chipload_job::load_job(project).unwrap();
    let drawing = chipload_pipeline::load_drawing(&job).unwrap();
    let plan = chipload_toolpath::plan_job(&job, &drawing.contours).unwrap();
    let mut post = chipload_post::load_builtin_post("grbl").unwrap();
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
