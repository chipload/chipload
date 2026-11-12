// SPDX-License-Identifier: GPL-3.0-or-later
//! The STL reader on any bytes (docs/specs/stl-relief.md §2, §11; CLAUDE.md: treat all
//! input as hostile). It may refuse a file with an error; it must never panic, never hang,
//! and never hand on a corner that isn't finite; its shadow must not panic either.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok((mesh, report)) = chipload_mesh::read_stl(data) else {
        return;
    };
    assert_eq!(report.triangles, mesh.triangles.len());
    assert!(
        mesh.triangles.iter().flatten().flatten().all(|v| v.is_finite()),
        "a corner that isn't finite"
    );
    if let Some((lo, hi)) = mesh.bounds() {
        let cell = ((hi[0] - lo[0]).max(hi[1] - lo[1]) / 100.0).max(1e-3);
        let _ = mesh.shadow(cell, 1_000_000);
    }
});
