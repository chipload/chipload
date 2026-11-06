// SPDX-License-Identifier: GPL-3.0-or-later
//! The drawing sources' forms: a DXF's placement, a picture's, a text's.

use super::*;

/// A picture's size, place and layer, as edited in its properties.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::app) struct ImagePlace {
    pub width: f64,
    /// The picture's lower-left corner before it is turned (the file's `x`, `y`).
    pub x: f64,
    pub y: f64,
    pub rotation: f64,
    pub layer: String,
}

/// An imported DXF's properties (docs/specs/properties-for-tools-and-dxfs.md §3): what it
/// is, its placement, and buttons to update or remove it.
pub(in crate::app) fn dxf_form(ui: &mut egui::Ui, edit: &EditJob, id: u64) -> Option<JobRequest> {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let drawing = &edit.job.drawing;
    let source = drawing.dxfs.iter().find(|d| d.id == id)?;
    let bbox = chipload_pipeline::placement::dxf_box(&drawing.shapes, id);
    let size = bbox.map(|((x0, y0), (x1, y1))| (x1 - x0, y1 - y0));
    let shapes = drawing
        .shapes
        .iter()
        .filter(|s| s.dxf() == Some(id))
        .count();
    let mut request = None;
    super::browser::title_line(ui, crate::icons::Icon::Dxf, None, &source.name);
    crate::app::forms::card(ui, |ui| {
        Grid::new(("dxf-facts", id))
            .num_columns(2)
            .min_col_width(label_width(ui) + ui.spacing().indent)
            .show(ui, |ui| {
                ui.label(t!(ui, "forms-file"));
                ui.label(&source.file);
                ui.end_row();
                ui.label(t!(ui, "forms-units"));
                ui.label(match source.units {
                    Units::Mm => "mm".to_string(),
                    Units::Inch => t!(ui, "forms-dxf-inch"),
                });
                ui.end_row();
                ui.label(t!(ui, "forms-shapes"));
                ui.label(shapes.to_string());
                ui.end_row();
            })
    });
    let summary = size.map_or(String::new(), |(w, h)| {
        format!("{} × {} {len_unit}", len_in(units, w), len_in(units, h))
    });
    let title = t!(ui, "forms-section-placement");
    section(ui, "dxf-place", &title, &summary, |ui| {
        placement_rows(ui, id, source.placement, bbox, &mut request);
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui
            .button(t!(ui, "forms-update-dxf"))
            .on_hover_text(t!(ui, "forms-update-dxf-hover"))
            .clicked()
        {
            request = Some(JobRequest::UpdateDxf(id));
        }
        if ui
            .button(t!(ui, "forms-explode-dxf"))
            .on_hover_text(t!(ui, "forms-explode-dxf-hover"))
            .clicked()
        {
            request = Some(JobRequest::ExplodeDxf(id));
        }
        if ui
            .button(t!(ui, "forms-remove"))
            .on_hover_text(t!(ui, "forms-remove-dxf-hover"))
            .clicked()
        {
            request = Some(JobRequest::RemoveDxf(id));
        }
    });
    request
}

/// A picture's properties (§3b): what it is, then its width, corner and layer. Each change
/// applies when the edit is finished, as one undo step, and traces the picture again.
pub(in crate::app) fn image_form(ui: &mut egui::Ui, edit: &EditJob, id: u64) -> Option<JobRequest> {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let image = edit.job.drawing.images.iter().find(|i| i.id == id)?;
    let mut request = None;
    super::browser::title_line(ui, crate::icons::Icon::Image, None, &image.file);
    // The fields edit a copy kept between frames; it is sent once the edit is finished.
    let key = egui::Id::new(("image-place", id));
    let stored = ImagePlace {
        width: image.width,
        x: image.x,
        y: image.y,
        rotation: image.rotation,
        layer: image.layer.clone(),
    };
    let mut draft: ImagePlace = ui
        .data(|d| d.get_temp::<(ImagePlace, ImagePlace)>(key))
        .filter(|(from, _)| *from == stored)
        .map_or_else(|| stored.clone(), |(_, draft)| draft);
    // The picture's size in pixels, decoded once per picture (not every frame).
    let size_key = egui::Id::new(("image-pixels", id, image.sha256.clone()));
    let pixels: Option<(u32, u32)> = match ui.data(|d| d.get_temp::<Option<(u32, u32)>>(size_key)) {
        Some(known) => known,
        None => {
            let decoded = chipload_pipeline::images::picture_bytes(image)
                .ok()
                .and_then(|bytes| chipload_pipeline::images::decode_picture(&bytes).ok())
                .map(|p| (p.width, p.height));
            ui.data_mut(|d| d.insert_temp(size_key, decoded));
            decoded
        }
    };
    let mut finished = false;
    crate::app::forms::card(ui, |ui| {
        Grid::new(("image-form", id))
            .num_columns(2)
            .min_col_width(label_width(ui) + ui.spacing().indent)
            .show(ui, |ui| {
                if let Some((w, h)) = pixels {
                    ui.label(t!(ui, "forms-picture"));
                    ui.label(t!(ui, "forms-picture-pixels", width = w, height = h));
                    ui.end_row();
                }
                ui.label(t!(ui, "forms-traced-as"));
                ui.label(match image.trace {
                    ImageTrace::Outline => t!(ui, "forms-traced-outlines"),
                    ImageTrace::CentreLine => t!(ui, "forms-traced-centre-lines"),
                });
                ui.end_row();
                // The picture's height follows from its width; its box is turned with it
                // (docs/specs/direct-placement.md §2).
                let aspect = pixels
                    .filter(|(w, _)| *w > 0)
                    .map_or(1.0, |(w, h)| f64::from(h) / f64::from(w));
                let turned = |d: &ImagePlace| {
                    chipload_pipeline::images::turned_box(
                        d.x,
                        d.y,
                        d.width,
                        d.width * aspect,
                        d.rotation,
                    )
                };
                let ((x0, y0), _) = turned(&draft);
                ui.label(t!(ui, "forms-position")).on_hover_text(t!(
                    ui,
                    "units-text-picture-position-hover",
                    unit = crate::units::unit(ui.ctx())
                ));
                // Moving the box's corner moves the picture's own corner as much.
                let hover = t!(ui, "forms-picture-reset-position");
                finished |= position_cells(ui, (x0, y0), (&mut draft.x, &mut draft.y), &hover);
                ui.end_row();
                ui.label(t!(ui, "forms-rotation"))
                    .on_hover_text(t!(ui, "forms-picture-rotation-hover"));
                // Turning keeps the picture's centre where it is: x, y stay.
                let hover = t!(ui, "forms-picture-reset-rotation");
                finished |= rotation_cells(ui, &mut draft.rotation, &hover);
                ui.end_row();
                ui.label(t!(ui, "forms-width"))
                    .on_hover_text(t!(ui, "forms-picture-width-hover"));
                ui.horizontal(|ui| {
                    let response = ui.add(
                        crate::units::length(&mut draft.width)
                            .range(0.1..=10_000.0)
                            .speed(0.5),
                    );
                    finished |= response.drag_stopped() || response.lost_focus();
                    if pixels.is_some() {
                        let height = format!("{} {len_unit}", len_in(units, draft.width * aspect));
                        ui.weak(t!(ui, "forms-picture-height", height = height));
                    }
                });
                ui.end_row();
                ui.label(t!(ui, "forms-layer"))
                    .on_hover_text(t!(ui, "forms-picture-layer-hover"));
                let response = ui.add(
                    TextEdit::singleline(&mut draft.layer).desired_width(field_width(ui, 160.0)),
                );
                finished |= response.lost_focus();
                ui.end_row();
            })
    });
    if finished && draft != stored {
        request = Some(JobRequest::ImagePlace(id, draft.clone()));
    }
    ui.data_mut(|d| d.insert_temp(key, (stored, draft)));
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui
            .button(t!(ui, "forms-trace-settings"))
            .on_hover_text(t!(ui, "forms-trace-settings-hover"))
            .clicked()
        {
            request = Some(JobRequest::TraceSettings(id));
        }
        if ui
            .button(t!(ui, "forms-remove"))
            .on_hover_text(t!(ui, "forms-remove-picture-hover"))
            .clicked()
        {
            request = Some(JobRequest::RemoveImage(id));
        }
    });
    request
}

/// A text's properties (docs/specs/text.md §5): what it says, its font and size, its place
/// and turn, how its lines line up, its layer. Each change applies when the edit is
/// finished, as one undo step, and writes the letters again. `grab`: the text field takes
/// the keyboard (a text just placed).
pub(in crate::app) fn text_form(
    ui: &mut egui::Ui,
    edit: &EditJob,
    id: u64,
    grab: bool,
    fonts: &super::text_fonts::TextFonts,
) -> Option<JobRequest> {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    use chipload_job::{Text, TextAlign, TextAnchor};
    let text = edit.job.drawing.texts.iter().find(|t| t.id == id)?;
    let mut request = None;
    super::browser::title_line(
        ui,
        crate::icons::Icon::Text,
        None,
        &chipload_pipeline::texts::text_label(text),
    );
    // The fields edit a copy kept between frames; it is sent once the edit is finished.
    let key = egui::Id::new(("text-draft", id));
    let stored = text.clone();
    let mut draft: Text = ui
        .data(|d| d.get_temp::<(Text, Text)>(key))
        .filter(|(from, _)| *from == stored)
        .map_or_else(|| stored.clone(), |(_, draft)| draft);
    // The box's size before its turn, worked out once per version of the text.
    let size_key = egui::Id::new(("text-size", id));
    let size = match ui
        .data(|d| d.get_temp::<(Text, Option<(f64, f64)>)>(size_key))
        .filter(|(t, _)| *t == stored)
    {
        Some((_, size)) => size,
        // Not worked out while the computer's fonts are still looked for.
        None if fonts.has(&stored.font).is_none() => None,
        None => {
            let size = chipload_pipeline::texts::text_size(&edit.job, &stored, &*fonts.finder).ok();
            ui.data_mut(|d| d.insert_temp(size_key, (stored.clone(), size)));
            size
        }
    };
    // Without its font, only what moves the stored letters can change
    // (docs/specs/system-fonts.md §5).
    let here = fonts.has(&stored.font);
    let letters_editable = here == Some(true);
    match here {
        Some(true) => {}
        Some(false) => {
            let note = t!(ui, "forms-text-font-missing", font = stored.font.as_str());
            ui.add(egui::Label::new(RichText::new(note).color(ui.visuals().warn_fg_color)).wrap());
        }
        None => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.weak(t!(ui, "forms-fonts-looking"));
            });
        }
    }
    if here == Some(true) && chipload_text::builtin(&stored.font).is_none() {
        super::text_fonts::load_sample(ui.ctx(), &fonts.finder, &stored.font);
    }
    let mut finished = false;
    crate::app::forms::card(ui, |ui| {
        Grid::new(("text-form", id))
            .num_columns(2)
            .min_col_width(label_width(ui) + ui.spacing().indent)
            .show(ui, |ui| {
                ui.label(t!(ui, "forms-text"))
                    .on_hover_text(t!(ui, "forms-text-hover"));
                let response = ui.add_enabled(
                    letters_editable,
                    TextEdit::multiline(&mut draft.text)
                        .desired_rows(2)
                        .desired_width(field_width(ui, 220.0)),
                );
                if grab {
                    response.request_focus();
                }
                finished |= response.lost_focus();
                ui.end_row();
                ui.label(t!(ui, "forms-font"))
                    .on_hover_text(t!(ui, "forms-font-hover"));
                let before = draft.font.clone();
                let mut shown = RichText::new(draft.font.as_str());
                let family = super::text_fonts::sample_family(&draft.font);
                if ui.ctx().fonts(|f| f.families()).contains(&family) {
                    shown = shown.family(family);
                }
                egui::ComboBox::from_id_salt(("text-font", id))
                    .selected_text(shown)
                    .height(480.0)
                    .show_ui(ui, |ui| font_list(ui, &mut draft.font, fonts));
                finished |= draft.font != before;
                ui.end_row();
                ui.label(t!(ui, "forms-height"))
                    .on_hover_text(t!(ui, "forms-text-height-hover"));
                let response = ui.add_enabled(
                    letters_editable,
                    crate::units::length(&mut draft.height)
                        .range(0.1..=chipload_job::TEXT_MAX_HEIGHT_MM)
                        .speed(0.1),
                );
                finished |= response.drag_stopped() || response.lost_focus();
                ui.end_row();
                let (w, h) = size.unwrap_or((0.0, 0.0));
                ui.label(t!(ui, "forms-anchor"))
                    .on_hover_text(t!(ui, "forms-anchor-hover"));
                let before = draft.anchor;
                let mut anchor = draft.anchor;
                egui::ComboBox::from_id_salt(("text-anchor", id))
                    .selected_text(match anchor {
                        TextAnchor::Corner => t!(ui, "forms-anchor-corner"),
                        TextAnchor::Center => t!(ui, "forms-anchor-centre"),
                    })
                    .show_ui(ui, |ui| {
                        let corner = t!(ui, "forms-anchor-corner");
                        ui.selectable_value(&mut anchor, TextAnchor::Corner, corner);
                        let centre = t!(ui, "forms-anchor-centre");
                        ui.selectable_value(&mut anchor, TextAnchor::Center, centre);
                    });
                if anchor != before {
                    // The text stays where it is: only what Position means changes.
                    draft = chipload_pipeline::texts::with_anchor(&draft, anchor, (w, h));
                    finished = true;
                }
                ui.end_row();
                // Shown as the box's lower-left corner on the stock (after the turn), or its
                // centre, which the turn keeps in place.
                let shown = |d: &Text| match d.anchor {
                    TextAnchor::Center => (d.x, d.y),
                    TextAnchor::Corner => {
                        chipload_pipeline::images::turned_box(d.x, d.y, w, h, d.rotation).0
                    }
                };
                let (x0, y0) = shown(&draft);
                ui.label(t!(ui, "forms-position"))
                    .on_hover_text(match draft.anchor {
                        TextAnchor::Corner => t!(
                            ui,
                            "units-text-text-position-corner-hover",
                            unit = crate::units::unit(ui.ctx())
                        ),
                        TextAnchor::Center => t!(
                            ui,
                            "units-text-text-position-centre-hover",
                            unit = crate::units::unit(ui.ctx())
                        ),
                    });
                // Moving what is shown moves the text's own position as much.
                let hover = t!(ui, "forms-text-reset-position");
                finished |= position_cells(ui, (x0, y0), (&mut draft.x, &mut draft.y), &hover);
                ui.end_row();
                ui.label(t!(ui, "forms-rotation"))
                    .on_hover_text(t!(ui, "forms-text-rotation-hover"));
                let hover = t!(ui, "forms-text-reset-rotation");
                finished |= rotation_cells(ui, &mut draft.rotation, &hover);
                ui.end_row();
                ui.label(t!(ui, "forms-align"))
                    .on_hover_text(t!(ui, "forms-align-hover"));
                let before = draft.align;
                let align_name = |ui: &egui::Ui, align: TextAlign| match align {
                    TextAlign::Left => t!(ui, "forms-align-left"),
                    TextAlign::Center => t!(ui, "forms-align-centre"),
                    TextAlign::Right => t!(ui, "forms-align-right"),
                };
                ui.add_enabled_ui(letters_editable, |ui| {
                    egui::ComboBox::from_id_salt(("text-align", id))
                        .selected_text(align_name(ui, draft.align))
                        .show_ui(ui, |ui| {
                            for align in [TextAlign::Left, TextAlign::Center, TextAlign::Right] {
                                ui.selectable_value(&mut draft.align, align, align_name(ui, align));
                            }
                        });
                });
                finished |= draft.align != before;
                ui.end_row();
                ui.label(t!(ui, "forms-letter-spacing"))
                    .on_hover_text(t!(ui, "forms-letter-spacing-hover"));
                let most = chipload_job::TEXT_MAX_SPACING_HEIGHTS * draft.height;
                let response = ui.add_enabled(
                    letters_editable,
                    crate::units::length(&mut draft.spacing)
                        .range(-most..=most)
                        .speed(0.05),
                );
                finished |= response.drag_stopped() || response.lost_focus();
                ui.end_row();
                ui.label(t!(ui, "forms-line-spacing"))
                    .on_hover_text(t!(ui, "forms-line-spacing-hover"));
                let response = ui.add_enabled(
                    letters_editable,
                    crate::units::number(ui, &mut draft.line_spacing)
                        .range(0.1..=chipload_job::TEXT_MAX_LINE_SPACING)
                        .speed(0.01)
                        .suffix(format!(" {}", t!(ui, "forms-line-spacing-suffix"))),
                );
                finished |= response.drag_stopped() || response.lost_focus();
                ui.end_row();
                ui.label(t!(ui, "forms-layer"))
                    .on_hover_text(t!(ui, "forms-text-layer-hover"));
                let response = ui.add(
                    TextEdit::singleline(&mut draft.layer).desired_width(field_width(ui, 160.0)),
                );
                finished |= response.lost_focus();
                ui.end_row();
                if let Some((w, h)) = size {
                    ui.label(t!(ui, "forms-size"));
                    ui.weak(format!(
                        "{} × {} {len_unit}",
                        len_in(units, w),
                        len_in(units, h)
                    ));
                    ui.end_row();
                }
            })
    });
    if finished && draft != stored {
        request = Some(JobRequest::TextChange(id, Box::new(draft.clone())));
    }
    ui.data_mut(|d| d.insert_temp(key, (stored, draft)));
    ui.add_space(4.0);
    if ui
        .button(t!(ui, "forms-remove"))
        .on_hover_text(t!(ui, "forms-remove-text-hover"))
        .clicked()
    {
        request = Some(JobRequest::RemoveText(id));
    }
    request
}

/// Margin the *Bottom left* button leaves, mm.
pub(super) const PLACE_MARGIN: f64 = 10.0;

/// A DXF's position, rotation and scale (docs/specs/direct-placement.md §2). Position is
/// the lower-left corner of its shapes' box; rotation and scale are from the DXF as drawn
/// and change about the box's centre. Each change is one undo step.
pub(in crate::app) fn placement_rows(
    ui: &mut egui::Ui,
    dxf: u64,
    placement: Placement,
    bbox: Option<((f64, f64), (f64, f64))>,
    request: &mut Option<JobRequest>,
) {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let mut place = |how: PlaceMove| *request = Some(JobRequest::Place(dxf, how));
    let salt = |what: &str| egui::Id::new(("dxf-place", dxf, what.to_string()));
    if let Some(((x0, y0), _)) = bbox {
        ui.label(t!(ui, "forms-position")).on_hover_text(t!(
            ui,
            "units-text-dxf-position-hover",
            unit = crate::units::unit(ui.ctx())
        ));
        ui.horizontal(|ui| {
            share_row(ui, 2, 30.0);
            if let Some(x) =
                finished_number(ui, salt("x"), x0, |ui, v| ui.add(mm_field(v).prefix("X ")))
            {
                place(PlaceMove::Corner(x, y0));
            }
            if let Some(y) =
                finished_number(ui, salt("y"), y0, |ui, v| ui.add(mm_field(v).prefix("Y ")))
            {
                place(PlaceMove::Corner(x0, y));
            }
            let moved = (placement.dx, placement.dy) != (0.0, 0.0);
            let hover = t!(ui, "forms-dxf-reset-position");
            if reset_button(ui, moved, &hover) {
                place(PlaceMove::To(0.0, 0.0));
            }
        });
        ui.end_row();
        ui.label("");
        ui.horizontal(|ui| {
            if ui
                .small_button(t!(ui, "forms-bottom-left"))
                .on_hover_text(t!(
                    ui,
                    "forms-bottom-left-hover",
                    margin = format!("{PLACE_MARGIN} mm")
                ))
                .clicked()
            {
                place(PlaceMove::BottomLeft {
                    margin: PLACE_MARGIN,
                });
            }
            if ui
                .small_button(t!(ui, "forms-centre"))
                .on_hover_text(t!(ui, "forms-centre-hover"))
                .clicked()
            {
                place(PlaceMove::Center);
            }
        });
        ui.end_row();
    }
    ui.label(t!(ui, "forms-rotation"))
        .on_hover_text(t!(ui, "forms-dxf-rotation-hover"));
    ui.horizontal(|ui| {
        share_row(ui, 1, 100.0);
        let current = placement.rotation;
        if let Some(degrees) = finished_number(ui, salt("rotation"), current, |ui, v| {
            ui.add(
                crate::units::number(ui, v)
                    .speed(1.0)
                    .suffix("°")
                    .max_decimals(3),
            )
        }) {
            place(PlaceMove::Rotate(degrees - current));
        }
        if let Some(by) = quarter_turn_buttons(ui) {
            place(PlaceMove::Rotate(by));
        }
        let hover = t!(ui, "forms-dxf-reset-rotation");
        if reset_button(ui, current != 0.0, &hover) {
            place(PlaceMove::Rotate(-current));
        }
    });
    ui.end_row();
    ui.label(t!(ui, "forms-scale"))
        .on_hover_text(t!(ui, "forms-scale-hover"));
    ui.horizontal(|ui| {
        let current = placement.scale;
        if let Some(scale) = finished_number(ui, salt("scale"), current, |ui, v| {
            ui.add(
                crate::units::number(ui, v)
                    .speed(0.01)
                    .range(Placement::MIN_SCALE..=Placement::MAX_SCALE)
                    .max_decimals(6)
                    .prefix("× "),
            )
        }) {
            place(PlaceMove::Scale(scale / current));
        }
        let hover = t!(ui, "forms-dxf-reset-scale");
        if reset_button(ui, current != 1.0, &hover) {
            place(PlaceMove::Scale(1.0 / current));
        }
    });
    ui.end_row();
    if let Some(((x0, y0), (x1, y1))) = bbox {
        ui.label(t!(ui, "forms-size"));
        ui.label(format!(
            "{} × {} {len_unit}",
            len_in(units, x1 - x0),
            len_in(units, y1 - y0)
        ));
        ui.end_row();
    }
}

/// The built-in fonts by style, each name in its own face where the app loaded it
/// (docs/specs/more-fonts.md §5).
/// The font list (docs/specs/system-fonts.md §6): the built-in fonts by style, each in its
/// own face, then the computer's by name with a search field; the one under the mouse
/// shows a sample.
pub(super) fn font_list(
    ui: &mut egui::Ui,
    chosen: &mut String,
    fonts: &super::text_fonts::TextFonts,
) {
    use chipload_text::{FONTS, Style};
    let loaded = ui.ctx().fonts(|f| f.families());
    ui.data_mut(|d| d.insert_temp(egui::Id::new(super::text_fonts::FONTS_WANTED), true));
    ui.label(RichText::new(t!(ui, "forms-font-builtin")).strong());
    for (style, heading) in [
        (Style::Plain, t!(ui, "forms-font-plain")),
        (Style::Script, t!(ui, "forms-font-script")),
        (Style::Display, t!(ui, "forms-font-display")),
    ] {
        ui.label(RichText::new(heading).small().weak());
        for font in FONTS.iter().filter(|f| f.style == style) {
            let family = crate::fonts::text_font_family(font.name);
            let mut text = RichText::new(font.name).size(18.0);
            if loaded.contains(&family) {
                text = text.family(family);
            }
            ui.selectable_value(chosen, font.name.to_string(), text);
        }
    }
    ui.separator();
    ui.label(RichText::new(t!(ui, "forms-font-computer")).strong());
    let Some(names) = &fonts.names else {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.weak(t!(ui, "forms-fonts-looking"));
        });
        return;
    };
    if names.is_empty() {
        ui.weak(t!(ui, "forms-font-computer-none"));
        return;
    }
    let search_id = egui::Id::new("font-search");
    let mut search: String = ui.data(|d| d.get_temp(search_id)).unwrap_or_default();
    ui.add(
        TextEdit::singleline(&mut search)
            .hint_text(t!(ui, "forms-font-search"))
            .desired_width(200.0),
    );
    ui.data_mut(|d| d.insert_temp(search_id, search.clone()));
    let wanted = search.to_lowercase();
    for name in names
        .iter()
        .filter(|n| wanted.is_empty() || n.to_lowercase().contains(&wanted))
    {
        let response = ui.selectable_value(chosen, name.clone(), name.as_str());
        if response.hovered() {
            super::text_fonts::load_sample(ui.ctx(), &fonts.finder, name);
            let family = super::text_fonts::sample_family(name);
            if loaded.contains(&family) {
                response.on_hover_ui(|ui| {
                    ui.label(RichText::new("AaBbCc 123").family(family).size(28.0));
                });
            }
        }
    }
}
