// SPDX-License-Identifier: GPL-3.0-or-later
//! Chipload desktop GUI (egui). Usage: `chipload-gui [project.chipload | part.dxf]`.

// A windowed program on Windows: no console window opens behind the GUI.
#![cfg_attr(windows, windows_subsystem = "windows")]
// Startup errors go to the terminal (and a message box) before any window exists.
#![allow(clippy::print_stderr)]

mod app;
mod draw;
mod editor;
mod fonts;
mod shape_form;
mod style;
mod view;

use std::path::PathBuf;
use std::process::ExitCode;

use eframe::egui;

fn main() -> ExitCode {
    let fonts = match fonts::system_fonts() {
        Ok(fonts) => fonts,
        Err(message) => {
            fatal(&message);
            return ExitCode::FAILURE;
        }
    };
    // A file on the command line: a .chipload project, an older .json job, or a .dxf to import.
    let file: Option<PathBuf> = std::env::args_os().nth(1).map(PathBuf::from);

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
            style::apply(&cc.egui_ctx);
            let mut app = app::ChiploadApp::default();
            app.set_settings_dir(chipload_pipeline::library::settings_dir());
            if let Some(file) = &file {
                app.open(file);
            }
            Ok(Box::new(app))
        }),
    );
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            fatal(&err.to_string());
            ExitCode::FAILURE
        }
    }
}

/// Report an error that keeps the GUI from starting: on the terminal and, because a
/// Windows GUI has no terminal, in a message box.
fn fatal(message: &str) {
    eprintln!("error: {message}");
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title("Chipload cannot start")
        .set_description(message)
        .show();
}
