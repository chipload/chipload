// SPDX-License-Identifier: GPL-3.0-or-later
//! The computer's fonts for texts in the window (docs/specs/system-fonts.md §6): looked for
//! in the background the first time they are needed, so the window never waits on it; a
//! font's sample loaded into egui only when it is shown.

use std::sync::Arc;

use chipload_pipeline::fonts::{ComputerFonts, FontSource};
use eframe::egui;

use super::ChiploadApp;

/// The ui data flag the font list sets while it is open: look for the fonts now.
pub(super) const FONTS_WANTED: &str = "computer-fonts-wanted";

/// The ui data the font list leaves when a font is picked from it: the font's name, kept
/// at the top of the recently used fonts (docs/specs/font-list.md).
pub(super) const FONT_PICKED: &str = "font-picked";

/// The computer's fonts as the window knows them.
#[derive(Debug, Clone)]
pub struct TextFonts {
    pub(super) finder: Arc<ComputerFonts>,
    started: bool,
    /// Their names, once looked for.
    pub(super) names: Option<Arc<Vec<String>>>,
    /// The fonts last picked, most recent first (the settings' `recent_fonts`).
    pub(super) recent: Vec<String>,
}

impl TextFonts {
    pub(super) fn new(settings_dir: Option<&std::path::Path>) -> Self {
        let folder = settings_dir.map(chipload_pipeline::library::fonts_path);
        TextFonts {
            finder: Arc::new(ComputerFonts::new(folder)),
            started: false,
            names: None,
            recent: Vec::new(),
        }
    }

    /// Whether a text in `font` can be changed here: Some(true) or Some(false), None while
    /// the computer's fonts are still looked for.
    pub(super) fn has(&self, font: &str) -> Option<bool> {
        if chipload_text::builtin(font).is_some() {
            return Some(true);
        }
        self.names
            .as_ref()
            .map(|names| names.binary_search_by(|n| n.as_str().cmp(font)).is_ok())
    }
}

impl Default for TextFonts {
    fn default() -> Self {
        TextFonts::new(None)
    }
}

/// The egui family a computer font's sample is drawn in.
pub(super) fn sample_family(name: &str) -> egui::FontFamily {
    egui::FontFamily::Name(format!("computer:{name}").into())
}

/// Load computer font `name` into egui for its sample, once; it shows from the next frame.
pub(super) fn load_sample(ctx: &egui::Context, finder: &ComputerFonts, name: &str) {
    let family = sample_family(name);
    if ctx.fonts(|f| f.families()).contains(&family) {
        return;
    }
    let Some(font) = finder.find(name) else {
        return;
    };
    let mut data = egui::FontData::from_owned(font.data);
    data.index = font.index;
    ctx.add_font(egui::epaint::text::FontInsert::new(
        &format!("computer:{name}"),
        data,
        vec![egui::epaint::text::InsertFontFamily {
            family,
            priority: egui::epaint::text::FontPriority::Highest,
        }],
    ));
}

impl ChiploadApp {
    /// Each frame: start looking for the computer's fonts when a text needs one or the font
    /// list is open (decision 6), and take their names once found.
    pub(super) fn poll_text_fonts(&mut self, ctx: &egui::Context) {
        let picked = ctx.data_mut(|d| d.remove_temp::<String>(egui::Id::new(FONT_PICKED)));
        if let Some(font) = picked {
            self.remember_font(&font);
        }
        if self.text_fonts.names.is_none() && self.text_fonts.finder.scanned() {
            self.text_fonts.names = Some(Arc::new(self.text_fonts.finder.names()));
        }
        if self.text_fonts.started {
            return;
        }
        let wanted = ctx.data(|d| d.get_temp::<bool>(egui::Id::new(FONTS_WANTED))) == Some(true)
            || self.edit.as_ref().is_some_and(|e| {
                e.job
                    .drawing
                    .texts
                    .iter()
                    .any(|t| chipload_text::builtin(&t.font).is_none())
            });
        if wanted {
            self.start_font_search(ctx);
        }
    }

    /// Put `font` at the top of the recently used fonts, here and in the settings.
    pub(super) fn remember_font(&mut self, font: &str) {
        let recent = &mut self.text_fonts.recent;
        recent.retain(|f| f != font);
        recent.insert(0, font.to_string());
        recent.truncate(chipload_job::library::RECENT_FONTS_MAX);
        self.change_settings(|settings| settings.add_recent_font(font));
    }

    /// Look for the computer's fonts in the background.
    pub(super) fn start_font_search(&mut self, ctx: &egui::Context) {
        if self.text_fonts.started {
            return;
        }
        self.text_fonts.started = true;
        let finder = self.text_fonts.finder.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            finder.names();
            ctx.request_repaint();
        });
    }

    /// Wait for the computer's fonts (tests and the command line's way, never the
    /// window's).
    #[cfg(test)]
    pub(super) fn wait_for_fonts(&mut self) {
        self.text_fonts.names = Some(Arc::new(self.text_fonts.finder.names()));
        self.text_fonts.started = true;
    }

    /// The font finder, for making letters.
    pub(super) fn font_source(&self) -> Arc<ComputerFonts> {
        self.text_fonts.finder.clone()
    }
}
