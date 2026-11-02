// SPDX-License-Identifier: GPL-3.0-or-later
//! The shape library's tab in the right panel (docs/specs/shape-library.md §5.1, §5.2):
//! its entries in groups, each a picture and a name; dragged onto the canvas to insert,
//! selected shapes dragged onto it to save. The rules (names, groups, saving, inserting)
//! are the pipeline's (`chipload_pipeline::shape_library`); this lists and shows them.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use chipload_core::Point;
use chipload_job::shape_library::ShapeEntry;
use chipload_pipeline::shape_library::{
    InsertSize, Item, Listing, add_group, entry_box, entry_from, find, insert, list, move_to,
    preview, read, remove, remove_group, rename, rename_group, save, shapes_dir,
};
use egui::{Align2, Color32, Pos2, Rect, Sense, Stroke, vec2};

use super::modify::ModifyTool;
use super::*;

/// What the right panel shows (decision 9: tabs).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RightTab {
    #[default]
    Properties,
    Shapes,
}

/// An entry being dragged out of the tab, by its full name (`Group/Name`).
#[derive(Debug, Clone, PartialEq)]
pub(super) struct DraggedEntry(pub String);

/// An entry read for its picture and for inserting.
#[derive(Debug)]
pub(super) struct Loaded {
    pub entry: ShapeEntry,
    /// Its shapes as lines, in its own mm, and their box.
    pub lines: Vec<Vec<(f64, f64)>>,
    pub bounds: ((f64, f64), (f64, f64)),
}

/// What the line at the top of the tab asks for.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Asking {
    /// A name for the selection being saved, in a group; `replace`: the name is taken
    /// and the user was asked whether to replace it.
    Save {
        group: Option<String>,
        name: String,
        entry: Arc<ShapeEntry>,
        replace: bool,
    },
    /// A new name for an entry or a group.
    Rename {
        from: String,
        group: bool,
        name: String,
    },
    /// Whether to delete an entry or a group.
    Delete { name: String, group: bool },
    /// A new group's name.
    NewGroup { name: String },
}

/// An item read, or why it can't be.
type Reading = Result<Arc<Loaded>, String>;

/// What saving gave.
enum Saved {
    Done,
    /// The name is taken: the user is asked whether to replace it.
    Taken,
    Failed,
}

/// The tab's state.
#[derive(Debug, Default)]
pub struct ShapeLibraryView {
    listing: Listing,
    listed: Option<Instant>,
    error: Option<String>,
    /// Entries read, by file, with the time the file had then.
    cache: BTreeMap<PathBuf, (Option<SystemTime>, Reading)>,
    search: String,
    pub(super) asking: Option<Asking>,
    /// The name field wants the keyboard (once, when it opens).
    focus_name: bool,
    /// Groups folded shut.
    folded: BTreeSet<String>,
    /// Where each group's entries are, last frame: a drop there saves into it.
    group_rects: Vec<(Option<String>, Rect)>,
    /// Where selected shapes may be dropped to save them, last frame: the tab's body, or
    /// its header when Properties is shown.
    pub(super) drop_rect: Option<Rect>,
    /// The group saved into last.
    last_group: Option<String>,
    /// The middle of the canvas, in drawing mm, last frame: a double-click inserts there.
    pub(super) view_middle: Option<Point>,
    /// What Properties would show, last frame: a change brings it back.
    seen: Option<(Focus, Option<ModifyTool>, bool)>,
}

/// How often the folder is read again while the tab is shown: files copied in by hand
/// show up without a button.
const RELIST_EVERY: Duration = Duration::from_secs(2);
/// An entry's cell: its picture and its name under it.
const CELL_WIDTH: f32 = 92.0;
const PICTURE: f32 = 76.0;

impl ChiploadApp {
    fn shapes_folder(&self) -> Option<PathBuf> {
        self.settings_dir.as_deref().map(shapes_dir)
    }

    /// Read the folder again when it is time.
    fn relist(&mut self, now: bool) {
        let due = self
            .shape_library
            .listed
            .is_none_or(|t| t.elapsed() >= RELIST_EVERY);
        if !(now || due) {
            return;
        }
        let Some(dir) = self.shapes_folder() else {
            return;
        };
        let lib = &mut self.shape_library;
        lib.listed = Some(Instant::now());
        match list(&dir) {
            Ok(listing) => {
                lib.listing = listing;
                lib.error = None;
            }
            Err(err) => lib.error = Some(err.message().render(self.language, self.units.into())),
        }
    }

    /// An item read (cached while its file doesn't change).
    pub(super) fn loaded(&mut self, item: &Item) -> Result<Arc<Loaded>, String> {
        let modified = std::fs::metadata(&item.path)
            .and_then(|m| m.modified())
            .ok();
        if let Some((when, loaded)) = self.shape_library.cache.get(&item.path)
            && *when == modified
        {
            return loaded.clone();
        }
        let loaded = read(item)
            .map_err(|e| e.message().render(self.language, self.units.into()))
            .and_then(|entry| {
                let bounds = entry_box(&entry).ok_or_else(|| "—".to_string())?;
                Ok(Arc::new(Loaded {
                    lines: preview(&entry),
                    entry,
                    bounds,
                }))
            });
        self.shape_library
            .cache
            .insert(item.path.clone(), (modified, loaded.clone()));
        loaded
    }

    /// The Properties | Shapes tabs at the top of the right panel. The Shapes tab takes a
    /// drop of selected shapes while Properties is shown.
    pub(super) fn right_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let names = [
                (RightTab::Properties, t!(ui, "app-properties")),
                (RightTab::Shapes, t!(ui, "shape-library-tab")),
            ];
            for (tab, name) in names {
                let text = RichText::new(name.to_uppercase()).small();
                let tab_response = ui.selectable_label(self.right_tab == tab, text);
                if tab_response.clicked() {
                    self.right_tab = tab;
                }
                if tab == RightTab::Shapes && self.right_tab != RightTab::Shapes {
                    self.shape_library.drop_rect = Some(tab_response.rect);
                    // An entry dragged over the tab's name opens it.
                    if tab_response.contains_pointer()
                        && egui::DragAndDrop::has_payload_of_type::<DraggedEntry>(ui.ctx())
                    {
                        self.right_tab = RightTab::Shapes;
                    }
                }
            }
        });
        ui.separator();
    }

    /// Back to Properties when the user asks for something shown there (decision 10): an
    /// item picked in the Browser, a tool with values, a text being placed. Picking shapes
    /// on the canvas doesn't switch. Compared with what it was last frame.
    pub(super) fn follow_properties(&mut self) {
        let now = (self.focus, self.modify, self.placing_text);
        let Some((focus, modify, placing)) = self.shape_library.seen.replace(now) else {
            return;
        };
        let asked = (self.focus != focus && self.focus != Focus::Selection)
            || (self.modify != modify && self.modify.is_some())
            || (self.placing_text && !placing);
        if asked {
            self.right_tab = RightTab::Properties;
        }
    }

    /// The Shapes tab.
    pub(super) fn shapes_tab(&mut self, ui: &mut egui::Ui) {
        let Some(dir) = self.shapes_folder() else {
            ui.weak(t!(ui, "shape-library-no-folder"));
            return;
        };
        self.relist(false);
        self.shape_library.drop_rect = Some(ui.available_rect_before_wrap());
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.shape_library.search)
                    .hint_text(t!(ui, "shape-library-search"))
                    .desired_width(ui.available_width() - 70.0),
            );
            if ui
                .button(t!(ui, "shape-library-folder"))
                .on_hover_text(dir.display().to_string())
                .clicked()
            {
                let _ = std::fs::create_dir_all(&dir);
                show_folder(&dir);
            }
        });
        ui.horizontal_wrapped(|ui| {
            let selected = !self.framed_shape_ids().is_empty();
            if ui
                .add_enabled(
                    selected && self.edit.is_some(),
                    egui::Button::new(t!(ui, "shape-library-save-selection")),
                )
                .on_hover_text(t!(ui, "shape-library-save-selection-hover"))
                .clicked()
            {
                let group = self.shape_library.last_group.clone();
                self.ask_library_save(group);
            }
            if ui.button(t!(ui, "shape-library-new-group")).clicked() {
                self.shape_library.asking = Some(Asking::NewGroup {
                    name: String::new(),
                });
                self.shape_library.focus_name = true;
            }
        });
        if let Some(error) = self.shape_library.error.clone() {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        self.asking_line(ui, &dir);
        ui.add_space(4.0);
        self.entry_groups(ui, &dir);
    }

    /// The line that asks: a name, or whether to delete.
    fn asking_line(&mut self, ui: &mut egui::Ui, dir: &std::path::Path) {
        let Some(asking) = self.shape_library.asking.clone() else {
            return;
        };
        let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
        let mut done = escape;
        egui::Frame::group(ui.style()).show(ui, |ui| match asking {
            Asking::Save {
                group,
                mut name,
                entry,
                replace,
            } => {
                let groups = self.shape_library.listing.groups.clone();
                let mut group = group;
                ui.label(t!(ui, "shape-library-save-as"));
                let (enter, changed) = self.name_field(ui, &mut name);
                let before = group.clone();
                egui::ComboBox::from_id_salt("shape-library-save-group")
                    .selected_text(
                        group
                            .clone()
                            .unwrap_or_else(|| t!(ui, "shape-library-no-group")),
                    )
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut group, None, t!(ui, "shape-library-no-group"));
                        for g in groups {
                            ui.selectable_value(&mut group, Some(g.clone()), g);
                        }
                    });
                // Asked to replace only for the name and group it was asked for.
                let mut replace = replace && !changed && group == before;
                ui.horizontal_wrapped(|ui| {
                    if replace {
                        ui.colored_label(
                            ui.visuals().warn_fg_color,
                            t!(ui, "shape-library-exists", name = name.clone()),
                        );
                    }
                    let label = if replace {
                        t!(ui, "shape-library-replace")
                    } else {
                        t!(ui, "shape-library-save")
                    };
                    if ui.button(label).clicked() || enter {
                        match self.save_entry(dir, group.clone(), &name, &entry, replace) {
                            Saved::Done => done = true,
                            Saved::Taken => replace = true,
                            Saved::Failed => {}
                        }
                    }
                    if ui.button(t!(ui, "shape-library-cancel")).clicked() {
                        done = true;
                    }
                });
                if !done {
                    self.shape_library.asking = Some(Asking::Save {
                        group,
                        name,
                        entry,
                        replace,
                    });
                }
            }
            Asking::Rename {
                from,
                group,
                mut name,
            } => {
                ui.label(t!(ui, "shape-library-rename", name = from.clone()));
                let (enter, _) = self.name_field(ui, &mut name);
                ui.horizontal(|ui| {
                    if ui.button(t!(ui, "shape-library-rename-button")).clicked() || enter {
                        let result = if group {
                            rename_group(dir, &from, &name)
                        } else {
                            let at = match from.rsplit_once('/') {
                                Some((g, _)) => format!("{g}/{name}"),
                                None => name.clone(),
                            };
                            rename(dir, &from, &at)
                        };
                        done = self.library_done(result.map(|_| ()));
                    }
                    if ui.button(t!(ui, "shape-library-cancel")).clicked() {
                        done = true;
                    }
                });
                if !done {
                    self.shape_library.asking = Some(Asking::Rename { from, group, name });
                }
            }
            Asking::Delete { name, group } => {
                let question = if group {
                    t!(
                        ui,
                        "shape-library-delete-group-question",
                        name = name.clone()
                    )
                } else {
                    t!(ui, "shape-library-delete-question", name = name.clone())
                };
                ui.label(question);
                ui.horizontal(|ui| {
                    if ui.button(t!(ui, "shape-library-delete")).clicked() {
                        let result = if group {
                            remove_group(dir, &name)
                        } else {
                            remove(dir, &name)
                        };
                        done = self.library_done(result.map(|_| ()));
                    }
                    if ui.button(t!(ui, "shape-library-cancel")).clicked() {
                        done = true;
                    }
                });
            }
            Asking::NewGroup { mut name } => {
                ui.label(t!(ui, "shape-library-new-group-name"));
                let (enter, _) = self.name_field(ui, &mut name);
                ui.horizontal(|ui| {
                    if ui.button(t!(ui, "shape-library-add-group")).clicked() || enter {
                        done = self.library_done(add_group(dir, &name).map(|_| ()));
                    }
                    if ui.button(t!(ui, "shape-library-cancel")).clicked() {
                        done = true;
                    }
                });
                if !done {
                    self.shape_library.asking = Some(Asking::NewGroup { name });
                }
            }
        });
        if done {
            self.shape_library.asking = None;
        }
    }

    /// A name field: Enter pressed in it, and whether the name changed.
    fn name_field(&mut self, ui: &mut egui::Ui, name: &mut String) -> (bool, bool) {
        let response = ui.add(egui::TextEdit::singleline(name).desired_width(f32::INFINITY));
        if std::mem::take(&mut self.shape_library.focus_name) {
            response.request_focus();
        }
        let enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        (enter, response.changed())
    }

    /// A change of the library done: the folder read again; an error shown. True when the
    /// question is answered.
    fn library_done(&mut self, result: Result<(), PipelineError>) -> bool {
        match result {
            Ok(()) => {
                self.relist(true);
                true
            }
            Err(err) => {
                let head = t!(self, "shape-library-failed");
                self.error_report(Report::failed(head, err.message()));
                false
            }
        }
    }

    /// Save `entry` as `name` in `group`; a name that is taken is replaced only with
    /// `replace`.
    fn save_entry(
        &mut self,
        dir: &std::path::Path,
        group: Option<String>,
        name: &str,
        entry: &ShapeEntry,
        replace: bool,
    ) -> Saved {
        let full = match &group {
            Some(g) => format!("{g}/{}", name.trim()),
            None => name.trim().to_string(),
        };
        if !replace && find(dir, &full).is_ok() {
            return Saved::Taken;
        }
        if !self.library_done(save(dir, &full, entry, replace).map(|_| ())) {
            return Saved::Failed;
        }
        self.shape_library.last_group = group;
        let text = t!(self, "shape-library-saved", name = full);
        self.set_status(text, STATUS_SECONDS);
        Saved::Done
    }

    /// Ask for a name for the selection (Save selection…, a drop, the canvas menu): the tab
    /// opens with the name field, a text's words or the layer's name suggested.
    pub(super) fn ask_library_save(&mut self, group: Option<String>) {
        let ids = self.framed_shape_ids();
        let Some(edit) = self.edit.as_ref() else {
            return;
        };
        match entry_from(&edit.settings, &ids, &[]) {
            Ok(entry) => {
                let name = suggested_name(&entry);
                self.right_tab = RightTab::Shapes;
                self.shape_library.asking = Some(Asking::Save {
                    group,
                    name,
                    entry: Arc::new(entry),
                    replace: false,
                });
                self.shape_library.focus_name = true;
            }
            Err(err) => {
                let head = t!(self, "shape-library-failed");
                self.error_report(Report::failed(head, err.message()));
            }
        }
    }

    /// The group saved into last, for a save from a menu.
    pub(super) fn shape_library_group(&self) -> Option<String> {
        self.shape_library.last_group.clone()
    }

    /// Selected shapes dropped on the tab at `pos` (screen): saved into the group there.
    pub(super) fn drop_selection_on_library(&mut self, pos: Pos2) {
        let group = self
            .shape_library
            .group_rects
            .iter()
            .find(|(_, r)| r.contains(pos))
            .map_or_else(|| self.shape_library.last_group.clone(), |(g, _)| g.clone());
        self.ask_library_save(group);
    }

    /// Whether `pos` (screen) is where selected shapes can be dropped to save them.
    pub(super) fn over_library(&self, pos: Pos2) -> bool {
        self.properties_open
            && self.settings_dir.is_some()
            && self
                .shape_library
                .drop_rect
                .is_some_and(|r| r.contains(pos))
    }

    /// The groups, each a header that folds with its entries under it; the entries with no
    /// group first.
    fn entry_groups(&mut self, ui: &mut egui::Ui, dir: &std::path::Path) {
        let search = self.shape_library.search.trim().to_lowercase();
        let listing = self.shape_library.listing.clone();
        let matches =
            |item: &Item| search.is_empty() || item.full_name().to_lowercase().contains(&search);
        let mut rects = Vec::new();
        let top: Vec<&Item> = listing
            .items
            .iter()
            .filter(|i| i.group.is_none() && matches(i))
            .collect();
        if listing.items.is_empty() && listing.groups.is_empty() {
            ui.weak(t!(ui, "shape-library-empty"));
        }
        let (zone, dropped) = drop_zone(ui, |ui| {
            ui.set_min_width(ui.available_width());
            self.entry_grid(ui, &top);
        });
        rects.push((None, zone));
        if let Some(dragged) = dropped {
            self.move_entry(dir, &dragged.0, None);
        }
        for group in &listing.groups {
            let items: Vec<&Item> = listing
                .items
                .iter()
                .filter(|i| i.group.as_ref() == Some(group) && matches(i))
                .collect();
            if !search.is_empty() && items.is_empty() {
                continue;
            }
            let (zone, dropped) = drop_zone(ui, |ui| {
                ui.set_min_width(ui.available_width());
                let open = !self.shape_library.folded.contains(group) || !search.is_empty();
                let header = egui::CollapsingHeader::new(
                    RichText::new(format!("{group}  ({})", items.len())).strong(),
                )
                .id_salt(("shape-group", group))
                .open(Some(open))
                .show(ui, |ui| self.entry_grid(ui, &items));
                if header.header_response.clicked() {
                    if open {
                        self.shape_library.folded.insert(group.clone());
                    } else {
                        self.shape_library.folded.remove(group);
                    }
                }
                let empty = items.is_empty()
                    && !listing
                        .groups
                        .iter()
                        .any(|g| g.starts_with(&format!("{group}/")));
                header.header_response.context_menu(|ui| {
                    if ui.button(t!(ui, "shape-library-rename-menu")).clicked() {
                        self.shape_library.asking = Some(Asking::Rename {
                            from: group.clone(),
                            group: true,
                            name: group.clone(),
                        });
                        self.shape_library.focus_name = true;
                        ui.close();
                    }
                    if ui
                        .add_enabled(
                            empty,
                            egui::Button::new(t!(ui, "shape-library-delete-menu")),
                        )
                        .on_disabled_hover_text(t!(ui, "shape-library-group-not-empty"))
                        .clicked()
                    {
                        self.shape_library.asking = Some(Asking::Delete {
                            name: group.clone(),
                            group: true,
                        });
                        ui.close();
                    }
                    if ui.button(t!(ui, "shape-library-show-in-folder")).clicked() {
                        show_folder(&dir.join(group));
                        ui.close();
                    }
                });
            });
            rects.push((Some(group.clone()), zone));
            if let Some(dragged) = dropped {
                self.move_entry(dir, &dragged.0, Some(group));
            }
        }
        self.shape_library.group_rects = rects;
    }

    /// An entry dropped on a group: moved there.
    fn move_entry(&mut self, dir: &std::path::Path, name: &str, group: Option<&str>) {
        let from = name.rsplit_once('/').map(|(g, _)| g);
        if from != group {
            self.library_done(move_to(dir, name, group).map(|_| ()));
        }
    }

    /// Entries as a grid of cells, as many to a row as fit.
    fn entry_grid(&mut self, ui: &mut egui::Ui, items: &[&Item]) {
        let spacing = ui.spacing().item_spacing.x;
        let columns = ((ui.available_width() + spacing) / (CELL_WIDTH + spacing))
            .floor()
            .max(1.0) as usize;
        for row in items.chunks(columns) {
            ui.horizontal(|ui| {
                for item in row {
                    self.entry_cell(ui, item);
                }
            });
        }
    }

    /// One entry: its picture and its name. Dragged, it is inserted where it is dropped on
    /// the canvas; double-clicked, in the middle of the view; right-clicked, its menu.
    fn entry_cell(&mut self, ui: &mut egui::Ui, item: &Item) {
        let loaded = self.loaded(item);
        let height = PICTURE + 2.0 * ui.text_style_height(&egui::TextStyle::Small) + 8.0;
        let (rect, response) =
            ui.allocate_exact_size(vec2(CELL_WIDTH, height), Sense::click_and_drag());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, item.name.clone())
        });
        let visuals = ui.visuals().clone();
        let painter = ui.painter_at(rect);
        if response.hovered() || response.dragged() {
            painter.rect_filled(rect, 4.0, visuals.widgets.hovered.weak_bg_fill);
        }
        let picture = Rect::from_center_size(
            Pos2::new(rect.center().x, rect.top() + 4.0 + PICTURE / 2.0),
            vec2(PICTURE, PICTURE),
        );
        painter.rect_filled(picture, 3.0, visuals.extreme_bg_color);
        let full = item.full_name();
        match &loaded {
            Ok(loaded) => {
                paint_picture(&painter, picture.shrink(6.0), loaded, visuals.text_color())
            }
            Err(_) => {
                painter.text(
                    picture.center(),
                    Align2::CENTER_CENTER,
                    "⚠",
                    egui::FontId::proportional(24.0),
                    visuals.warn_fg_color,
                );
            }
        }
        let small = egui::TextStyle::Small.resolve(ui.style());
        let name = ui.fonts_mut(|f| {
            let mut job = egui::text::LayoutJob::simple(
                item.name.clone(),
                small,
                if loaded.is_ok() {
                    visuals.text_color()
                } else {
                    visuals.weak_text_color()
                },
                CELL_WIDTH - 4.0,
            );
            job.halign = egui::Align::Center;
            job.wrap.max_rows = 2;
            job.wrap.break_anywhere = false;
            job.wrap.overflow_character = Some('…');
            f.layout_job(job)
        });
        painter.galley(
            Pos2::new(rect.center().x, picture.bottom() + 3.0),
            name,
            visuals.text_color(),
        );
        let hover = match &loaded {
            Ok(_) => full.clone(),
            Err(err) => format!("{full}\n{err}"),
        };
        let response = response.on_hover_text(hover);
        if loaded.is_ok() && self.edit.is_some() {
            if response.drag_started() {
                response.dnd_set_drag_payload(DraggedEntry(full.clone()));
            }
            if response.double_clicked() {
                let at = self.shape_library.view_middle;
                self.insert_entry(&full, at, InsertSize::AsSaved);
            }
        }
        let selected = !self.framed_shape_ids().is_empty();
        response.context_menu(|ui| {
            let can_insert = loaded.is_ok() && self.edit.is_some();
            if ui
                .add_enabled(
                    can_insert,
                    egui::Button::new(t!(ui, "shape-library-insert-middle")),
                )
                .clicked()
            {
                let at = self.shape_library.view_middle;
                self.insert_entry(&full, at, InsertSize::AsSaved);
                ui.close();
            }
            if ui
                .add_enabled(
                    can_insert && selected,
                    egui::Button::new(t!(ui, "shape-library-insert-around")),
                )
                .on_hover_text(t!(ui, "shape-library-insert-around-hover"))
                .clicked()
            {
                let ids = self.framed_shape_ids();
                let margin = self.frame.margin;
                self.insert_entry(&full, None, InsertSize::Around { ids, margin });
                ui.close();
            }
            ui.separator();
            if ui.button(t!(ui, "shape-library-rename-menu")).clicked() {
                self.shape_library.asking = Some(Asking::Rename {
                    from: full.clone(),
                    group: false,
                    name: item.name.clone(),
                });
                self.shape_library.focus_name = true;
                ui.close();
            }
            let groups = self.shape_library.listing.groups.clone();
            ui.menu_button(t!(ui, "shape-library-move-to"), |ui| {
                let dir = self.shapes_folder();
                let mut target = None;
                if ui
                    .add_enabled(
                        item.group.is_some(),
                        egui::Button::new(t!(ui, "shape-library-no-group")),
                    )
                    .clicked()
                {
                    target = Some(None);
                }
                for g in &groups {
                    if ui
                        .add_enabled(item.group.as_ref() != Some(g), egui::Button::new(g))
                        .clicked()
                    {
                        target = Some(Some(g.clone()));
                    }
                }
                if let (Some(group), Some(dir)) = (target, dir) {
                    self.move_entry(&dir, &full, group.as_deref());
                    ui.close();
                }
            });
            if ui.button(t!(ui, "shape-library-delete-menu")).clicked() {
                self.shape_library.asking = Some(Asking::Delete {
                    name: full.clone(),
                    group: false,
                });
                ui.close();
            }
            if ui.button(t!(ui, "shape-library-show-in-folder")).clicked() {
                if let Some(folder) = item.path.parent() {
                    show_folder(folder);
                }
                ui.close();
            }
        });
    }

    /// Insert entry `name` with its middle at `at` (the stock's middle without): one undo
    /// step, the new shapes selected.
    pub(super) fn insert_entry(&mut self, name: &str, at: Option<Point>, size: InsertSize) -> bool {
        let Some(dir) = self.shapes_folder() else {
            return false;
        };
        let entry = match find(&dir, name).map(|item| self.loaded(&item)) {
            Ok(Ok(loaded)) => loaded,
            Ok(Err(text)) => {
                self.set_status(text, STATUS_SECONDS);
                return false;
            }
            Err(err) => {
                let head = t!(self, "shape-library-failed");
                self.error_report(Report::failed(head, err.message()));
                return false;
            }
        };
        let Some(edit) = self.edit.as_mut() else {
            return false;
        };
        match insert(&edit.settings, &entry.entry, at, &size, None) {
            Ok(done) => {
                edit.settings.drawing = done.job.drawing;
                self.drawing_changed(&done.shapes);
                let text = t!(self, "shape-library-inserted", name = name.to_string());
                self.set_status(text, STATUS_SECONDS);
                true
            }
            Err(err) => {
                let head = t!(self, "shape-library-failed");
                self.error_report(Report::failed(head, err.message()));
                false
            }
        }
    }

    /// While an entry is dragged over the canvas: its outlines where it would land, the
    /// middle of its box at the snapped pointer. Returns that point.
    pub(super) fn entry_drag_preview(&mut self, ctx: &egui::Context, pos: Pos2) -> Option<Point> {
        let dragged = egui::DragAndDrop::payload::<DraggedEntry>(ctx)?;
        let dir = self.shapes_folder()?;
        let item = find(&dir, &dragged.0).ok()?;
        let loaded = self.loaded(&item).ok()?;
        let alt = ctx.input(|i| i.modifiers.alt);
        let at = self.snap_world(self.view.to_world(pos), alt, false);
        let ((x0, y0), (x1, y1)) = loaded.bounds;
        let (dx, dy) = (at.0 - (x0 + x1) / 2.0, at.1 - (y0 + y1) / 2.0);
        self.scene.preview.extend(
            loaded
                .lines
                .iter()
                .map(|line| line.iter().map(|p| (p.0 + dx, p.1 + dy)).collect()),
        );
        Some(at)
    }

    /// While selected shapes are carried over the tab: their outline by the pointer.
    pub(super) fn paint_carried(&self, ctx: &egui::Context, pos: Pos2) {
        let Some(edit) = &self.edit else {
            return;
        };
        let Ok(entry) = entry_from(&edit.settings, &self.framed_shape_ids(), &[]) else {
            return;
        };
        let Some(bounds) = entry_box(&entry) else {
            return;
        };
        let loaded = Loaded {
            lines: preview(&entry),
            entry,
            bounds,
        };
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::new("shape-library-carried"),
        ));
        let rect = Rect::from_min_size(pos + vec2(12.0, 12.0), vec2(PICTURE, PICTURE));
        let visuals = ctx.global_style().visuals.clone();
        painter.rect_filled(rect, 3.0, visuals.extreme_bg_color);
        painter.rect_stroke(
            rect,
            3.0,
            visuals.selection.stroke,
            egui::StrokeKind::Inside,
        );
        paint_picture(
            &painter,
            rect.shrink(6.0),
            &loaded,
            visuals.selection.stroke.color,
        );
    }
}

/// A place an entry can be dropped on: outlined while one is dragged over it. Returns its
/// rect and the entry dropped there.
fn drop_zone(
    ui: &mut egui::Ui,
    contents: impl FnOnce(&mut egui::Ui),
) -> (Rect, Option<Arc<DraggedEntry>>) {
    let rect = ui.scope(contents).response.rect;
    let response = ui.interact(rect, ui.next_auto_id(), Sense::hover());
    if response.dnd_hover_payload::<DraggedEntry>().is_some() {
        ui.painter().rect_stroke(
            rect,
            4.0,
            ui.visuals().selection.stroke,
            egui::StrokeKind::Inside,
        );
    }
    (rect, response.dnd_release_payload::<DraggedEntry>())
}

/// An entry's lines scaled into `rect`, Y up.
fn paint_picture(painter: &egui::Painter, rect: Rect, loaded: &Loaded, color: Color32) {
    let ((x0, y0), (x1, y1)) = loaded.bounds;
    let (w, h) = ((x1 - x0).max(1e-9), (y1 - y0).max(1e-9));
    let scale = (f64::from(rect.width()) / w).min(f64::from(rect.height()) / h);
    let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let middle = rect.center();
    let at = |p: &(f64, f64)| {
        Pos2::new(
            middle.x + ((p.0 - cx) * scale) as f32,
            middle.y - ((p.1 - cy) * scale) as f32,
        )
    };
    for line in &loaded.lines {
        let points: Vec<Pos2> = line.iter().map(at).collect();
        painter.add(egui::Shape::line(points, Stroke::new(1.0, color)));
    }
}

/// A name for a new entry: its first text's words, else the layer of its first shape.
fn suggested_name(entry: &ShapeEntry) -> String {
    let words = entry.texts.first().map(|t| t.text.replace('\n', " "));
    let name = words.unwrap_or_else(|| {
        entry
            .shapes
            .first()
            .map(|s| s.layer.clone())
            .unwrap_or_default()
    });
    let name: String = name
        .chars()
        .filter(|c| !c.is_control() && !"/\\:*?\"<>|".contains(*c))
        .take(40)
        .collect();
    name.trim().trim_matches('.').trim().to_string()
}

/// Open `folder` in the system's file manager.
pub(super) fn show_folder(folder: &std::path::Path) {
    let program = if cfg!(target_os = "windows") {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    // Nothing to report if there is no file manager: the folder is on the button's hover.
    let _ = std::process::Command::new(program).arg(folder).spawn();
}
