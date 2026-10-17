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

impl Widget for Field<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let units = display(ui.ctx());
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
            Kind::Length => field.custom_parser(move |text| parse_length(text, units)),
            Kind::Feed | Kind::Area => field.custom_parser(move |text| {
                let number = text.trim().parse::<f64>().ok()?;
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
