// SPDX-License-Identifier: GPL-3.0-or-later
//! The window's typeface is Inter, built in (docs/specs/look-and-feel.md §9, ADR 0008); the
//! system's fonts fill in what it lacks, loaded at startup. egui's own bundled fonts stay
//! out. The built-in fonts for text (chipload-text) are added only to draw their own names
//! in the font list (docs/specs/more-fonts.md §5).

use std::path::{Path, PathBuf};
#[cfg(not(windows))]
use std::process::Command;
use std::sync::Arc;

use eframe::egui::{FontData, FontDefinitions, FontFamily};

/// Well-known locations, tried in order after fontconfig.
const SANS_CANDIDATES: &[&str] = &[
    // Linux
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    "/usr/share/fonts/TTF/DejaVuSans.ttf",
    "/usr/share/fonts/dejavu/DejaVuSans.ttf",
    "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
    "/usr/share/fonts/noto/NotoSans-Regular.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
    // Windows
    "C:\\Windows\\Fonts\\segoeui.ttf",
    "C:\\Windows\\Fonts\\arial.ttf",
    // macOS
    "/System/Library/Fonts/Supplemental/Arial.ttf",
    "/Library/Fonts/Arial.ttf",
];

const MONO_CANDIDATES: &[&str] = &[
    "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
    "/usr/share/fonts/dejavu/DejaVuSansMono.ttf",
    "/usr/share/fonts/truetype/noto/NotoSansMono-Regular.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
    "C:\\Windows\\Fonts\\consola.ttf",
    "C:\\Windows\\Fonts\\cour.ttf",
    "/System/Library/Fonts/Supplemental/Courier New.ttf",
    "/Library/Fonts/Courier New.ttf",
];

/// Fonts with the symbols the interface uses (✔ ⚠ ✖ ▸ ▾ …), as a last fallback: Segoe UI
/// and Arial lack some of them.
const SYMBOL_CANDIDATES: &[&str] = &[
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    "/usr/share/fonts/TTF/DejaVuSans.ttf",
    "/usr/share/fonts/dejavu/DejaVuSans.ttf",
    "C:\\Windows\\Fonts\\seguisym.ttf",
    "/System/Library/Fonts/Apple Symbols.ttf",
];

/// Ask fontconfig (Linux and other Unix desktops) for a font file.
#[cfg(not(windows))]
fn fontconfig(pattern: &str) -> Option<PathBuf> {
    let output = Command::new("fc-match")
        .args(["-f", "%{file}", pattern])
        .output()
        .ok()?;
    let path = PathBuf::from(String::from_utf8(output.stdout).ok()?.trim());
    is_loadable(&path).then_some(path)
}

/// Windows has no fontconfig: its fonts are the candidates' files. An `fc-match` that Git
/// for Windows, Inkscape or GIMP put on the PATH is a console program, and each call
/// flashed a terminal window at startup (the user, 2026-11-03: "three terminal windows
/// that briefly flash and disappear").
#[cfg(windows)]
fn fontconfig(_pattern: &str) -> Option<PathBuf> {
    None
}

/// TrueType/OpenType files only: collections (.ttc) need a face index we can't know.
fn is_loadable(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    path.is_file() && matches!(ext.as_deref(), Some("ttf" | "otf"))
}

fn find(pattern: &str, candidates: &[&str]) -> Option<PathBuf> {
    fontconfig(pattern).or_else(|| {
        candidates
            .iter()
            .map(PathBuf::from)
            .find(|p| is_loadable(p))
    })
}

/// The egui family that draws in built-in text font `name`.
pub fn text_font_family(name: &str) -> FontFamily {
    FontFamily::Name(format!("text:{name}").into())
}

/// Add every built-in text font as a family of its own (falling back to Inter for
/// anything it lacks), for the font list.
pub fn add_text_fonts(fonts: &mut FontDefinitions) {
    for font in chipload_text::FONTS {
        let key = format!("text:{}", font.name);
        fonts
            .font_data
            .insert(key.clone(), Arc::new(FontData::from_static(font.data())));
        fonts
            .families
            .insert(text_font_family(font.name), vec![key, "inter".into()]);
    }
}

/// Inter 4.1, Regular and SemiBold, unchanged: the window's typeface on every system
/// (docs/specs/look-and-feel.md §9; SIL OFL 1.1, fonts/LICENSE-OFL.txt, ADR 0008).
const INTER_REGULAR: &[u8] = include_bytes!("../fonts/Inter-Regular.ttf");
const INTER_SEMIBOLD: &[u8] = include_bytes!("../fonts/Inter-SemiBold.ttf");

/// The family of bold text: Inter SemiBold (`style::bold`).
pub fn bold_family() -> FontFamily {
    FontFamily::Name("bold".into())
}

/// The window's fonts: Inter first, then the system's sans, monospace and symbol fonts for
/// what Inter lacks (✔ ⚠ ✖, scripts beyond Cyrillic and Greek) and for monospace text. A
/// system font that is missing or unreadable is left out: Inter alone is enough to start.
pub fn ui_fonts() -> FontDefinitions {
    let read = |path: &Path| std::fs::read(path).ok();
    let sans = find("sans-serif", SANS_CANDIDATES);
    let mono = find("monospace", MONO_CANDIDATES).filter(|p| Some(p) != sans.as_ref());
    let symbols = find("DejaVu Sans", SYMBOL_CANDIDATES)
        .filter(|p| Some(p) != sans.as_ref() && Some(p) != mono.as_ref());
    let mut fonts = FontDefinitions::empty();
    let mut add = |key: &str, data: Option<Vec<u8>>| {
        data.map(|data| {
            fonts
                .font_data
                .insert(key.into(), Arc::new(FontData::from_owned(data)));
            key.to_string()
        })
    };
    let sans = add("sans", sans.as_deref().and_then(read));
    let mono = add("mono", mono.as_deref().and_then(read));
    let symbols = add("symbols", symbols.as_deref().and_then(read));
    fonts.font_data.insert(
        "inter".into(),
        Arc::new(FontData::from_static(INTER_REGULAR)),
    );
    fonts.font_data.insert(
        "inter-semibold".into(),
        Arc::new(FontData::from_static(INTER_SEMIBOLD)),
    );
    let fallbacks: Vec<String> = [sans, symbols].into_iter().flatten().collect();
    let chain = |first: &[Option<String>]| -> Vec<String> {
        let mut names: Vec<String> = first.iter().flatten().cloned().collect();
        for name in &fallbacks {
            if !names.contains(name) {
                names.push(name.clone());
            }
        }
        names
    };
    let inter = Some("inter".to_string());
    let semibold = Some("inter-semibold".to_string());
    fonts.families.insert(
        FontFamily::Proportional,
        chain(std::slice::from_ref(&inter)),
    );
    fonts
        .families
        .insert(FontFamily::Monospace, chain(&[mono, inter.clone()]));
    fonts
        .families
        .insert(bold_family(), chain(&[semibold, inter]));
    fonts
}
