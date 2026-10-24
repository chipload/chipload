// SPDX-License-Identifier: GPL-3.0-or-later
//! Lengths and feeds shown and typed in mm or inches (docs/specs/inch-units.md §5). Values
//! are always mm inside; these only turn them into text and back.

use chipload_i18n::Digits;
use chipload_job::Units;

/// Decimals a length is shown with: 3 in mm, 4 in inches.
pub fn decimals(units: Units) -> usize {
    match units {
        Units::Mm => 3,
        Units::Inch => 4,
    }
}

/// The unit's short name after a number: `mm` or `in`.
pub fn length_unit(units: Units) -> &'static str {
    match units {
        Units::Mm => "mm",
        Units::Inch => "in",
    }
}

/// The feed's unit: `mm/min` or `in/min`.
pub fn feed_unit(units: Units) -> &'static str {
    match units {
        Units::Mm => "mm/min",
        Units::Inch => "in/min",
    }
}

/// `mm` in `units`, as a number.
pub fn from_mm(mm: f64, units: Units) -> f64 {
    mm / units.mm_per_unit()
}

/// A value in `units` back in mm.
pub fn to_mm(value: f64, units: Units) -> f64 {
    value * units.mm_per_unit()
}

/// A length for reading, without its unit: `12.5`, `0.4921`.
pub fn length_number(mm: f64, units: Units) -> String {
    chipload_i18n::message::length_number(mm, Digits::Trim(3), units.into())
}

/// A length for reading, with its unit: `12.5 mm`, `0.4921 in`.
pub fn length_text(mm: f64, units: Units) -> String {
    format!("{} {}", length_number(mm, units), length_unit(units))
}

/// A feed for reading, without its unit: up to 3 decimals in mm/min, 2 in in/min.
pub fn feed_number(mm_per_min: f64, units: Units) -> String {
    chipload_i18n::message::feed_number(mm_per_min, Digits::Trim(3), units.into())
}

/// A feed for reading: `1200 mm/min`, `47.24 in/min` (two decimals in inches, whole mm).
pub fn feed_text(mm_per_min: f64, units: Units) -> String {
    format!(
        "{} {}",
        chipload_i18n::message::feed_number(mm_per_min, Digits::Trim(0), units.into()),
        feed_unit(units)
    )
}

/// A length typed in `units`, in mm. Accepts a decimal number (`12.5`, `-3`, `.25`), and in
/// inches also a fraction or a whole number and a fraction (`1/4`, `1 1/2`, `-3/8`). A unit
/// after it wins over `units`: `mm`, `in`, `"` or `inch`. None for anything else.
pub fn parse_length(text: &str, units: Units) -> Option<f64> {
    let text = text.trim();
    let (number, units) = if let Some(rest) = text.strip_suffix("mm") {
        (rest, Units::Mm)
    } else if let Some(rest) = text
        .strip_suffix("inch")
        .or_else(|| text.strip_suffix("in"))
        .or_else(|| text.strip_suffix('"'))
    {
        (rest, Units::Inch)
    } else {
        (text, units)
    };
    let value = parse_number(number.trim())?;
    value.is_finite().then(|| to_mm(value, units))
}

/// `12.5`, `1/4`, `1 1/2`, `-1 1/2`, `-3/8`.
fn parse_number(text: &str) -> Option<f64> {
    if let Ok(value) = text.parse::<f64>() {
        return Some(value);
    }
    let (negative, text) = match text.strip_prefix('-') {
        Some(rest) => (true, rest.trim_start()),
        None => (false, text),
    };
    let (whole, fraction) = match text.split_once(' ') {
        Some((whole, fraction)) => (whole.parse::<u64>().ok()? as f64, fraction.trim()),
        None => (0.0, text),
    };
    let (top, bottom) = fraction.split_once('/')?;
    let (top, bottom) = (
        top.trim().parse::<u64>().ok()? as f64,
        bottom.trim().parse::<u64>().ok()? as f64,
    );
    if bottom == 0.0 {
        return None;
    }
    let value = whole + top / bottom;
    Some(if negative { -value } else { value })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lengths_and_feeds_read_in_either_units() {
        assert_eq!(length_text(12.5, Units::Mm), "12.5 mm");
        assert_eq!(length_text(6.35, Units::Inch), "0.25 in");
        assert_eq!(length_text(12.5, Units::Inch), "0.4921 in");
        assert_eq!(feed_text(1200.0, Units::Mm), "1200 mm/min");
        assert_eq!(feed_text(1200.0, Units::Inch), "47.24 in/min");
    }

    #[test]
    fn typed_lengths_take_decimals_fractions_and_units() {
        let inch = |t: &str| parse_length(t, Units::Inch);
        assert_eq!(inch("0.25"), Some(6.35));
        assert_eq!(inch("1/4"), Some(6.35));
        assert_eq!(inch("3/8\""), Some(9.524999999999999));
        assert_eq!(inch("1 1/2"), Some(38.099999999999994));
        assert_eq!(inch("-1 1/2 in"), Some(-38.099999999999994));
        assert_eq!(inch("12 mm"), Some(12.0));
        assert_eq!(inch(".5inch"), Some(12.7));
        assert_eq!(parse_length("12.5", Units::Mm), Some(12.5));
        assert_eq!(parse_length("1in", Units::Mm), Some(25.4));
        for bad in ["", "abc", "1/0", "1/", "/4", "1 1", "1.5/2", "inf", "NaN"] {
            assert_eq!(inch(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn a_quarter_inch_is_exactly_6_35_mm() {
        assert_eq!(to_mm(0.25, Units::Inch), 6.35);
        assert_eq!(length_number(6.35, Units::Inch), "0.25");
    }
}
