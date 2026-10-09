// SPDX-License-Identifier: GPL-3.0-or-later
//! Fonts come from the system, not the binary: egui's bundled fonts carry font licences
//! outside our allowed list (decision 2026-10-09), so we load installed ones at startup.

use std::path::{Path, PathBuf};
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

/// Ask fontconfig (Linux and other Unix desktops) for a font file.
fn fontconfig(pattern: &str) -> Option<PathBuf> {
    let output = Command::new("fc-match")
        .args(["-f", "%{file}", pattern])
        .output()
        .ok()?;
    let path = PathBuf::from(String::from_utf8(output.stdout).ok()?.trim());
    is_loadable(&path).then_some(path)
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

/// Font definitions using installed fonts, or a message saying what to install.
pub fn system_fonts() -> Result<FontDefinitions, String> {
    let sans = find("sans-serif", SANS_CANDIDATES).ok_or(
        "No usable system font found. Install a TrueType font such as DejaVu Sans \
         (Debian/Ubuntu: sudo apt install fonts-dejavu-core).",
    )?;
    // A missing monospace font is not fatal: reports then use the sans font.
    let mono = find("monospace", MONO_CANDIDATES).unwrap_or_else(|| sans.clone());
    let read = |path: &Path| {
        std::fs::read(path).map_err(|e| format!("Cannot read font {}: {e}", path.display()))
    };
    let mut fonts = FontDefinitions::empty();
    fonts
        .font_data
        .insert("sans".into(), Arc::new(FontData::from_owned(read(&sans)?)));
    fonts
        .font_data
        .insert("mono".into(), Arc::new(FontData::from_owned(read(&mono)?)));
    for (family, first) in [
        (FontFamily::Proportional, "sans"),
        (FontFamily::Monospace, "mono"),
    ] {
        let fallback = if first == "sans" { "mono" } else { "sans" };
        fonts
            .families
            .insert(family, vec![first.into(), fallback.into()]);
    }
    Ok(fonts)
}
