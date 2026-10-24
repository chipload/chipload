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
mod i18n;
mod icons;
mod shape_form;
mod style;
mod units;
mod view;

use std::path::PathBuf;
use std::process::ExitCode;

use eframe::egui;

fn main() -> ExitCode {
    let fonts = match fonts::system_fonts() {
        Ok(mut fonts) => {
            fonts::add_text_fonts(&mut fonts);
            fonts
        }
        Err(message) => {
            fatal(&message);
            return ExitCode::FAILURE;
        }
    };
    // A file on the command line: a .chipload project, an older .json job, or a .dxf to import.
    let file: Option<PathBuf> = std::env::args_os().nth(1).map(PathBuf::from);

    #[allow(unused_mut)]
    let mut size = [1200.0, 800.0];
    // A larger window for the README's picture (packaging/docs/screenshots.sh).
    #[cfg(feature = "screenshots")]
    if let Some((w, h)) = std::env::var("CHIPLOAD_SCREENSHOT_SIZE")
        .ok()
        .and_then(|s| s.split_once('x').map(|(w, h)| (w.parse(), h.parse())))
        .and_then(|(w, h)| Some((w.ok()?, h.ok()?)))
    {
        size = [w, h];
    }
    let options = eframe::NativeOptions {
        viewport: window_icon(
            egui::ViewportBuilder::default()
                .with_title("Chipload")
                .with_inner_size(size),
        ),
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
            // The user guide's screenshots (packaging/docs/screenshots.sh).
            #[cfg(feature = "screenshots")]
            if let Ok(steps) = std::env::var("CHIPLOAD_SCREENSHOT_SETUP") {
                app.screenshot_setup(&cc.egui_ctx, &steps);
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

/// The window's icon: Chipload's mark (packaging/brand/). A picture that can't be read leaves
/// the system's default, which is no reason to stop.
fn window_icon(builder: egui::ViewportBuilder) -> egui::ViewportBuilder {
    match eframe::icon_data::from_png_bytes(include_bytes!("../assets/chipload-256.png")) {
        Ok(icon) => builder.with_icon(icon),
        Err(_) => builder,
    }
}
