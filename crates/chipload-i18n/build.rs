// SPDX-License-Identifier: GPL-3.0-or-later
//! Lists the files of texts (`locales/en/*.ftl`) for `SOURCES`, so a new file needs no
//! change to the code. Every language folder must hold the same files; a missing one is a
//! build error from `include_str!`.

use std::fmt::Write as _;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?).join("locales");
    println!("cargo::rerun-if-changed=locales");
    let mut files: Vec<String> = std::fs::read_dir(dir.join("en"))?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".ftl"))
        .collect();
    files.sort();
    let mut list = String::from("sources![");
    for file in &files {
        write!(list, "{file:?}, ")?;
    }
    list.push(']');
    let out = PathBuf::from(std::env::var("OUT_DIR")?).join("sources.rs");
    std::fs::write(out, list)?;
    Ok(())
}
