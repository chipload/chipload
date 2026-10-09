// SPDX-License-Identifier: GPL-3.0-or-later
//! Chipload desktop GUI (egui). Usage: `chipload-gui [part.dxf] [job.json]`.

// Startup errors go to the terminal before any window exists.
#![allow(clippy::print_stderr)]

mod app;
mod editor;
mod fonts;
mod view;

use std::path::PathBuf;
use std::process::ExitCode;

use eframe::egui;

fn main() -> ExitCode {
    let fonts = match fonts::system_fonts() {
        Ok(fonts) => fonts,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::FAILURE;
        }
    };
    // Paths on the command line, in any order: .json is a job, anything else a drawing.
    let (jobs, drawings): (Vec<PathBuf>, Vec<PathBuf>) = std::env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .partition(|p| {
            p.extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("json"))
        });

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Chipload")
            .with_inner_size([1200.0, 800.0]),
        ..Default::default()
    };
    let result = eframe::run_native(
        "Chipload",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_fonts(fonts);
            let mut app = app::ChiploadApp::default();
            if let Some(job) = jobs.first() {
                app.open_job(job);
            }
            if let Some(dxf) = drawings.first() {
                app.open_dxf(dxf);
            }
            Ok(Box::new(app))
        }),
    );
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}
