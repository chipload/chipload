// SPDX-License-Identifier: GPL-3.0-or-later
//! Golden simulation numbers (docs/specs/simulation.md §7): `chipload simulate` on corpus
//! projects must print tests/corpus/expected/sim/<name>.txt exactly, heights hash included.
//! Regenerate with `CHIPLOAD_UPDATE_GOLDEN=1` only when a change is intended.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;

/// (golden name, project): a profile with tabs, a pocket with a helix, a V-carve.
const GOLDEN: [(&str, &str); 3] = [
    ("simple_part_tabs", "jobs/v4/simple_part_tabs.chipload"),
    ("helix_rings", "jobs/v13/helix_rings.chipload"),
    ("vcarve_letters", "jobs/v10/vcarve_letters.chipload"),
];

fn corpus(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus")
        .join(rel)
}

fn simulate(project: &Path, extra: &[&str]) -> std::process::Output {
    let settings =
        std::env::temp_dir().join(format!("chipload-sim-settings-{}", std::process::id()));
    Command::new(env!("CARGO_BIN_EXE_chipload"))
        .env("CHIPLOAD_CONFIG_DIR", settings)
        .arg("simulate")
        .arg(project)
        .args(extra)
        .output()
        .unwrap()
}

#[test]
fn simulation_numbers_match_their_golden_files() {
    let update = std::env::var_os("CHIPLOAD_UPDATE_GOLDEN").is_some();
    for (name, project) in GOLDEN {
        let out = simulate(&corpus(project), &["--no-image", "--hash"]);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let text = String::from_utf8(out.stdout).unwrap();
        let golden = corpus(&format!("expected/sim/{name}.txt"));
        if update {
            std::fs::create_dir_all(golden.parent().unwrap()).unwrap();
            std::fs::write(&golden, &text).unwrap();
            continue;
        }
        let expected = std::fs::read_to_string(&golden).unwrap_or_else(|_| {
            panic!("{name}: no golden file; run with CHIPLOAD_UPDATE_GOLDEN=1")
        });
        assert_eq!(text, expected, "{name}: simulation changed");
    }
}

#[test]
fn the_image_is_written_and_the_same_every_time() {
    let dir = std::env::temp_dir().join(format!("chipload-sim-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let project = corpus("jobs/v10/vcarve_letters.chipload");
    let (a, b) = (dir.join("a.png"), dir.join("b.png"));
    for path in [&a, &b] {
        let out = simulate(&project, &["-o", path.to_str().unwrap()]);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let bytes = std::fs::read(&a).unwrap();
    assert_eq!(&bytes[1..4], b"PNG");
    assert_eq!(bytes, std::fs::read(&b).unwrap());
    // A cell size the stock can't take is refused.
    let out = simulate(&project, &["--no-image", "--cell", "0.01"]);
    assert!(!out.status.success());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn after_n_moves_cuts_less() {
    let project = corpus("jobs/v4/simple_part_tabs.chipload");
    let removed = |extra: &[&str]| -> f64 {
        let mut args = vec!["--no-image"];
        args.extend(extra);
        let out = simulate(&project, &args);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let text = String::from_utf8(out.stdout).unwrap();
        let line = text.lines().find(|l| l.starts_with("Simulation:")).unwrap();
        line.split_whitespace().nth(1).unwrap().parse().unwrap()
    };
    let (none, half, all) = (
        removed(&["--after", "0"]),
        removed(&["--after", "200"]),
        removed(&[]),
    );
    assert_eq!(none, 0.0);
    assert!(0.0 < half && half < all, "{half} of {all}");
    let out = simulate(&project, &["--no-image", "--after", "999999"]);
    assert!(!out.status.success());
}
