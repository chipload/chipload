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
const GOLDEN: [(&str, &str); 19] = [
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

fn run(project: &Path, out: &Path) -> String {
    chipload(&[
        "run".as_ref(),
        project.as_os_str(),
        "-o".as_ref(),
        out.as_os_str(),
    ]);
    std::fs::read_to_string(out).unwrap()
}

#[test]
fn golden_gcode() {
    let dir = std::env::temp_dir().join(format!("chipload-golden-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let update = std::env::var_os("CHIPLOAD_UPDATE_GOLDEN").is_some();
    let mut mismatches = Vec::new();
    for (name, project) in GOLDEN {
        let project = corpus(project);
        let first = run(&project, &dir.join(format!("{name}-1.nc")));
        let second = run(&project, &dir.join(format!("{name}-2.nc")));
        assert_eq!(first, second, "{name}: output differs between runs");
        let expected = corpus(&format!("expected/{name}.nc"));
        if update {
            std::fs::write(&expected, &first).unwrap();
        } else if std::fs::read_to_string(&expected).unwrap() != first {
            mismatches.push(name);
        }
    }
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(
        mismatches.is_empty(),
        "golden G-code differs for {mismatches:?}; if intended, regenerate with \
         CHIPLOAD_UPDATE_GOLDEN=1 and explain the change in the commit message"
    );
}

/// The same project rendered with the GRBL post but `arcs = false`: what run_job does, with
/// that one setting changed.
fn render_without_arcs(project: &Path) -> String {
    let job = chipload_job::load_job(project).unwrap();
    let drawing = chipload_pipeline::load_drawing(&job).unwrap();
    let plan = chipload_toolpath::plan_job(&job, &drawing.contours).unwrap();
    let output = chipload_pipeline::in_work_coordinates(&plan.program, job.work_zero_xy());
    let mut post = chipload_post::load_builtin_post("grbl").unwrap();
    post.arcs = false;
    chipload_post::render_program_with(&output, &post, Some(&job.work_zero_text())).unwrap()
}

#[test]
fn without_arcs_the_output_is_the_line_only_golden() {
    // docs/specs/arc-output.md: `arcs = false` must give exactly the G-code from before arcs
    // (tests/corpus/expected/lines/, frozen when arcs were added).
    let update = std::env::var_os("CHIPLOAD_UPDATE_GOLDEN").is_some();
    for (name, project) in GOLDEN {
        let lines = render_without_arcs(&corpus(project));
        let path = corpus(&format!("expected/lines/{name}.nc"));
        if update {
            std::fs::write(&path, &lines).unwrap();
        }
        let golden = std::fs::read_to_string(&path).unwrap();
        assert!(lines == golden, "{name}: arcs = false changed the G-code");
        assert!(
            !lines
                .lines()
                .any(|l| l.starts_with("G2 ") || l.starts_with("G3 "))
        );
    }
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
    for (name, _) in GOLDEN {
        let gcode = std::fs::read_to_string(corpus(&format!("expected/{name}.nc"))).unwrap();
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
