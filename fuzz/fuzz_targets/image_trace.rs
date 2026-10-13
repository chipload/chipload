// SPDX-License-Identifier: GPL-3.0-or-later
//! Pictures on any bytes (docs/specs/image-trace.md §8, §9): decoding may refuse a file with
//! an error; it must never panic outside the third-party decoder (whose panics become
//! errors), never hang, and a picture it reads must trace into loops of finite points.
#![no_main]

use chipload_image::{Settings, decode, trace};
use libfuzzer_sys::fuzz_target;

fuzz_target!(
    init: {
        // As in dxf_import: panics the decoder's guard catches on purpose stay quiet.
        std::panic::set_hook(Box::new(|_| {}));
    },
    |data: &[u8]| {
    let Ok(picture) = decode(data) else {
        return;
    };
    let Ok(traced) = trace(&picture, &Settings::default(), 100.0) else {
        return;
    };
    for ring in &traced.loops {
        assert!(ring.len() >= 3, "a loop of {} points", ring.len());
        for &(x, y) in ring {
            assert!(x.is_finite() && y.is_finite(), "({x}, {y})");
        }
    }
});
