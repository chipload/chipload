// SPDX-License-Identifier: GPL-3.0-or-later
//! The report under the canvas (docs/specs/look-and-feel.md §6): the last action's result
//! as a pill in its tone, its warnings in a card of their own, the time estimate as a
//! table, the other notes (a line naming an operation selects it), and the drawing's facts.

use chipload_core::chain::Contour;
use chipload_dxf::{ImportReport, Severity};
use chipload_i18n::Message;
use chipload_pipeline::report::{contour_messages, import_report_messages};
use eframe::egui::{self, Color32, RichText, ScrollArea, Sense};

use super::{ChiploadApp, Focus};
use crate::i18n::{Lang, t};
use chipload_job::Problem;

/// How a line reads: its colour and mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Info,
    Good,
    Warning,
    Error,
}

impl Tone {
    fn mark(self) -> &'static str {
        match self {
            Tone::Info => "•",
            Tone::Good => "✔",
            Tone::Warning => "⚠",
            Tone::Error => "✖",
        }
    }

    pub fn color(self, visuals: &egui::Visuals) -> Color32 {
        match self {
            Tone::Info => visuals.text_color(),
            Tone::Good => crate::style::good_color(visuals),
            Tone::Warning => visuals.warn_fg_color,
            Tone::Error => visuals.error_fg_color,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReportLine {
    pub tone: Tone,
    /// The line as built; for a message from the library crates, in English and mm.
    pub text: String,
    /// A message from the library crates, shown in the app's language and display units
    /// (docs/specs/translations.md §7).
    pub message: Option<Message>,
    /// What a click on the line opens or does, if anything.
    pub target: Option<Target>,
}

/// What a report line is about (docs/specs/tool-updates.md decision 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// An operation (index): a click opens it.
    Op(usize),
    /// A project tool (id): a click opens it.
    Tool(i64),
    /// *Update all from library*: a click updates the project's tools.
    UpdateTools,
}

impl ReportLine {
    /// The line as shown: a message in the language of `ui` and the display units; other
    /// text, the app's own, as it was made (its lengths already in the display units).
    fn shown(&self, ui: &egui::Ui) -> String {
        match &self.message {
            Some(message) => message.render(ui.lang(), crate::units::display(ui.ctx()).into()),
            None => self.text.clone(),
        }
    }
}

/// What the last action had to say. The first line is the headline.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Report {
    pub lines: Vec<ReportLine>,
    /// A generated program's time estimate, shown as a table.
    pub estimate: Option<EstimateTable>,
}

/// The time estimate as a table: its total, then a row per part, operation and file.
#[derive(Debug, Clone, PartialEq)]
pub struct EstimateTable {
    /// "Estimated time: …", with its parts.
    pub total: Message,
    pub rows: Vec<EstimateRow>,
}

/// One row of [`EstimateTable`].
#[derive(Debug, Clone, PartialEq)]
pub struct EstimateRow {
    pub label: RowLabel,
    pub time: Message,
    /// Under a part's row.
    pub indented: bool,
    pub target: Option<Target>,
}

/// What a row of the estimate names.
#[derive(Debug, Clone, PartialEq)]
pub enum RowLabel {
    Part {
        number: usize,
        name: String,
    },
    /// An operation's or a rest pass's label, from the project.
    Operation(Message),
    File(usize),
}

impl RowLabel {
    fn shown(&self, ui: &egui::Ui) -> String {
        match self {
            RowLabel::Part { number, name } => {
                t!(ui, "report-part", number = *number, name = name.as_str())
            }
            RowLabel::Operation(message) => {
                message.render(ui.lang(), crate::units::display(ui.ctx()).into())
            }
            RowLabel::File(number) => t!(ui, "report-file", number = *number),
        }
    }
}

impl Report {
    /// Plain lines, all informational.
    pub fn info(lines: impl IntoIterator<Item = String>) -> Self {
        let mut report = Report::default();
        for line in lines {
            report.push(Tone::Info, line);
        }
        report
    }

    /// A failure: every line is an error.
    pub fn error(lines: impl IntoIterator<Item = String>) -> Self {
        let mut report = Report::default();
        for line in lines {
            report.push(Tone::Error, line);
        }
        report
    }

    /// Messages from the library crates (a report of a change), all informational.
    pub fn of_messages(messages: impl IntoIterator<Item = Message>) -> Self {
        let mut report = Report::default();
        for message in messages {
            report.push_message(Tone::Info, message, None);
        }
        report
    }

    /// A failure that its message from the library crates (an error) says all of.
    pub fn of_error(why: Message) -> Self {
        let mut report = Report::default();
        report.push_message(Tone::Error, why, None);
        report
    }

    /// A failure: `head`, then why, a message from the library crates (an error).
    pub fn failed(head: String, why: Message) -> Self {
        let mut report = Report::error([head]);
        report.push_message(Tone::Error, why, None);
        report
    }

    /// A failure: `head`, then every problem, each leading to its operation.
    pub fn of_problems(head: &str, problems: &[Problem]) -> Self {
        let mut report = Report::error([head.to_string()]);
        for problem in problems {
            let line = Message::new("report-indented").arg("line", problem.text_message.clone());
            report.push_message(Tone::Error, line, problem.operation());
        }
        report
    }

    /// A line about nothing in particular.
    pub fn push(&mut self, tone: Tone, text: String) {
        self.push_at(tone, text, None);
    }

    /// A line about operation `op` (an index), or about the project with None.
    pub fn push_at(&mut self, tone: Tone, text: String, op: Option<usize>) {
        self.lines.push(ReportLine {
            tone,
            text,
            message: None,
            target: op.map(Target::Op),
        });
    }

    /// A line of the app's own that a click acts on.
    pub fn push_target(&mut self, tone: Tone, text: String, target: Target) {
        self.lines.push(ReportLine {
            tone,
            text,
            message: None,
            target: Some(target),
        });
    }

    /// A message from the library crates (a note, a line of the time estimate), about
    /// operation `op` or the project.
    pub fn push_message(&mut self, tone: Tone, message: Message, op: Option<usize>) {
        self.lines.push(ReportLine {
            tone,
            text: message.english(),
            message: Some(message),
            target: op.map(Target::Op),
        });
    }

    /// The lines' text, in order.
    #[cfg(test)]
    pub fn iter(&self) -> impl Iterator<Item = &String> {
        self.lines.iter().map(|l| &l.text)
    }

    /// Lines that are warnings or errors.
    pub fn problems(&self) -> usize {
        self.lines
            .iter()
            .filter(|l| matches!(l.tone, Tone::Warning | Tone::Error))
            .count()
    }
}

impl std::ops::Index<usize> for Report {
    type Output = String;

    fn index(&self, index: usize) -> &String {
        &self.lines[index].text
    }
}

/// The current drawing's facts: units, extents, issues, the contour table.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DrawingFacts {
    /// Shown in the app's language and display units.
    pub lines: Vec<Message>,
    /// Why the drawing could not be read, instead of the lines.
    pub error: Option<Message>,
    pub contours: usize,
    /// Import issues; errors among them make the section red.
    pub issues: usize,
    pub errors: bool,
}

impl DrawingFacts {
    pub fn new(report: &ImportReport, contours: &[Contour]) -> Self {
        let mut lines = import_report_messages(report);
        lines.extend(contour_messages(contours));
        DrawingFacts {
            lines,
            error: None,
            contours: contours.len(),
            issues: report.issues.len(),
            errors: report.issues.iter().any(|i| i.severity == Severity::Error),
        }
    }

    /// The drawing could not be read.
    pub fn failed(message: Message) -> Self {
        DrawingFacts {
            lines: Vec::new(),
            error: Some(message),
            contours: 0,
            issues: 1,
            errors: true,
        }
    }

    /// The facts as shown: in the language of `ui` and the display units.
    fn text(&self, ui: &egui::Ui) -> String {
        if let Some(error) = &self.error {
            return error.render(ui.lang(), crate::units::display(ui.ctx()).into());
        }
        let units = crate::units::display(ui.ctx()).into();
        let lines: Vec<String> = self
            .lines
            .iter()
            .map(|line| line.render(ui.lang(), units))
            .collect();
        lines.join("\n")
    }

    fn title(&self, src: &impl crate::i18n::Lang) -> String {
        t!(
            src,
            "report-drawing-title",
            contours = self.contours,
            issues = self.issues
        )
    }
}

impl ChiploadApp {
    /// Show a new report; one with warnings or errors opens the panel.
    pub(super) fn set_report(&mut self, report: Report) {
        if report.problems() > 0 {
            self.panels.show(super::layout::Pane::Report);
        }
        self.report = report;
    }

    /// The Report's pane: the headline as a pill in its tone, then the rest and the
    /// drawing's facts.
    pub(super) fn report_pane(&mut self, ui: &mut egui::Ui) {
        if let Some(first) = self.report.lines.first().cloned() {
            let tint = first.tone.color(ui.visuals()).linear_multiply(0.14);
            egui::Frame::new()
                .fill(tint)
                .corner_radius(12)
                .inner_margin(egui::Margin::symmetric(10, 3))
                .show(ui, |ui| self.report_line(ui, &first, true));
            ui.add_space(4.0);
        }
        self.report_body(ui);
    }

    fn report_body(&mut self, ui: &mut egui::Ui) {
        ScrollArea::vertical()
            .id_salt("report")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let lines: Vec<ReportLine> = self.report.lines.iter().skip(1).cloned().collect();
                let (problems, notes): (Vec<ReportLine>, Vec<ReportLine>) = lines
                    .into_iter()
                    .partition(|l| matches!(l.tone, Tone::Warning | Tone::Error));
                // Warnings and errors first, in a card striped in the worst one's colour.
                if !problems.is_empty() {
                    let worst = if problems.iter().any(|l| l.tone == Tone::Error) {
                        Tone::Error
                    } else {
                        Tone::Warning
                    };
                    let stripe = Some(worst.color(ui.visuals()));
                    super::forms::card_with(ui, stripe, |ui| {
                        for line in &problems {
                            self.report_line(ui, line, false);
                        }
                    });
                }
                if let Some(table) = self.report.estimate.clone() {
                    super::forms::card_with(ui, None, |ui| self.estimate_table(ui, &table));
                }
                if !notes.is_empty() {
                    super::forms::card_with(ui, None, |ui| {
                        for line in &notes {
                            self.report_line(ui, line, false);
                        }
                    });
                }
                if let Some(facts) = &self.drawing_facts {
                    let mut title = RichText::new(facts.title(ui));
                    if facts.issues > 0 {
                        let tone = if facts.errors {
                            Tone::Error
                        } else {
                            Tone::Warning
                        };
                        title = title.color(tone.color(ui.visuals()));
                    }
                    let text = facts.text(ui);
                    let open = facts.issues > 0;
                    super::forms::card_with(ui, None, |ui| {
                        egui::CollapsingHeader::new(title)
                            .id_salt("drawing-facts")
                            .default_open(open)
                            .show(ui, |ui| {
                                ScrollArea::horizontal()
                                    .id_salt("drawing-facts-scroll")
                                    .show(ui, |ui| {
                                        ui.add(
                                            egui::Label::new(RichText::new(text).monospace())
                                                .extend(),
                                        );
                                    });
                            });
                    });
                }
            });
    }

    /// The estimate: its total in bold, then its rows, each name beside its time, a row
    /// about an operation opening it.
    fn estimate_table(&mut self, ui: &mut egui::Ui, table: &EstimateTable) {
        let units = crate::units::display(ui.ctx()).into();
        ui.label(RichText::new(table.total.render(ui.lang(), units)).strong());
        if table.rows.is_empty() {
            return;
        }
        ui.add_space(2.0);
        egui::Grid::new("report-estimate")
            .num_columns(2)
            .spacing(egui::vec2(24.0, 3.0))
            .show(ui, |ui| {
                for row in &table.rows {
                    let name = row.label.shown(ui);
                    let indent = if row.indented { 16.0 } else { 0.0 };
                    let response = ui
                        .horizontal(|ui| {
                            ui.add_space(indent);
                            let text = match row.label {
                                RowLabel::Operation(_) => RichText::new(name),
                                _ => RichText::new(name).strong(),
                            };
                            ui.add(egui::Label::new(text).sense(Sense::click()))
                        })
                        .inner;
                    ui.label(row.time.render(ui.lang(), units));
                    ui.end_row();
                    if let Some(Target::Op(op)) = row.target {
                        let response = response.on_hover_text(t!(
                            ui,
                            "report-select-operation-hover",
                            number = op + 1
                        ));
                        if response.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        if response.clicked() {
                            self.focus = Focus::Op(op);
                        }
                    }
                }
            });
    }

    /// One line: its mark and text in its tone; a line about an operation selects it.
    pub(super) fn report_line(&mut self, ui: &mut egui::Ui, line: &ReportLine, headline: bool) {
        let color = line.tone.color(ui.visuals());
        let mut text = RichText::new(line.shown(ui)).color(color);
        if headline {
            text = text.strong();
        }
        let row = ui.horizontal_top(|ui| {
            ui.label(RichText::new(line.tone.mark()).color(color));
            ui.add(egui::Label::new(text).wrap().sense(Sense::click()))
        });
        let response = row.inner;
        let Some(target) = line.target else {
            return;
        };
        let hover = match target {
            Target::Op(op) => t!(ui, "report-select-operation-hover", number = op + 1),
            Target::Tool(id) => t!(ui, "report-select-tool-hover", tool = id),
            Target::UpdateTools => t!(ui, "report-update-tools-hover"),
        };
        let response = response.on_hover_text(hover);
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if response.clicked() {
            match target {
                Target::Op(op) => self.focus = Focus::Op(op),
                Target::Tool(id) => self.focus = Focus::Tool(id),
                Target::UpdateTools => {
                    self.update_tools_from_library(None);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_naming_an_operation_link_to_it() {
        let mut report = Report::default();
        report.push(
            Tone::Good,
            "Generated 10 lines for GRBL. Check passed.".into(),
        );
        report.push_at(
            Tone::Warning,
            "Operation 2 (Outline): cuts below".into(),
            Some(1),
        );
        assert_eq!(report.lines[0].target, None);
        assert_eq!(report.lines[1].target, Some(Target::Op(1)));
        assert_eq!(report.problems(), 1);
        assert_eq!(report[1], "Operation 2 (Outline): cuts below");
    }
}
