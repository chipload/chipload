// SPDX-License-Identifier: GPL-3.0-or-later
//! Pictures on any bytes (docs/specs/image-trace.md §8, §9; image-arcs-centrelines.md §8):
//! decoding may refuse a file with an error; it must never panic outside the third-party
//! decoder (whose panics become errors), never hang, and a picture it reads must trace into
//! loops of finite points, with lines only or with arcs, and into centre lines.
#![no_main]

use chipload_core::fit::fit_path;
use chipload_core::primitives::Primitive;
use chipload_image::{Settings, decode, trace, trace_raw};
use chipload_toolpath::centre::centre_lines;
use libfuzzer_sys::fuzz_target;

fn finite(pieces: &[Primitive]) {
    for piece in pieces {
        for (x, y) in [piece.start(), piece.end()] {
            assert!(x.is_finite() && y.is_finite(), "({x}, {y})");
        }
    }
}

fuzz_target!(
    init: {
        // As in dxf_import: panics the decoder's guard catches on purpose stay quiet.
        std::panic::set_hook(Box::new(|_| {}));
    },
    |data: &[u8]| {
    let Ok(picture) = decode(data) else {
        return;
    };
    for arcs in [false, true] {
        let settings = Settings { arcs, ..Settings::default() };
        let Ok(traced) = trace(&picture, &settings, 100.0) else {
            return;
        };
        for ring in &traced.loops {
            assert!(ring.len() >= 3, "a loop of {} points", ring.len());
            for &(x, y) in ring {
                assert!(x.is_finite() && y.is_finite(), "({x}, {y})");
            }
        }
        for pieces in &traced.pieces {
            finite(pieces);
        }
    }
    let settings = Settings::default();
    let Ok(raw) = trace_raw(&picture, &settings, 100.0) else {
        return;
    };
    // An error (an outline the centre line can't be built in) is allowed; a panic isn't.
    if let Ok(found) = centre_lines(&raw.loops, settings.smoothing) {
        for path in &found.paths {
            finite(&fit_path(&path.points, path.closed, settings.smoothing, true));
        }
    }
});
