// SPDX-License-Identifier: GPL-3.0-or-later
//! The app's texts in English, German, French, Spanish, Italian and Swedish
//! (docs/specs/translations.md). Texts are Fluent messages in `locales/<lang>/*.ftl`,
//! compiled in; `tr` finds one by id and fills in its arguments. English is the source:
//! a text missing in another language falls back to it.
//!
//! No I/O and no other workspace crate, so any crate may use it.

use std::sync::LazyLock;

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource, FluentValue};
use unic_langid::LanguageIdentifier;

/// A language the app is translated into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Language {
    #[default]
    En,
    De,
    Fr,
    Es,
    It,
    Sv,
}

impl Language {
    /// Every language, English first, the rest in the order of the language menu.
    pub const ALL: [Language; 6] = [
        Language::En,
        Language::De,
        Language::Fr,
        Language::Es,
        Language::It,
        Language::Sv,
    ];

    /// The ISO 639-1 code, as written in the settings: `en`, `de`, …
    pub fn code(self) -> &'static str {
        match self {
            Language::En => "en",
            Language::De => "de",
            Language::Fr => "fr",
            Language::Es => "es",
            Language::It => "it",
            Language::Sv => "sv",
        }
    }

    /// The language's name in itself, for the language menu.
    pub fn native_name(self) -> &'static str {
        match self {
            Language::En => "English",
            Language::De => "Deutsch",
            Language::Fr => "Français",
            Language::Es => "Español",
            Language::It => "Italiano",
            Language::Sv => "Svenska",
        }
    }

    /// Has a native speaker who machines read the translation (decision 8)? Until then
    /// the language menu asks for a review.
    pub fn reviewed(self) -> bool {
        matches!(self, Language::En)
    }

    /// Does the language write decimals with a comma (`6,5`)? Fields then take one
    /// (decision 6); the app still shows a point.
    pub fn decimal_comma(self) -> bool {
        self != Language::En
    }

    /// The language of a locale or language tag: `de`, `de-AT`, `sv_SE.UTF-8`, `fr-CA`.
    /// None for languages the app isn't translated into.
    pub fn from_tag(tag: &str) -> Option<Language> {
        let primary = tag
            .split(['-', '_', '.', '@'])
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        Language::ALL.into_iter().find(|l| l.code() == primary)
    }

    fn index(self) -> usize {
        self as usize
    }
}

/// An argument to a text: words, or a whole number (which also picks the plural form).
/// Lengths and other measures are formatted by the caller, in the display units, and
/// passed as text, so a number always reads as the app writes it (decision 6).
#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    Text(String),
    Count(i64),
}

impl From<&str> for Arg {
    fn from(s: &str) -> Self {
        Arg::Text(s.to_string())
    }
}

impl From<String> for Arg {
    fn from(s: String) -> Self {
        Arg::Text(s)
    }
}

impl From<&String> for Arg {
    fn from(s: &String) -> Self {
        Arg::Text(s.clone())
    }
}

macro_rules! count_from {
    ($($t:ty),*) => {$(
        impl From<$t> for Arg {
            fn from(n: $t) -> Self {
                Arg::Count(i64::try_from(n).unwrap_or(i64::MAX))
            }
        }
    )*};
}
count_from!(i32, i64, u32, u64, usize);

/// One `.ftl` file in every language, in the order of `Language::ALL`.
macro_rules! sources {
    ($($file:literal),* $(,)?) => {
        [$(
            (
                $file,
                [
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/en/", $file)),
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/de/", $file)),
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/fr/", $file)),
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/es/", $file)),
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/it/", $file)),
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/sv/", $file)),
                ],
            )
        ),*]
    };
}

/// Every file of texts, as `build.rs` found them in `locales/en/`.
pub const SOURCES: &[(&str, [&str; 6])] = &include!(concat!(env!("OUT_DIR"), "/sources.rs"));

type Bundle = FluentBundle<FluentResource>;

/// The texts of each language, parsed once. Read only after they are built: tables, not
/// state.
static BUNDLES: LazyLock<Vec<Bundle>> =
    LazyLock::new(|| Language::ALL.into_iter().map(bundle).collect());

fn bundle(lang: Language) -> Bundle {
    let id: LanguageIdentifier = lang.code().parse().unwrap_or_default();
    let mut bundle = FluentBundle::new_concurrent(vec![id]);
    // No Unicode isolation marks around arguments: egui would draw them as boxes.
    bundle.set_use_isolating(false);
    for (_, texts) in SOURCES {
        // A file that doesn't parse loses only its broken entries; the tests catch it.
        let resource = match FluentResource::try_new(texts[lang.index()].to_string()) {
            Ok(r) => r,
            Err((r, _)) => r,
        };
        // A text given twice keeps the first; the tests catch that too.
        let _ = bundle.add_resource(resource);
    }
    bundle
}

/// The text `id` in `lang`, with its arguments filled in. Falls back to English when the
/// language lacks the text or can't format it, and to the id itself when English lacks
/// it too (a bug the tests catch; never a panic).
pub fn tr(lang: Language, id: &str, args: &[(&str, Arg)]) -> String {
    let fluent_args = fluent_args(args);
    format(lang, id, &fluent_args)
        .or_else(|| format(Language::En, id, &fluent_args))
        .unwrap_or_else(|| id.to_string())
}

fn fluent_args(args: &[(&str, Arg)]) -> FluentArgs<'static> {
    let mut out = FluentArgs::new();
    for (name, arg) in args {
        let value: FluentValue<'static> = match arg {
            Arg::Text(s) => FluentValue::from(s.clone()),
            Arg::Count(n) => FluentValue::from(*n),
        };
        out.set(name.to_string(), value);
    }
    out
}

fn format(lang: Language, id: &str, args: &FluentArgs) -> Option<String> {
    let bundle = BUNDLES.get(lang.index())?;
    let pattern = bundle.get_message(id)?.value()?;
    let mut errors = Vec::new();
    let text = bundle.format_pattern(pattern, Some(args), &mut errors);
    errors.is_empty().then(|| text.into_owned())
}

pub mod check;
