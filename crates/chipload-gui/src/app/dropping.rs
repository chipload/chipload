// SPDX-License-Identifier: GPL-3.0-or-later
//! Files dropped on the window (docs/specs/small-items.md §2): a project opens, a DXF starts a
//! new project, as File → Open and Import do, after the same unsaved-changes question.

use super::*;

/// What dropping `paths` does, for the hover hint; Err: the status line when nothing will.
pub(super) fn drop_meaning(paths: &[PathBuf]) -> Result<&'static str, &'static str> {
    const ONE: &str = "Drop one .chipload or .dxf file";
    let [path] = paths else {
        return Err(ONE);
    };
    let extension = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match extension.as_str() {
        "dxf" => Ok("New project from this DXF"),
        e if e == PROJECT_EXTENSION || e == "json" => Ok("Open this project"),
        _ => Err(ONE),
    }
}

impl ChiploadApp {
    /// Open what was dropped, asking first if unsaved changes would be lost. Returns whether
    /// it was a file the app takes.
    pub fn drop_files(&mut self, paths: &[PathBuf]) -> bool {
        if let Err(why) = drop_meaning(paths) {
            self.set_status(why.into(), STATUS_SECONDS);
            return false;
        }
        let path = paths[0].clone();
        if self.guard(Pending::Open(path.clone())) {
            self.open(&path);
        }
        true
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
        let text = match drop_meaning(&hovered) {
            Ok(meaning) => meaning,
            // Some systems don't say what is being dragged until it is dropped.
            Err(_) if hovered.is_empty() => "Drop a .chipload project or a .dxf drawing",
            Err(why) => why,
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
    fn one_project_or_dxf_at_a_time() {
        let p = |names: &[&str]| -> Vec<PathBuf> { names.iter().map(PathBuf::from).collect() };
        assert_eq!(drop_meaning(&p(&["a.chipload"])), Ok("Open this project"));
        assert_eq!(drop_meaning(&p(&["old.json"])), Ok("Open this project"));
        assert_eq!(
            drop_meaning(&p(&["Part.DXF"])),
            Ok("New project from this DXF")
        );
        assert!(drop_meaning(&p(&["a.dxf", "b.dxf"])).is_err());
        assert!(drop_meaning(&p(&["photo.png"])).is_err());
        assert!(drop_meaning(&[]).is_err());
    }
}
