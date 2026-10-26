// SPDX-License-Identifier: GPL-3.0-or-later
//! Dogbones chosen per corner on the canvas (docs/specs/dogbone-per-corner.md §6): the
//! focused profile's or pocket's corners as markers, and a menu that gives one its own bone.

use super::*;
use chipload_core::order::Ordered;
use chipload_job::{Bone, Wall};
use chipload_pipeline::corners::{CornerRow, corners_in, set_choice};
use chipload_toolpath::corners::{Gets, gets, notch_centre};

use crate::view::CornerMarker;

/// A corner's own choice: None for "as the operation".
type Choice = Option<(Bone, Option<Wall>)>;

/// The menu's entries, in order.
const CHOICES: [Choice; 5] = [
    None,
    Some((Bone::None, None)),
    Some((Bone::Dogbone, None)),
    Some((Bone::Tbone, Some(Wall::Longer))),
    Some((Bone::Tbone, Some(Wall::Shorter))),
];

/// The focused operation's corners, found again only when the operation or the drawing
/// changes.
#[derive(Default)]
pub(super) struct CornerMarkers {
    /// What `rows` were found for: the operation's index and the operation as it was.
    key: Option<(usize, Operation)>,
    rows: Vec<CornerRow>,
    /// The corner whose menu is open (index in `rows`).
    menu: Option<usize>,
    /// Corners the last generation's bones couldn't reach, by operation.
    pub(super) unreached: Vec<(usize, Point)>,
}

impl CornerMarkers {
    /// Find the corners again: the drawing changed.
    pub(super) fn forget(&mut self) {
        self.key = None;
    }
}

impl ChiploadApp {
    /// The focused profile's or pocket's corners on the canvas, in top view.
    pub(super) fn update_corner_markers(&mut self) {
        let focused = match (self.focus, &self.edit) {
            (Focus::Op(i), Some(edit)) => edit.ops.get(i).map(|e| (i, e.op.clone())),
            _ => None,
        };
        let shown = !self.three_d && self.sketch.is_none() && self.show_drawing();
        let Some((index, op)) = focused.filter(|_| shown) else {
            self.scene.corner_markers.clear();
            self.corners.menu = None;
            return;
        };
        if self.corners.key.as_ref() != Some(&(index, op.clone())) {
            let label = op.label_message(index);
            self.corners.rows = corners_in(&op, &self.contours, &label).unwrap_or_default();
            self.corners.key = Some((index, op.clone()));
            self.corners.menu = None;
        }
        let radius = self.tool_radius(&op);
        let clearance = op.clearance();
        let unreached = |at: Point| {
            self.corners.unreached.iter().any(|&(op, p)| {
                op == index
                    && chipload_core::primitives::distance(p, at)
                        <= chipload_core::tolerances::ARC_APPROX_TOL_MM
            })
        };
        self.scene.corner_markers = self
            .corners
            .rows
            .iter()
            .map(|row| {
                let unreached = row.gets.is_some() && unreached(row.corner.at);
                CornerMarker {
                    at: row.corner.at,
                    notch: row
                        .gets
                        .zip(radius)
                        .filter(|_| !unreached)
                        .map(|(bone, r)| (notch_centre(&row.corner, bone, r, clearance), r)),
                    chosen: row.chosen,
                    unreached,
                }
            })
            .collect();
    }

    /// The radius of the tool `op` cuts with, if it is in the project.
    fn tool_radius(&self, op: &Operation) -> Option<f64> {
        let edit = self.edit.as_ref()?;
        let tool = edit.settings.tools.iter().find(|t| t.id == op.tool())?;
        Some(tool.diameter / 2.0)
    }

    /// The corner marker at `pos`, if any.
    pub(super) fn corner_marker_at(&self, pos: egui::Pos2) -> Option<usize> {
        self.scene
            .corner_markers
            .iter()
            .enumerate()
            .map(|(i, m)| (i, self.view.to_screen(m.at).distance(pos)))
            .filter(|&(_, d)| d <= crate::view::PICK_TOLERANCE_PX + 2.0)
            .min_by_key(|&(_, d)| Ordered(f64::from(d)))
            .map(|(i, _)| i)
    }

    /// Whether no corner's menu is open.
    pub(super) fn corners_menu_closed(&self) -> bool {
        self.corners.menu.is_none()
    }

    /// Open corner `k`'s menu, beside it.
    pub(super) fn open_corner_menu(&mut self, k: usize) {
        self.corners.menu = Some(k);
    }

    /// What hovering corner `k` says: where it is, its angle and what it gets.
    pub(super) fn corner_hover(&self, k: usize) -> String {
        let Some(row) = self.corners.rows.get(k) else {
            return String::new();
        };
        let unreached = self
            .scene
            .corner_markers
            .get(k)
            .is_some_and(|m| m.unreached);
        let id = if unreached {
            "canvas-corner-hover-unreached"
        } else if row.chosen {
            "canvas-corner-hover-own"
        } else {
            "canvas-corner-hover"
        };
        chipload_i18n::Message::new(id)
            .arg(
                "at",
                chipload_i18n::Value::Point(row.at[0], row.at[1], chipload_i18n::Digits::Fixed(3)),
            )
            .arg(
                "angle",
                chipload_i18n::Value::Number(
                    row.angle.to_degrees(),
                    chipload_i18n::Digits::Fixed(1),
                ),
            )
            .arg("gets", gets_text(self, row.gets))
            .render(self.language, self.units.into())
    }

    /// The open corner menu: the five choices, the current one ticked; hovering one draws
    /// its bone. A click stores it as one undo step and generates again.
    pub(super) fn corner_menu(&mut self, ui: &egui::Ui) {
        self.scene.corner_preview = None;
        let Some(k) = self.corners.menu else {
            return;
        };
        let (Some(row), Some((index, op))) =
            (self.corners.rows.get(k).copied(), self.corners.key.clone())
        else {
            self.corners.menu = None;
            return;
        };
        let current: Choice = op
            .corner_choices()
            .iter()
            .find(|c| {
                chipload_core::primitives::distance((c.at[0], c.at[1]), row.corner.at)
                    <= chipload_core::tolerances::ARC_APPROX_TOL_MM
            })
            .map(|c| (c.bone, c.along));
        let operation_gets = gets(&row.corner, op.corners(), &[]);
        let radius = self.tool_radius(&op);
        let mut open = true;
        let mut picked: Option<Choice> = None;
        let mut hovered: Option<Choice> = None;
        egui::Popup::new(
            egui::Id::new("corner-menu"),
            ui.ctx().clone(),
            egui::PopupAnchor::Position(
                self.view.to_screen(row.corner.at) + egui::Vec2::new(10.0, 10.0),
            ),
            ui.layer_id(),
        )
        .kind(egui::PopupKind::Menu)
        .layout(egui::Layout::top_down_justified(egui::Align::Min))
        .open_bool(&mut open)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClick)
        .show(|ui| {
            for choice in CHOICES {
                let label = match choice {
                    None => t!(
                        ui,
                        "canvas-corner-as-operation",
                        gets = gets_text(ui, operation_gets)
                    ),
                    Some((Bone::None, _)) => t!(ui, "canvas-corner-round"),
                    Some((Bone::Dogbone, _)) => t!(ui, "canvas-corner-dogbone"),
                    Some((Bone::Tbone, Some(Wall::Shorter))) => {
                        t!(ui, "canvas-corner-tbone-shorter")
                    }
                    Some((Bone::Tbone, _)) => t!(ui, "canvas-corner-tbone-longer"),
                };
                let response = ui.selectable_label(choice == current, label);
                if response.hovered() {
                    hovered = Some(choice);
                }
                if response.clicked() {
                    picked = Some(choice);
                }
            }
        });
        if let (Some(choice), Some(r)) = (hovered, radius) {
            let bone = match choice {
                None => operation_gets,
                Some((Bone::None, _)) => None,
                Some((Bone::Dogbone, _)) => Some(Gets::Dogbone),
                Some((Bone::Tbone, wall)) => Some(Gets::Tbone(wall.unwrap_or(Wall::Longer))),
            };
            self.scene.corner_preview =
                bone.map(|b| (notch_centre(&row.corner, b, r, op.clearance()), r));
        }
        if let Some(choice) = picked {
            self.choose_corner(index, row.at, choice);
            open = false;
        }
        if !open {
            self.corners.menu = None;
        }
    }

    /// Store a corner's choice on operation `index`, as one undo step, and generate again.
    pub(super) fn choose_corner(&mut self, index: usize, at: [f64; 2], choice: Choice) {
        let Some(entry) = self.edit.as_mut().and_then(|e| e.ops.get_mut(index)) else {
            return;
        };
        set_choice(&mut entry.op, at, choice);
        self.commit();
        self.generate();
    }
}

/// What a corner gets, in words.
fn gets_text(lang: &impl crate::i18n::Lang, gets: Option<Gets>) -> String {
    match gets {
        None => t!(lang, "canvas-corner-gets-round"),
        Some(Gets::Dogbone) => t!(lang, "canvas-corner-gets-dogbone"),
        Some(Gets::Tbone(Wall::Longer)) => t!(lang, "canvas-corner-gets-tbone-longer"),
        Some(Gets::Tbone(Wall::Shorter)) => t!(lang, "canvas-corner-gets-tbone-shorter"),
    }
}
