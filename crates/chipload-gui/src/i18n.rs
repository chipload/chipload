// SPDX-License-Identifier: GPL-3.0-or-later
//! The app's language (docs/specs/translations.md). Texts are written `t!(ui, "id")` or
//! `t!(ui, "id", name = value)`, where the first argument is anything that knows the
//! language: a `Ui`, the egui `Context`, the app or a `Language`. Like the display units,
//! the language rides in egui's memory each frame, so every widget finds it.

pub use chipload_i18n::{Arg, Language};
use eframe::egui::{self, Id};

/// A text in the current language. Ids are always literals, so the tests can find them.
macro_rules! t {
    ($src:expr, $id:literal) => {
        $crate::i18n::text(&$src, $id, &[])
    };
    ($src:expr, $id:literal, $($name:ident = $value:expr),+ $(,)?) => {
        $crate::i18n::text(
            &$src,
            $id,
            &[$((stringify!($name), $crate::i18n::Arg::from($value))),+],
        )
    };
}
pub(crate) use t;

fn key() -> Id {
    Id::new("chipload-language")
}

/// Make `language` the language of this frame's widgets.
pub fn set_language(ctx: &egui::Context, language: Language) {
    ctx.data_mut(|d| d.insert_temp(key(), language));
}

/// Anything that knows the language texts are shown in.
pub trait Lang {
    fn lang(&self) -> Language;
}

impl Lang for Language {
    fn lang(&self) -> Language {
        *self
    }
}

impl Lang for egui::Context {
    /// English until set.
    fn lang(&self) -> Language {
        self.data(|d| d.get_temp(key())).unwrap_or_default()
    }
}

impl Lang for egui::Ui {
    fn lang(&self) -> Language {
        self.ctx().lang()
    }
}

impl<T: Lang + ?Sized> Lang for &T {
    fn lang(&self) -> Language {
        (**self).lang()
    }
}

impl<T: Lang + ?Sized> Lang for &mut T {
    fn lang(&self) -> Language {
        (**self).lang()
    }
}

/// The text `id` in the language of `src` (use `t!`).
pub fn text(src: &impl Lang, id: &str, args: &[(&str, Arg)]) -> String {
    chipload_i18n::tr(src.lang(), id, args)
}

/// The language a `language` setting means: a known code, or the system's language when it
/// is one of ours, else English.
pub fn resolve(setting: &str) -> Language {
    Language::from_tag(setting)
        .or_else(|| sys_locale::get_locale().and_then(|tag| Language::from_tag(&tag)))
        .unwrap_or_default()
}
