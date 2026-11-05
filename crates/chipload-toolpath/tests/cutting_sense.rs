// SPDX-License-Identifier: GPL-3.0-or-later
//! Every travel direction is chosen with the cutting sense, never the machine's spindle
//! itself (docs/specs/two-sided.md §4): a back setup is planned with the sense turned and
//! mirrored afterwards, and one direction chosen from `machine.spindle` would cut it the
//! wrong way round.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[test]
fn only_the_programs_own_spindle_reads_the_machine() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut reads = Vec::new();
    for entry in std::fs::read_dir(&src).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "rs") {
            let text = std::fs::read_to_string(&path).unwrap();
            for (n, line) in text.lines().enumerate() {
                if line.contains("machine.spindle") {
                    reads.push(format!("{}:{}: {}", path.display(), n + 1, line.trim()));
                }
            }
        }
    }
    // The one: the program's spindle, which the post writes as M3 or M4.
    assert_eq!(reads.len(), 1, "{reads:#?}");
    assert!(
        reads[0].contains("spindle: job.machine.spindle,"),
        "{reads:#?}"
    );
}
