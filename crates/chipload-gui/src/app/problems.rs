// SPDX-License-Identifier: GPL-3.0-or-later
//! Problems shown where they are while editing (docs/specs/live-problems.md): a red frame
//! on the field, a dot on its section, a line on the operation card and the Problems strip.
//! The rules are the job's and the toolpath's; this only shows them.

use std::collections::BTreeMap;
use std::sync::Arc;

use chipload_job::Problem;

use super::*;

/// The problems of the form being drawn, by field (relative to the operation for an
/// operation's form, the project path for the job form), in plain words. Kept in egui's
/// temporary data while the form is drawn, so its rows needn't pass it along.
#[derive(Clone, Default)]
pub(super) struct FormProblems(Arc<BTreeMap<String, String>>);

fn form_key() -> egui::Id {
    egui::Id::new("form-problems")
}

/// The problems the form about to be drawn shows (empty: none).
pub(super) fn set_form_problems(ui: &egui::Ui, problems: BTreeMap<String, String>) {
    ui.data_mut(|d| d.insert_temp(form_key(), FormProblems(Arc::new(problems))));
}

fn form_problems(ui: &egui::Ui) -> FormProblems {
    ui.data(|d| d.get_temp::<FormProblems>(form_key()))
        .unwrap_or_default()
}

/// `field` is `prefix` or lies under it (`tabs.width` under `tabs`).
fn under(field: &str, prefix: &str) -> bool {
    field == prefix
        || field
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with('.'))
}

/// Mark the field `response` belongs to red, with its problem on hover, if it has one.
pub(super) fn flag(response: egui::Response, ui: &egui::Ui, field: &str) -> egui::Response {
    let Some(text) = form_problems(ui).0.get(field).cloned() else {
        return response;
    };
    let color = ui.visuals().error_fg_color;
    ui.painter().rect_stroke(
        response.rect.expand(2.0),
        3.0,
        egui::Stroke::new(1.5, color),
        egui::StrokeKind::Outside,
    );
    response.on_hover_text(RichText::new(text).color(color))
}

/// A field at or under `prefix` (`tools.0`) has a problem.
pub(super) fn has_problem_under(ui: &egui::Ui, prefix: &str) -> bool {
    form_problems(ui).0.keys().any(|field| under(field, prefix))
}

/// The fields each form section holds, for the dot on its header.
pub(super) fn section_fields(key: &str) -> &'static [&'static str] {
    match key {
        "op-cut" => &["start_depth", "end_depth", "clearance", "step_down"],
        // A profile's ramp angle sits with its depths (docs/specs/profile-form.md).
        "op-profile-cut" => &[
            "start_depth",
            "end_depth",
            "clearance",
            "step_down",
            "ramp_angle",
        ],
        "op-feeds" => &["feed", "plunge_feed"],
        "op-entry" => &["entry", "lead"],
        "op-holding" => &["tabs", "skin"],
        "op-clearing" => &["stepover", "rest"],
        "job-stock" => &["stock"],
        "part-copies" => &["parts"],
        "job-machine" => &["machine"],
        "job-tools" => &["tools"],
        "job-placement" => &["drawing"],
        _ => &[],
    }
}

/// A field of section `key` has a problem.
pub(super) fn section_has_problem(ui: &egui::Ui, key: &str) -> bool {
    let problems = form_problems(ui);
    section_fields(key)
        .iter()
        .any(|prefix| problems.0.keys().any(|field| under(field, prefix)))
}

/// What the form calls a field: of an operation (`feed`), or of the job (`machine.safe_z`).
fn field_label(field: &str) -> &'static str {
    let parts: Vec<&str> = field.split('.').collect();
    match parts.as_slice() {
        ["stock", "x"] => return "Stock X",
        ["stock", "y"] => return "Stock Y",
        ["stock", "thickness"] => return "Thickness",
        ["stock", "max_overcut"] => return "Max depth below stock",
        ["machine", "safe_z"] => return "Safe Z",
        ["machine", "travel", ..] => return "Travel",
        ["parts", _, "copies", ..] => return "Copies",
        ["parts", _, "name"] => return "Part name",
        ["parts", ..] => return "Parts",
        ["tools", _, "diameter"] => return "Tool diameter",
        ["tools", _, "rpm"] => return "Spindle speed",
        ["tools", _, "angle"] => return "V-bit angle",
        ["tools", _, "tip_diameter"] => return "Tip",
        _ => {}
    }
    match field {
        "feed" => "Feed",
        "plunge_feed" => "Plunge feed",
        "start_depth" => "Start depth",
        "end_depth" => "End depth",
        "step_down" => "Step down",
        "stepover" => "Stepover",
        "clearance" => "Clearance",
        "entry" => "Entry",
        "ramp_angle" => "Ramp angle",
        "lead" => "Lead",
        "tool" => "Tool",
        "part" => "Part",
        "side" => "Side",
        "select" => "Geometry",
        f if f.starts_with("tabs") => "Tabs",
        _ => "",
    }
}

fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// The problem in plain words at its field: "Must be greater than 0, got 0".
pub(super) fn at_field(problem: &Problem) -> String {
    capitalized(&problem.short)
}

/// The problem in plain words on its own, with the field's name: "Feed: must be greater
/// than 0, got 0". A message that already names the field isn't given it twice.
pub(super) fn plain(problem: &Problem) -> String {
    let label = field_label(problem.field());
    if label.is_empty()
        || problem
            .short
            .to_lowercase()
            .starts_with(&label.to_lowercase())
    {
        capitalized(&problem.short)
    } else {
        format!("{label}: {}", problem.short)
    }
}

impl ChiploadApp {
    /// Check the project as it is now (after a finished edit, or when opened).
    pub(super) fn refresh_problems(&mut self) {
        self.problems = self
            .edit
            .as_ref()
            .map(EditJob::problems)
            .unwrap_or_default();
    }

    /// The problems of operation `i`.
    pub(super) fn op_problems(&self, i: usize) -> impl Iterator<Item = &Problem> {
        self.problems
            .iter()
            .filter(move |p| p.operation() == Some(i))
    }

    /// The problems of the job's own settings (not of an operation).
    pub(super) fn job_problems(&self) -> impl Iterator<Item = &Problem> {
        self.problems.iter().filter(|p| p.operation().is_none())
    }

    /// The problems of the job settings form itself: not of a tool, a part or the drawing,
    /// which have forms of their own (docs/specs/properties-for-tools-and-dxfs.md).
    pub(super) fn settings_problems(&self) -> impl Iterator<Item = &Problem> {
        self.job_problems().filter(|p| {
            let field = p.field();
            !(field.starts_with("tools")
                || field.starts_with("parts")
                || field.starts_with("drawing"))
        })
    }

    /// The problems under `prefix` ("tools.0."), of the job's own settings.
    fn problems_under(&self, prefix: Option<String>) -> impl Iterator<Item = &Problem> {
        self.job_problems().filter(move |p| {
            prefix
                .as_ref()
                .is_some_and(|prefix| p.field().starts_with(prefix.as_str()))
        })
    }

    /// The problems the form for `focus` shows, by field.
    pub(super) fn form_problem_map(&self, focus: Focus) -> BTreeMap<String, String> {
        let settings = self.edit.as_ref().map(|e| &e.settings);
        let index_of = |list: Option<usize>, what: &str| list.map(|k| format!("{what}.{k}."));
        let pick: Box<dyn Iterator<Item = &Problem>> = match focus {
            Focus::Op(i) => Box::new(self.op_problems(i)),
            Focus::Job => Box::new(self.settings_problems()),
            Focus::Tools => Box::new(self.problems_under(Some("tools.".into()))),
            Focus::LibraryTools
            | Focus::LibraryTool(_)
            | Focus::LibraryMachines
            | Focus::LibraryMachine(_)
            | Focus::Preferences
            | Focus::Posts
            | Focus::Post(_) => Box::new(std::iter::empty()),
            Focus::Part(id) => {
                let k = settings.and_then(|s| s.parts.iter().position(|p| p.id == id));
                Box::new(self.problems_under(index_of(k, "parts")))
            }
            Focus::Tool(id) => {
                let k = settings.and_then(|s| s.tools.iter().position(|t| t.id == id));
                Box::new(self.problems_under(index_of(k, "tools")))
            }
            Focus::Dxf(id) => {
                let k = settings.and_then(|s| s.drawing.dxfs.iter().position(|d| d.id == id));
                Box::new(self.problems_under(index_of(k, "drawing.dxfs")))
            }
            Focus::Image(id) => {
                let k = settings.and_then(|s| s.drawing.images.iter().position(|i| i.id == id));
                Box::new(self.problems_under(index_of(k, "drawing.images")))
            }
        };
        let mut map = BTreeMap::new();
        for problem in pick {
            map.entry(problem.field().to_string())
                .or_insert_with(|| at_field(problem));
        }
        map
    }

    /// The Problems strip at the top of the properties panel: one line per problem; a
    /// click opens its operation or the job settings.
    pub(super) fn problems_strip(&mut self, ui: &mut egui::Ui) {
        if self.problems.is_empty() {
            return;
        }
        let color = ui.visuals().error_fg_color;
        let parts: Vec<u64> = self
            .edit
            .as_ref()
            .map(|e| e.settings.parts.iter().map(|p| p.id).collect())
            .unwrap_or_default();
        let tools: Vec<i64> = self
            .edit
            .as_ref()
            .map(|e| e.settings.tools.iter().map(|t| t.id).collect())
            .unwrap_or_default();
        let mut go = None;
        egui::Frame::new()
            .stroke(egui::Stroke::new(1.0, color))
            .corner_radius(4.0)
            .inner_margin(egui::Margin::symmetric(8, 6))
            .show(ui, |ui| {
                let count = self.problems.len();
                let s = if count == 1 { "" } else { "s" };
                ui.colored_label(color, RichText::new(format!("{count} problem{s}")).strong());
                if self.auto_generate {
                    ui.weak("The toolpath updates once they are fixed.");
                }
                for problem in &self.problems {
                    let place = match problem.operation() {
                        Some(i) => format!("Operation {}", i + 1),
                        None => "Job".to_string(),
                    };
                    let line = format!("{place} · {}", plain(problem));
                    if ui
                        .add(egui::Label::new(line).sense(egui::Sense::click()))
                        .on_hover_text("Show it")
                        .clicked()
                    {
                        // A part's or a tool's field opens it (docs/specs/parts.md §8,
                        // properties-for-tools-and-dxfs.md §2).
                        let index = |what: &str| {
                            problem
                                .field()
                                .strip_prefix(what)
                                .and_then(|rest| rest.split('.').next())
                                .and_then(|k| k.parse::<usize>().ok())
                        };
                        let part = index("parts.").and_then(|k| parts.get(k).copied());
                        let tool = index("tools.").and_then(|k| tools.get(k).copied());
                        go = Some(match (problem.operation(), part, tool) {
                            (Some(i), _, _) => Focus::Op(i),
                            (None, Some(id), _) => Focus::Part(id),
                            (None, None, Some(id)) => Focus::Tool(id),
                            (None, None, None) => Focus::Job,
                        });
                    }
                }
            });
        ui.add_space(6.0);
        if let Some(focus) = go {
            self.focus = focus;
        }
    }
}
