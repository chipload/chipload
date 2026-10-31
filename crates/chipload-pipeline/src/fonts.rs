// SPDX-License-Identifier: GPL-3.0-or-later
//! Fonts for texts besides the built-in ones: the computer's installed fonts and the
//! settings folder's `fonts` folder (docs/specs/system-fonts.md §2, §4). A project names a
//! font; its letters are stored as shapes, so a font is only needed to make them again.
//! The browser build has the built-in fonts only (`BuiltinOnly`).

/// A font file read for making letters: its bytes and the face in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedFont {
    pub data: Vec<u8>,
    pub index: u32,
}

/// Where fonts that aren't built in are found.
pub trait FontSource {
    /// The font a project names `name`, if it is on this computer. Never asked for a
    /// built-in font's name.
    fn find(&self, name: &str) -> Option<LoadedFont>;
}

/// No fonts but the built-in ones.
#[derive(Debug, Clone, Copy, Default)]
pub struct BuiltinOnly;

impl FontSource for BuiltinOnly {
    fn find(&self, _name: &str) -> Option<LoadedFont> {
        None
    }
}

/// Whether a text in font `name` can be made again here.
pub fn available(name: &str, fonts: &dyn FontSource) -> bool {
    chipload_text::builtin(name).is_some() || fonts.find(name).is_some()
}

#[cfg(not(target_arch = "wasm32"))]
pub use computer::ComputerFonts;

#[cfg(not(target_arch = "wasm32"))]
mod computer {
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use std::sync::OnceLock;

    use super::{FontSource, LoadedFont};

    /// Where a font's face is.
    #[derive(Debug, Clone)]
    struct FaceAt {
        path: PathBuf,
        index: u32,
    }

    /// The computer's fonts and a folder of fonts, by the name a project gives them. They
    /// are looked for the first time they are needed, once (decision 6).
    #[derive(Debug)]
    pub struct ComputerFonts {
        folder: Option<PathBuf>,
        system: bool,
        faces: OnceLock<BTreeMap<String, FaceAt>>,
    }

    impl ComputerFonts {
        /// The installed fonts and those in `folder` (the settings folder's `fonts`), which
        /// win when both have a name.
        pub fn new(folder: Option<PathBuf>) -> Self {
            ComputerFonts {
                folder,
                system: true,
                faces: OnceLock::new(),
            }
        }

        /// Only the fonts in `folder`, not the installed ones: for tests, which mustn't
        /// depend on the computer.
        pub fn folder_only(folder: PathBuf) -> Self {
            ComputerFonts {
                folder: Some(folder),
                system: false,
                faces: OnceLock::new(),
            }
        }

        /// The fonts' names, sorted; a built-in font's name is never among them.
        pub fn names(&self) -> Vec<String> {
            self.faces().keys().cloned().collect()
        }

        /// True once the fonts have been looked for, so asking is quick.
        pub fn scanned(&self) -> bool {
            self.faces.get().is_some()
        }

        fn faces(&self) -> &BTreeMap<String, FaceAt> {
            self.faces
                .get_or_init(|| scan(self.folder.as_ref(), self.system))
        }
    }

    impl FontSource for ComputerFonts {
        fn find(&self, name: &str) -> Option<LoadedFont> {
            let at = self.faces().get(name)?;
            let data = std::fs::read(&at.path).ok()?;
            Some(LoadedFont {
                data,
                index: at.index,
            })
        }
    }

    /// Every face letters can be made in, by name: the folder's first, then the system's;
    /// the first face of a name wins, and built-in names and hidden system faces (".SF
    /// NS") are left out.
    fn scan(folder: Option<&PathBuf>, system: bool) -> BTreeMap<String, FaceAt> {
        let mut db = fontdb::Database::new();
        if let Some(folder) = folder {
            db.load_fonts_dir(folder);
        }
        if system {
            db.load_system_fonts();
        }
        let mut faces = BTreeMap::new();
        let mut read: Option<(PathBuf, Vec<u8>)> = None;
        for face in db.faces() {
            let fontdb::Source::File(path) = &face.source else {
                continue;
            };
            if read.as_ref().is_none_or(|(p, _)| p != path) {
                read = std::fs::read(path).ok().map(|data| (path.clone(), data));
            }
            let Some((_, data)) = &read else {
                continue;
            };
            let Some(name) = chipload_text::face_name(data, face.index) else {
                continue;
            };
            if name.starts_with('.') || chipload_text::builtin(&name).is_some() {
                continue;
            }
            faces.entry(name).or_insert_with(|| FaceAt {
                path: path.clone(),
                index: face.index,
            });
        }
        faces
    }
}
