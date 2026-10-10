// SPDX-License-Identifier: GPL-3.0-or-later
//! Golden G-code: `chipload run` on the corpus projects (tests/corpus/jobs/v4, v5) must reproduce
//! tests/corpus/expected/ byte for byte, twice in a row; and embedding each frozen older job
//! (`chipload import --into`) must give the same G-code.
//!
//! When a change is intended, regenerate with `CHIPLOAD_UPDATE_GOLDEN=1 cargo test -p
//! chipload-cli --test golden`, review the diff, and explain it in the commit message.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

/// (golden name, frozen v4 project, the older job it was embedded from, its drawing)
const GOLDEN: [(&str, &str, &str, &str); 5] = [
    (
        "v1_example_job",
        "jobs/v4/example_job.chipload",
        "jobs/v1/example_job.json",
        "dxf/simple_part.dxf",
    ),
    (
        "v2_simple_part_full",
        "jobs/v4/simple_part_full.chipload",
        "jobs/v2/simple_part_full.json",
        "dxf/simple_part.dxf",
    ),
    (
        "v3_messy_2_lower",
        "jobs/v4/messy_2_lower.chipload",
        "jobs/v3/messy_2_lower.json",
        "dxf/messy_2.dxf",
    ),
    (
        "v3_simple_part_tabs",
        "jobs/v4/simple_part_tabs.chipload",
        "jobs/v3/simple_part_tabs.json",
        "dxf/simple_part.dxf",
    ),
    (
        "v3_messy_2_lower_tabs",
        "jobs/v4/messy_2_lower_tabs.chipload",
        "jobs/v3/messy_2_lower_tabs.json",
        "dxf/messy_2.dxf",
    ),
];

/// (golden name, project) for projects with no older version: shapes drawn in Chipload.
const DRAWN: [(&str, &str); 1] = [("v5_drawn_part", "jobs/v5/drawn_part.chipload")];

fn corpus(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus")
        .join(rel)
}

fn chipload(args: &[&OsStr]) {
    let out = Command::new(env!("CARGO_BIN_EXE_chipload"))
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
    let projects = GOLDEN
        .iter()
        .map(|&(name, project, _, _)| (name, project))
        .chain(DRAWN);
    for (name, project) in projects {
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

#[test]
fn embedding_each_frozen_older_job_gives_its_golden_gcode() {
    let dir = std::env::temp_dir().join(format!("chipload-embed-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for (name, _, job, dxf) in GOLDEN {
        let project = dir.join(format!("{name}.chipload"));
        chipload(&[
            "import".as_ref(),
            corpus(dxf).as_os_str(),
            "--into".as_ref(),
            corpus(job).as_os_str(),
            "-o".as_ref(),
            project.as_os_str(),
        ]);
        let gcode = run(&project, &dir.join(format!("{name}.nc")));
        let golden = std::fs::read_to_string(corpus(&format!("expected/{name}.nc"))).unwrap();
        assert!(gcode == golden, "{name}: embedding changed the G-code");
    }
    std::fs::remove_dir_all(&dir).unwrap();
}
