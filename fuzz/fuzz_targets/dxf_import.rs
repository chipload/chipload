// SPDX-License-Identifier: GPL-3.0-or-later
//! The DXF importer on any bytes (CLAUDE.md: treat all input as hostile). It may refuse a
//! file with an error; it must never panic outside the third-party parser (whose panics it
//! turns into errors), never hang, and never hand on coordinates that aren't finite.
#![no_main]

use chipload_job::{PathSegment, Units};
use libfuzzer_sys::fuzz_target;

fuzz_target!(
    init: {
        // libfuzzer-sys aborts on every panic, also one the importer catches on purpose (a
        // panic in the third-party parser becomes an import error). A quiet hook lets those
        // be caught; a panic that escapes Chipload's own code still aborts the run, since
        // libfuzzer-sys checks every input for one.
        std::panic::set_hook(Box::new(|_| {}));
    },
    |data: &[u8]| {
    let Ok(imported) = chipload_dxf::shapes_from_dxf_bytes(data, "fuzz.dxf", Some(Units::Mm), 1.0)
    else {
        return;
    };
    for shape in &imported.shapes {
        for segment in shape.segments() {
            let numbers = match segment {
                PathSegment::Line(l) => vec![l.start[0], l.start[1], l.end[0], l.end[1]],
                PathSegment::Arc(a) => {
                    vec![a.center[0], a.center[1], a.radius, a.start_angle, a.sweep]
                }
            };
            assert!(
                numbers.iter().all(|v| v.is_finite()),
                "shape {} has a coordinate that isn't finite: {segment:?}",
                shape.id
            );
        }
    }
    // Contours, nesting and the report, as Chipload builds them after every import.
    let _ = chipload_dxf::contours_from_shapes(&imported.shapes, Some(&imported.source));
    }
);
