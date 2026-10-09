// SPDX-License-Identifier: GPL-3.0-or-later
//! Golden G-code: `chipload run` on the corpus jobs must reproduce tests/corpus/expected/
//! byte for byte, and twice in a row.
//!
//! When a change is intended, regenerate with `CHIPLOAD_UPDATE_GOLDEN=1 cargo test -p
//! chipload-cli --test golden`, review the diff, and explain it in the commit message.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;

const GOLDEN: [(&str, &str, &str); 5] = [
    (
        "v1_example_job",
        "jobs/v1/example_job.json",
        "dxf/simple_part.dxf",
    ),
    (
        "v2_simple_part_full",
        "jobs/v2/simple_part_full.json",
        "dxf/simple_part.dxf",
    ),
    (
        "v3_messy_2_lower",
        "jobs/v3/messy_2_lower.json",
        "dxf/messy_2.dxf",
    ),
    (
        "v3_simple_part_tabs",
        "jobs/v3/simple_part_tabs.json",
        "dxf/simple_part.dxf",
    ),
    (
        "v3_messy_2_lower_tabs",
        "jobs/v3/messy_2_lower_tabs.json",
        "dxf/messy_2.dxf",
    ),
];

fn corpus(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus")
        .join(rel)
}

fn run(job: &str, dxf: &str, out: &Path) -> String {
    let status = Command::new(env!("CARGO_BIN_EXE_chipload"))
        .args(["run"])
        .arg(corpus(dxf))
        .arg("--job")
        .arg(corpus(job))
        .arg("-o")
        .arg(out)
        .output()
        .unwrap();
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    std::fs::read_to_string(out).unwrap()
}

#[test]
fn golden_gcode() {
    let dir = std::env::temp_dir().join(format!("chipload-golden-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let update = std::env::var_os("CHIPLOAD_UPDATE_GOLDEN").is_some();
    let mut mismatches = Vec::new();
    for (name, job, dxf) in GOLDEN {
        let first = run(job, dxf, &dir.join(format!("{name}-1.nc")));
        let second = run(job, dxf, &dir.join(format!("{name}-2.nc")));
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
