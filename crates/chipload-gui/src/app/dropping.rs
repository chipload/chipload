// SPDX-License-Identifier: GPL-3.0-or-later
//! Files dropped on the window (docs/specs/small-items.md §2, drawing-sources.md §3): a
//! project opens, after the unsaved-changes question; DXFs and pictures are added to the
//! open project, or start one when none is open.

use super::*;

/// What a dropped file is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Dropped {
    Project,
    Dxf,
    Picture,
}

pub(super) fn dropped_kind(path: &Path) -> Option<Dropped> {
    let extension = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match extension.as_str() {
        "dxf" => Some(Dropped::Dxf),
        "png" | "jpg" | "jpeg" | "bmp" => Some(Dropped::Picture),
        e if e == PROJECT_EXTENSION || e == "json" => Some(Dropped::Project),
        _ => None,
    }
}

/// What dropping `paths` does, for the hover hint (`open`: a project is open); Err: the
/// status line when nothing will.
pub(super) fn drop_meaning(paths: &[PathBuf], open: bool) -> Result<String, &'static str> {
    const TAKEN: &str = "Drop a .chipload project, or DXFs and pictures";
    let kinds: Option<Vec<Dropped>> = paths.iter().map(|p| dropped_kind(p)).collect();
    let Some(kinds) = kinds.filter(|k| !k.is_empty()) else {
        return Err(TAKEN);
    };
    if kinds.contains(&Dropped::Project) {
        return if kinds.len() == 1 {
            Ok("Open this project".into())
        } else {
            Err("Drop one .chipload project at a time")
        };
    }
    let dxfs = kinds.iter().filter(|&&k| k == Dropped::Dxf).count();
    let pictures = kinds.len() - dxfs;
    let count = |n: usize, one: &str, many: &str| match n {
        0 => None,
        1 => Some(format!("1 {one}")),
        n => Some(format!("{n} {many}")),
    };
    let what = [
        count(dxfs, "DXF", "DXFs"),
        count(pictures, "picture", "pictures"),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" and ");
    Ok(if open {
        format!("Add {what} to the project")
    } else {
        format!("New project with {what}")
    })
}

impl ChiploadApp {
    /// Take what was dropped: a project opens (asking first if unsaved changes would be
    /// lost); DXFs and pictures are queued and brought in one at a time. Returns whether
    /// the app takes the files.
    pub fn drop_files(&mut self, paths: &[PathBuf]) -> bool {
        if let Err(why) = drop_meaning(paths, self.edit.is_some()) {
            self.set_status(why.into(), STATUS_SECONDS);
            return false;
        }
        if let [path] = paths
            && dropped_kind(path) == Some(Dropped::Project)
        {
            if self.guard(Pending::Open(path.clone())) {
                self.open(path);
            }
            return true;
        }
        self.drops.extend(paths.iter().cloned());
        self.next_drop();
        true
    }

    /// Bring in the queued dropped files until one needs an answer (a DXF's units) or a
    /// window (a picture's): the rest wait until it is closed.
    pub(super) fn next_drop(&mut self) {
        while self.prompt.is_none() && self.image_dialog.is_none() {
            let Some(path) = self.drops.pop_front() else {
                return;
            };
            match dropped_kind(&path) {
                Some(Dropped::Dxf) if self.edit.is_none() => {
                    self.import_dxf(&path, None);
                }
                Some(Dropped::Dxf) => {
                    self.add_dxf_from(&path, None);
                }
                Some(Dropped::Picture) => {
                    self.import_image(&path);
                }
                Some(Dropped::Project) | None => {}
            }
        }
    }

    /// Files dropped this frame, and the hint while they are dragged over the window.
    pub(super) fn file_drops(&mut self, ctx: &egui::Context) {
        let (hovered, dropped) = ctx.input(|i| {
            (
                i.raw
                    .hovered_files
                    .iter()
                    .filter_map(|f| f.path.clone())
                    .collect::<Vec<PathBuf>>(),
                i.raw
                    .dropped_files
                    .iter()
                    .map(|f| f.path().to_path_buf())
                    .collect::<Vec<PathBuf>>(),
            )
        });
        if !dropped.is_empty() {
            self.drop_files(&dropped);
        }
        if ctx.input(|i| i.raw.hovered_files.is_empty()) {
            return;
        }
        let text = match drop_meaning(&hovered, self.edit.is_some()) {
            Ok(meaning) => meaning,
            // Some systems don't say what is being dragged until it is dropped.
            Err(_) if hovered.is_empty() => "Drop a .chipload project, or DXFs and pictures".into(),
            Err(why) => why.into(),
        };
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("file-drop"),
        ));
        let screen = ctx.content_rect();
        painter.rect_filled(screen, 0.0, egui::Color32::from_black_alpha(150));
        painter.text(
            screen.center(),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(22.0),
            egui::Color32::WHITE,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_project_alone_or_dxfs_and_pictures() {
        let p = |names: &[&str]| -> Vec<PathBuf> { names.iter().map(PathBuf::from).collect() };
        assert_eq!(
            drop_meaning(&p(&["a.chipload"]), true).as_deref(),
            Ok("Open this project")
        );
        assert_eq!(
            drop_meaning(&p(&["old.json"]), false).as_deref(),
            Ok("Open this project")
        );
        assert_eq!(
            drop_meaning(&p(&["Part.DXF"]), false).as_deref(),
            Ok("New project with 1 DXF")
        );
        assert_eq!(
            drop_meaning(&p(&["a.dxf", "b.dxf", "logo.PNG"]), true).as_deref(),
            Ok("Add 2 DXFs and 1 picture to the project")
        );
        assert!(drop_meaning(&p(&["a.chipload", "b.dxf"]), true).is_err());
        assert!(drop_meaning(&p(&["notes.txt"]), true).is_err());
        assert!(drop_meaning(&p(&["a.dxf", "notes.txt"]), true).is_err());
        assert!(drop_meaning(&[], true).is_err());
    }
}
