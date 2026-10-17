// SPDX-License-Identifier: GPL-3.0-or-later
//! Lengths and feeds shown and typed in mm or inches (docs/specs/inch-units.md §5). Values
//! are always mm inside; these only turn them into text and back.

use chipload_core::text::trimmed;
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
    trimmed(from_mm(mm, units), decimals(units))
}

/// A length for reading, with its unit: `12.5 mm`, `0.4921 in`.
pub fn length_text(mm: f64, units: Units) -> String {
    format!("{} {}", length_number(mm, units), length_unit(units))
}

/// A feed for reading, without its unit: up to 3 decimals in mm/min, 2 in in/min.
pub fn feed_number(mm_per_min: f64, units: Units) -> String {
    let places = match units {
        Units::Mm => 3,
        Units::Inch => 2,
    };
    trimmed(from_mm(mm_per_min, units), places)
}

/// A feed for reading: `1200 mm/min`, `47.24 in/min` (two decimals in inches, whole mm).
pub fn feed_text(mm_per_min: f64, units: Units) -> String {
    let places = match units {
        Units::Mm => 0,
        Units::Inch => 2,
    };
    format!(
        "{} {}",
        trimmed(from_mm(mm_per_min, units), places),
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

/// A message from the library crates (problems, notes, the report, the checker) in `units`
/// (docs/specs/inch-units.md §5.1). Those write every length as a number followed by ` mm`,
/// every feed by ` mm/min` and every chip load by ` mm/tooth`; in inches each becomes its
/// inch text, and a range written `A–B mm` or `A to B mm` has both ends converted. In mm the
/// text is returned as it is.
pub fn text_in(text: &str, units: Units) -> String {
    if units == Units::Mm {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some((before, number, unit, after)) = next_measure(rest) {
        let convert = |n: f64| match unit {
            Measure::Length => length_number(n, units),
            Measure::Feed => trimmed(from_mm(n, units), 2),
            Measure::ChipLoad => format!("{:.4}", from_mm(n, units)),
        };
        match number {
            Number::One(value) => {
                // A range's first end: `A–`, `A to ` or `A and ` right before the number.
                let (head, first) = range_start(before);
                out.push_str(head);
                if let Some((first, joint)) = first {
                    out.push_str(&convert(first));
                    out.push_str(joint);
                }
                out.push_str(&convert(value));
            }
            Number::Pair(x, y) => {
                out.push_str(before);
                out.push_str(&format!("({}, {})", convert(x), convert(y)));
            }
        }
        out.push(' ');
        out.push_str(match unit {
            Measure::Length => length_unit(units),
            Measure::Feed => feed_unit(units),
            Measure::ChipLoad => "in/tooth",
        });
        rest = after;
    }
    out.push_str(rest);
    out
}

/// A number before a unit, or a point `(x, y)` before it.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Number {
    One(f64),
    Pair(f64, f64),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Measure {
    Length,
    Feed,
    ChipLoad,
}

/// The next `<number> mm`, `<number> mm/min` or `<number> mm/tooth` in `text`, or a point
/// `(x, y) mm`: the text before the number, the number, what it measures, and the text
/// after the unit.
fn next_measure(text: &str) -> Option<(&str, Number, Measure, &str)> {
    let mut from = 0;
    while let Some(found) = text[from..].find(" mm") {
        let at = from + found;
        let after = &text[at + 3..];
        let (unit, len) = if after.starts_with("/min") {
            (Measure::Feed, 4)
        } else if after.starts_with("/tooth") {
            (Measure::ChipLoad, 6)
        } else {
            (Measure::Length, 0)
        };
        let tail = &after[len..];
        // A whole word: "mm" not followed by a letter or digit ("mm²" and "mmx" aren't).
        let whole = tail
            .chars()
            .next()
            .is_none_or(|c| !(c.is_alphanumeric() || c == '²' || c == '³'));
        if whole {
            let head = &text[..at];
            if let Some(start) = number_start(head)
                && let Ok(value) = head[start..].parse::<f64>()
            {
                return Some((&text[..start], Number::One(value), unit, tail));
            }
            if let Some((start, x, y)) = point_at_end(head) {
                return Some((&text[..start], Number::Pair(x, y), unit, tail));
            }
        }
        from = at + 3;
    }
    None
}

/// Where the number that ends `text` starts: digits with at most one point, and a minus
/// sign before them. None if `text` doesn't end in a number.
fn number_start(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut start = bytes.len();
    let mut points = 0;
    while start > 0 {
        match bytes[start - 1] {
            b'0'..=b'9' => start -= 1,
            b'.' if points == 0 => {
                points += 1;
                start -= 1;
            }
            _ => break,
        }
    }
    if start == bytes.len() || !text[start..].bytes().any(|b| b.is_ascii_digit()) {
        return None;
    }
    if start > 0 && bytes[start - 1] == b'-' {
        start -= 1;
    }
    Some(start)
}

/// A point `(x, y)` that ends `text`: where it starts, and its numbers.
fn point_at_end(text: &str) -> Option<(usize, f64, f64)> {
    let inner = text.strip_suffix(')')?;
    let open = inner.rfind('(')?;
    let (x, y) = inner[open + 1..].split_once(", ")?;
    Some((open, x.trim().parse().ok()?, y.trim().parse().ok()?))
}

/// `text` split before a range's first end, `A–`, `A to ` or `A and `: the text before it,
/// and the value with what joins it to the second end. `text` whole and None when it
/// doesn't end so.
fn range_start(text: &str) -> (&str, Option<(f64, &str)>) {
    for joint in ["–", " to ", " and "] {
        if let Some(head) = text.strip_suffix(joint)
            && let Some(start) = number_start(head)
            && let Ok(value) = head[start..].parse::<f64>()
        {
            return (&text[..start], Some((value, joint)));
        }
    }
    (text, None)
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

    #[test]
    fn messages_follow_the_display_units() {
        let text = "end depth 12.7 mm is 0.5 mm below the stock (25.4 mm thick)";
        assert_eq!(text_in(text, Units::Mm), text);
        assert_eq!(
            text_in(text, Units::Inch),
            "end depth 0.5 in is 0.0197 in below the stock (1 in thick)"
        );
        assert_eq!(
            text_in("feed 2540 mm/min exceeds max_feed 1270 mm/min", Units::Inch),
            "feed 100 in/min exceeds max_feed 50 in/min"
        );
        assert_eq!(
            text_in(
                "often take 0.05–0.254 mm/tooth, aluminium 0.02 to 0.08 mm/tooth",
                Units::Inch
            ),
            "often take 0.0020–0.0100 in/tooth, aluminium 0.0008 to 0.0031 in/tooth"
        );
        assert_eq!(
            text_in("from 6.35 to 12.7 mm, Z-1.27 mm", Units::Inch),
            "from 0.25 to 0.5 in, Z-0.05 in"
        );
        assert_eq!(
            text_in(
                "corner at (25.4, -12.7) mm: between 2.54 and 25.4 mm",
                Units::Inch
            ),
            "corner at (1, -0.5) in: between 0.1 and 1 in"
        );
        // Not lengths: no number before, a longer word, other units.
        for same in [
            "mm only",
            "12 mmx",
            "3 cm³ removed",
            "tool 3 (6 mm2)",
            "a mm",
        ] {
            assert_eq!(text_in(same, Units::Inch), same);
        }
    }
}
