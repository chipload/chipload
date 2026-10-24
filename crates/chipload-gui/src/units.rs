// SPDX-License-Identifier: GPL-3.0-or-later
//! Display units (docs/specs/inch-units.md §5): the app shows and takes lengths and feeds in
//! mm or inches, as Settings > Units says. Values stay in mm; a field only formats and
//! parses. The setting rides in egui's memory each frame, so every field finds it.

use std::ops::RangeInclusive;

use chipload_job::Units;
use chipload_pipeline::units::{feed_unit, length_unit, parse_length};
use eframe::egui::{self, DragValue, Id, Response, Ui, Widget};

fn key() -> Id {
    Id::new("chipload-display-units")
}

/// Make `units` the display units for this frame's fields.
pub fn set_display(ctx: &egui::Context, units: Units) {
    ctx.data_mut(|d| d.insert_temp(key(), units));
}

/// The display units; mm until set.
pub fn display(ctx: &egui::Context) -> Units {
    ctx.data(|d| d.get_temp(key())).unwrap_or(Units::Mm)
}

/// A plain line of the report (the app's own words, or another library's, such as the
/// file system's) in the display units: its `… mm` lengths and `… mm/min` feeds as inches
/// when inches are shown (docs/specs/inch-units.md §5.1). Everything from the library
/// crates is a message, rendered in the language and units instead
/// (docs/specs/translations.md §7); this goes with `text_in` in step 7.3.6.
pub fn message(ctx: &egui::Context, text: &str) -> String {
    chipload_pipeline::units::text_in(text, display(ctx))
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Kind {
    /// mm, or inches with 4 decimals.
    Length,
    /// mm/min, or in/min with 2 decimals.
    Feed,
    /// mm², or square inches with 4 decimals.
    Area,
}

/// A field for a value in mm (or mm/min, mm²), shown and typed in the display units.
#[must_use]
pub struct Field<'a> {
    value: &'a mut f64,
    kind: Kind,
    speed: f64,
    range: Option<RangeInclusive<f64>>,
    prefix: Option<&'a str>,
    /// At most this many decimals in mm.
    max_decimals: Option<usize>,
    clamp_existing: bool,
}

/// A length field.
pub fn length(value: &mut f64) -> Field<'_> {
    Field::new(value, Kind::Length)
}

/// A feed field.
pub fn feed(value: &mut f64) -> Field<'_> {
    Field::new(value, Kind::Feed)
}

/// An area field.
pub fn area(value: &mut f64) -> Field<'_> {
    Field::new(value, Kind::Area)
}

impl<'a> Field<'a> {
    fn new(value: &'a mut f64, kind: Kind) -> Self {
        Field {
            value,
            kind,
            speed: 0.1,
            range: None,
            prefix: None,
            max_decimals: None,
            clamp_existing: true,
        }
    }

    /// Drag speed in mm (mm/min, mm²) per pixel.
    pub fn speed(mut self, speed: f64) -> Self {
        self.speed = speed;
        self
    }

    /// The allowed values, in mm (mm/min, mm²).
    pub fn range(mut self, range: RangeInclusive<f64>) -> Self {
        self.range = Some(range);
        self
    }

    pub fn prefix(mut self, prefix: &'a str) -> Self {
        self.prefix = Some(prefix);
        self
    }

    /// Keep a value outside the range as it is until it is edited.
    pub fn clamp_existing_to_range(mut self, clamp: bool) -> Self {
        self.clamp_existing = clamp;
        self
    }

    /// At most `decimals` decimals in mm (inches always show 4).
    pub fn max_decimals(mut self, decimals: usize) -> Self {
        self.max_decimals = Some(decimals);
        self
    }
}

/// A typed number with its decimal comma made a point, when the app's language writes
/// decimals with a comma (docs/specs/translations.md decision 6): `6,5` is 6.5 in German,
/// and stays unreadable in English, where `1,000` means a thousand.
pub fn decimal_point(text: &str, comma: bool) -> String {
    if comma && !text.contains('.') && text.matches(',').count() == 1 {
        text.replace(',', ".")
    } else {
        text.to_string()
    }
}

/// A number field (no unit) that also takes a decimal comma in a language that writes one.
pub fn number<'a, N: egui::emath::Numeric>(ui: &Ui, value: &'a mut N) -> DragValue<'a> {
    let comma = crate::i18n::Lang::lang(ui).decimal_comma();
    DragValue::new(value).custom_parser(move |text| {
        let text = decimal_point(text, comma);
        text.trim().replace('−', "-").parse::<f64>().ok()
    })
}

impl Widget for Field<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let units = display(ui.ctx());
        let comma = crate::i18n::Lang::lang(ui).decimal_comma();
        let kind = self.kind;
        // How many mm (mm/min, mm²) one shown unit is.
        let size = match kind {
            Kind::Length | Kind::Feed => units.mm_per_unit(),
            Kind::Area => units.mm_per_unit() * units.mm_per_unit(),
        };
        let mut field = DragValue::new(self.value).speed(self.speed);
        if let Some(range) = self.range {
            field = field
                .range(range)
                .clamp_existing_to_range(self.clamp_existing);
        }
        if let Some(prefix) = self.prefix {
            field = field.prefix(prefix);
        }
        if let Some(decimals) = self.max_decimals {
            field = field.max_decimals(decimals);
        }
        let suffix = match (kind, units) {
            (Kind::Length, _) => format!(" {}", length_unit(units)),
            (Kind::Feed, _) => format!(" {}", feed_unit(units)),
            (Kind::Area, Units::Mm) => " mm²".to_string(),
            (Kind::Area, Units::Inch) => " in²".to_string(),
        };
        field = field.suffix(suffix);
        // A typed length may carry its own unit ("1/4in" in mm, "12mm" in inches).
        field = match kind {
            Kind::Length => {
                field.custom_parser(move |text| parse_length(&decimal_point(text, comma), units))
            }
            Kind::Feed | Kind::Area => field.custom_parser(move |text| {
                let number = decimal_point(text, comma).trim().parse::<f64>().ok()?;
                number.is_finite().then_some(number * size)
            }),
        };
        if units == Units::Inch {
            let decimals = match kind {
                Kind::Length | Kind::Area => 4,
                Kind::Feed => 2,
            };
            field = field.custom_formatter(move |value, _| format!("{:.decimals$}", value / size));
        }
        ui.add(field)
    }
}

#[cfg(test)]
mod tests {
    use super::decimal_point;

    #[test]
    fn a_decimal_comma_is_read_only_in_languages_that_write_one() {
        assert_eq!(decimal_point("6,5", true), "6.5");
        assert_eq!(decimal_point("-0,25 in", true), "-0.25 in");
        assert_eq!(decimal_point("6,5", false), "6,5");
        assert_eq!(decimal_point("1,000", false), "1,000");
        // A point already there, or two commas: left as typed, so it doesn't parse.
        assert_eq!(decimal_point("1.000,5", true), "1.000,5");
        assert_eq!(decimal_point("1,0,5", true), "1,0,5");
    }
}
