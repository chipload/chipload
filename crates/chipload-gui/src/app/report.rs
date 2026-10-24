// SPDX-License-Identifier: GPL-3.0-or-later
//! The report under the canvas: the last action's result line, its warnings and notes
//! (a line naming an operation selects it), and the drawing's facts.

use chipload_core::chain::Contour;
use chipload_dxf::{ImportReport, Severity};
use chipload_pipeline::report::{format_contours, format_import_report};
use eframe::egui::{self, Color32, RichText, ScrollArea, Sense};

use super::{ChiploadApp, Focus, Panel};
use crate::i18n::t;
use chipload_job::Problem;

/// Height the open report starts with.
const REPORT_HEIGHT: f32 = 190.0;

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
    pub text: String,
    /// The operation the line is about (index), if it names one.
    pub op: Option<usize>,
}

/// What the last action had to say. The first line is the headline.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Report {
    pub lines: Vec<ReportLine>,
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

    /// A failure: `head`, then every problem, each leading to its operation.
    pub fn of_problems(head: &str, problems: &[Problem]) -> Self {
        let mut report = Report::error([head.to_string()]);
        for problem in problems {
            report.push_at(Tone::Error, format!("  {problem}"), problem.operation());
        }
        report
    }

    /// A line about nothing in particular.
    pub fn push(&mut self, tone: Tone, text: String) {
        self.push_at(tone, text, None);
    }

    /// A line about operation `op` (an index), or about the project with None.
    pub fn push_at(&mut self, tone: Tone, text: String, op: Option<usize>) {
        self.lines.push(ReportLine { tone, text, op });
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
    pub lines: Vec<String>,
    pub contours: usize,
    /// Import issues; errors among them make the section red.
    pub issues: usize,
    pub errors: bool,
}

impl DrawingFacts {
    pub fn new(report: &ImportReport, contours: &[Contour]) -> Self {
        let mut lines = format_import_report(report);
        lines.extend(format_contours(contours));
        DrawingFacts {
            lines,
            contours: contours.len(),
            issues: report.issues.len(),
            errors: report.issues.iter().any(|i| i.severity == Severity::Error),
        }
    }

    /// The drawing could not be read.
    pub fn failed(message: String) -> Self {
        DrawingFacts {
            lines: vec![message],
            contours: 0,
            issues: 1,
            errors: true,
        }
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
            self.report_open = true;
        }
        self.report = report;
    }

    /// The report panel along the bottom: one line when closed.
    pub(super) fn report_panel(&mut self, ui: &mut egui::Ui) {
        if self.report_open {
            Panel::bottom("report")
                .resizable(true)
                .default_size(REPORT_HEIGHT)
                .min_size(80.0)
                .show(ui, |ui| {
                    self.report_header(ui);
                    ui.separator();
                    self.report_body(ui);
                });
        } else {
            Panel::bottom("report-bar").show(ui, |ui| self.report_header(ui));
        }
    }

    /// The toggle, the headline, and the number of problems further down.
    fn report_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let arrow = if self.report_open { "▾" } else { "▸" };
            if ui
                .selectable_label(
                    false,
                    RichText::new(format!("{arrow} {}", t!(ui, "report-heading"))).strong(),
                )
                .on_hover_text(if self.report_open {
                    t!(ui, "report-hide-hover")
                } else {
                    t!(ui, "report-show-hover")
                })
                .clicked()
            {
                self.report_open = !self.report_open;
            }
            ui.separator();
            if let Some(first) = self.report.lines.first().cloned() {
                self.report_line(ui, &first, true);
            }
            let rest = self.report.problems()
                - self.report.lines.first().map_or(0, |l| {
                    usize::from(matches!(l.tone, Tone::Warning | Tone::Error))
                });
            if rest > 0 && !self.report_open {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let text = t!(ui, "report-more-warnings", count = rest);
                    ui.label(RichText::new(text).color(Tone::Warning.color(ui.visuals())));
                });
            }
        });
    }

    fn report_body(&mut self, ui: &mut egui::Ui) {
        ScrollArea::vertical()
            .id_salt("report")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let lines: Vec<ReportLine> = self.report.lines.iter().skip(1).cloned().collect();
                for line in &lines {
                    self.report_line(ui, line, false);
                }
                if let Some(facts) = &self.drawing_facts {
                    if !lines.is_empty() {
                        ui.add_space(4.0);
                    }
                    let mut title = RichText::new(facts.title(ui));
                    if facts.issues > 0 {
                        let tone = if facts.errors {
                            Tone::Error
                        } else {
                            Tone::Warning
                        };
                        title = title.color(tone.color(ui.visuals()));
                    }
                    egui::CollapsingHeader::new(title)
                        .id_salt("drawing-facts")
                        .default_open(facts.issues > 0)
                        .show(ui, |ui| {
                            ScrollArea::horizontal()
                                .id_salt("drawing-facts-scroll")
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(crate::units::message(
                                                ui.ctx(),
                                                &facts.lines.join("\n"),
                                            ))
                                            .monospace(),
                                        )
                                        .extend(),
                                    );
                                });
                        });
                }
            });
    }

    /// One line: its mark and text in its tone; a line about an operation selects it.
    fn report_line(&mut self, ui: &mut egui::Ui, line: &ReportLine, headline: bool) {
        let color = line.tone.color(ui.visuals());
        let mut text = RichText::new(crate::units::message(ui.ctx(), &line.text)).color(color);
        if headline {
            text = text.strong();
        }
        let row = ui.horizontal_top(|ui| {
            ui.label(RichText::new(line.tone.mark()).color(color));
            let label = if headline {
                egui::Label::new(text).truncate()
            } else {
                egui::Label::new(text).wrap()
            };
            ui.add(label.sense(Sense::click()))
        });
        let response = row.inner;
        if let Some(op) = line.op {
            let response =
                response.on_hover_text(t!(ui, "report-select-operation-hover", number = op + 1));
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if response.clicked() {
                self.focus = Focus::Op(op);
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
        assert_eq!(report.lines[0].op, None);
        assert_eq!(report.lines[1].op, Some(1));
        assert_eq!(report.problems(), 1);
        assert_eq!(report[1], "Operation 2 (Outline): cuts below");
    }
}
