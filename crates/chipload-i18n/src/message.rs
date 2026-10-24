// SPDX-License-Identifier: GPL-3.0-or-later
//! Messages from the library crates (docs/specs/translations.md §7): an id and typed
//! arguments, turned into words where they are shown, in the reader's language and units.
//! Lengths travel as numbers in mm, so a message reads in inches without searching its text.

use crate::{Arg, Language, tr};

/// The units lengths and feeds are shown in. The project is always mm inside.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Units {
    #[default]
    Mm,
    Inch,
}

/// Millimetres in an inch.
const MM_PER_INCH: f64 = 25.4;

impl Units {
    fn of_mm(self, mm: f64) -> f64 {
        match self {
            Units::Mm => mm,
            Units::Inch => mm / MM_PER_INCH,
        }
    }
}

/// How a number is written in mm: at most so many decimals and no trailing zeros
/// (`12.5`), exactly so many (`0.300`), or as Rust writes it with `{}` (`0.1`, `1e-7`
/// written out, `NaN`), the shortest text that reads back as the same number. In inches
/// lengths keep 4 decimals, feeds 2, as the app shows them (docs/specs/inch-units.md §5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Digits {
    Trim(usize),
    Fixed(usize),
    Shortest,
}

impl Digits {
    fn write(self, value: f64) -> String {
        match self {
            Digits::Trim(decimals) => trimmed(value, decimals),
            Digits::Fixed(decimals) => fixed(value, decimals),
            Digits::Shortest => format!("{value}"),
        }
    }
}

/// `value` with at most `decimals` decimals and no trailing zeros: 3.5, 12, 0.125. Never
/// "-0". The same rule as `chipload_core::text::trimmed`, which this crate can't use.
pub fn trimmed(value: f64, decimals: usize) -> String {
    let text = format!("{value:.decimals$}");
    let text = if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.')
    } else {
        text.as_str()
    };
    match text {
        "-0" => "0".to_string(),
        text => text.to_string(),
    }
}

fn fixed(value: f64, decimals: usize) -> String {
    format!("{value:.decimals$}")
}

/// A length's number in `units`, without its unit: `12.5`, `0.4921`.
pub fn length_number(mm: f64, digits: Digits, units: Units) -> String {
    match units {
        Units::Mm => digits.write(mm),
        Units::Inch => trimmed(units.of_mm(mm), 4),
    }
}

/// The length unit after a number: `mm` or `in`.
pub fn length_unit(units: Units) -> &'static str {
    match units {
        Units::Mm => "mm",
        Units::Inch => "in",
    }
}

/// The feed unit after a number: `mm/min` or `in/min`.
pub fn feed_unit(units: Units) -> &'static str {
    match units {
        Units::Mm => "mm/min",
        Units::Inch => "in/min",
    }
}

/// A feed's number in `units`: as written in mm, 2 decimals at most in in/min.
pub fn feed_number(mm_per_min: f64, digits: Digits, units: Units) -> String {
    match units {
        Units::Mm => digits.write(mm_per_min),
        Units::Inch => trimmed(units.of_mm(mm_per_min), 2),
    }
}

/// An argument of a message.
#[derive(Debug, Clone)]
pub enum Value {
    /// Words that are never translated: names of layers, parts, tools, files.
    Text(String),
    /// A whole number; it also picks the plural form.
    Count(i64),
    /// A length in mm, with its unit: `12.5 mm`, `0.4921 in`.
    Length(f64, Digits),
    /// A length in mm, its number only: the first end of a range, `2–5 mm`.
    LengthNumber(f64, Digits),
    /// A feed in mm/min, with its unit: `1200 mm/min`, `47.24 in/min`.
    Feed(f64, Digits),
    /// A chip load in mm per tooth, with its unit: `0.050 mm/tooth`, `0.0020 in/tooth`.
    ChipLoad(f64, Digits),
    /// A point in mm, with its unit: `(10, 20.5) mm`.
    Point(f64, f64, Digits),
    /// A number that isn't a length: degrees, a fraction, a factor.
    Number(f64, Digits),
    /// A message inside this one, in the same language and units.
    Message(Box<Message>),
}

impl Value {
    /// A length written as `chipload_core::text::mm` writes it: at most 3 decimals.
    pub fn mm(mm: f64) -> Value {
        Value::Length(mm, Digits::Trim(3))
    }

    fn render(&self, lang: Language, units: Units) -> Arg {
        match self {
            Value::Text(text) => Arg::Text(text.clone()),
            Value::Count(n) => Arg::Count(*n),
            Value::Length(mm, digits) => Arg::Text(format!(
                "{} {}",
                length_number(*mm, *digits, units),
                length_unit(units)
            )),
            Value::LengthNumber(mm, digits) => Arg::Text(length_number(*mm, *digits, units)),
            Value::Feed(feed, digits) => Arg::Text(format!(
                "{} {}",
                feed_number(*feed, *digits, units),
                feed_unit(units)
            )),
            Value::ChipLoad(load, digits) => Arg::Text(match units {
                Units::Mm => format!("{} mm/tooth", digits.write(*load)),
                Units::Inch => format!("{:.4} in/tooth", units.of_mm(*load)),
            }),
            Value::Point(x, y, digits) => Arg::Text(format!(
                "({}, {}) {}",
                length_number(*x, *digits, units),
                length_number(*y, *digits, units),
                length_unit(units)
            )),
            Value::Number(value, digits) => Arg::Text(digits.write(*value)),
            Value::Message(message) => Arg::Text(message.render(lang, units)),
        }
    }
}

impl PartialEq for Value {
    /// Numbers compare by their bits, so a message equals itself and `Eq` holds.
    fn eq(&self, other: &Self) -> bool {
        let bits = |v: &f64| v.to_bits();
        match (self, other) {
            (Value::Text(a), Value::Text(b)) => a == b,
            (Value::Count(a), Value::Count(b)) => a == b,
            (Value::Length(a, da), Value::Length(b, db))
            | (Value::LengthNumber(a, da), Value::LengthNumber(b, db))
            | (Value::Feed(a, da), Value::Feed(b, db))
            | (Value::ChipLoad(a, da), Value::ChipLoad(b, db))
            | (Value::Number(a, da), Value::Number(b, db)) => bits(a) == bits(b) && da == db,
            (Value::Point(ax, ay, da), Value::Point(bx, by, db)) => {
                bits(ax) == bits(bx) && bits(ay) == bits(by) && da == db
            }
            (Value::Message(a), Value::Message(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for Value {}

impl From<&str> for Value {
    fn from(text: &str) -> Self {
        Value::Text(text.to_string())
    }
}

impl From<String> for Value {
    fn from(text: String) -> Self {
        Value::Text(text)
    }
}

impl From<&String> for Value {
    fn from(text: &String) -> Self {
        Value::Text(text.clone())
    }
}

impl From<Message> for Value {
    fn from(message: Message) -> Self {
        Value::Message(Box::new(message))
    }
}

macro_rules! value_count_from {
    ($($t:ty),*) => {$(
        impl From<$t> for Value {
            fn from(n: $t) -> Self {
                Value::Count(i64::try_from(n).unwrap_or(i64::MAX))
            }
        }
    )*};
}
value_count_from!(i32, i64, u32, u64, usize);

/// A message: the id of its text in `locales/<lang>/*.ftl`, and its arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub id: &'static str,
    pub args: Vec<(&'static str, Value)>,
}

impl Message {
    pub fn new(id: &'static str) -> Self {
        Message {
            id,
            args: Vec::new(),
        }
    }

    /// The message with one more argument.
    #[must_use]
    pub fn arg(mut self, name: &'static str, value: impl Into<Value>) -> Self {
        self.args.push((name, value.into()));
        self
    }

    /// The message in `lang`, its lengths and feeds in `units`.
    pub fn render(&self, lang: Language, units: Units) -> String {
        let args: Vec<(&str, Arg)> = self
            .args
            .iter()
            .map(|(name, value)| (*name, value.render(lang, units)))
            .collect();
        tr(lang, self.id, &args)
    }

    /// The message as the command line and `--json` write it: English, mm.
    pub fn english(&self) -> String {
        self.render(Language::En, Units::Mm)
    }
}
