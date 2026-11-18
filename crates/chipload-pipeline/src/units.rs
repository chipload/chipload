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

/// A length typed in `units`, in mm (docs/specs/drawing-improvements.md §2). A number
/// (`12.5`, `-3`, `.25`), a fraction or a whole number and a fraction (`1/4`, `1 1/2`), each
/// with an optional unit that wins over `units` (`mm`, `in`, `"` or `inch`); or arithmetic
/// of them: `+ - * /`, parentheses and a leading minus, `*` and `/` first (`18+0.2`,
/// `(100-6)/2`, `2*1/4in`). A length can't be multiplied by a length or divide anything.
/// None for anything else, a division by zero or a result that isn't finite.
pub fn parse_length(text: &str, units: Units) -> Option<f64> {
    let value = match evaluate(text, Some(units))? {
        // Plain numbers become a length at the end, so `0.25` gives the bits it always did.
        Value::Bare(v) => to_mm(v, units),
        Value::Length(mm) => mm,
    };
    value.is_finite().then_some(value)
}

/// A number typed with the same arithmetic as [`parse_length`] but no units (an angle, a
/// count): `360/7`, `90-12.5`.
pub fn parse_number_expr(text: &str) -> Option<f64> {
    match evaluate(text, None)? {
        Value::Bare(v) => Some(v).filter(|v| v.is_finite()),
        Value::Length(_) => None,
    }
}

/// A value while evaluating: a plain number, or a length in mm.
#[derive(Debug, Clone, Copy)]
enum Value {
    Bare(f64),
    Length(f64),
}

/// `text` evaluated; `units` are the field's (None: no units allowed, a plain number).
fn evaluate(text: &str, units: Option<Units>) -> Option<Value> {
    let tokens = tokens(text, units.is_some())?;
    let mut parser = Parser {
        tokens,
        at: 0,
        units: units.unwrap_or(Units::Mm),
    };
    let value = parser.sum()?;
    (parser.at == parser.tokens.len()).then_some(value)
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Token {
    /// A number literal, with its unit when it has one.
    Number(f64, Option<Units>),
    Plus,
    Minus,
    Times,
    Divide,
    Open,
    Close,
}

/// The tokens of `text`. A number is a decimal, a fraction `a/b` or a mixed number
/// `w a/b` of whole numbers, with spaces allowed round its parts, then optionally a unit.
fn tokens(text: &str, lengths: bool) -> Option<Vec<Token>> {
    let chars: Vec<char> = text.trim().chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    let skip_spaces = |i: &mut usize| {
        while *i < chars.len() && chars[*i].is_whitespace() {
            *i += 1;
        }
    };
    while i < chars.len() {
        let c = chars[i];
        match c {
            ' ' | '\t' => i += 1,
            '+' => {
                out.push(Token::Plus);
                i += 1;
            }
            '-' | '−' => {
                out.push(Token::Minus);
                i += 1;
            }
            '*' => {
                out.push(Token::Times);
                i += 1;
            }
            '/' => {
                out.push(Token::Divide);
                i += 1;
            }
            '(' => {
                out.push(Token::Open);
                i += 1;
            }
            ')' => {
                out.push(Token::Close);
                i += 1;
            }
            c if c.is_ascii_digit() || c == '.' => {
                let (value, next) = number_literal(&chars, i)?;
                i = next;
                // A unit right after it, a space allowed between.
                let mut j = i;
                skip_spaces(&mut j);
                let rest: String = chars[j..].iter().collect();
                let unit = [
                    ("mm", Units::Mm),
                    ("inch", Units::Inch),
                    ("in", Units::Inch),
                    ("\"", Units::Inch),
                ]
                .into_iter()
                .find(|(name, _)| rest.starts_with(name));
                let unit = match unit {
                    Some((name, unit)) => {
                        if !lengths {
                            return None;
                        }
                        i = j + name.chars().count();
                        Some(unit)
                    }
                    None => None,
                };
                out.push(Token::Number(value, unit));
            }
            _ => return None,
        }
    }
    Some(out)
}

/// The digits (and one point) at `i`: the text and where it ends.
fn digits(chars: &[char], i: usize) -> (String, usize) {
    let mut j = i;
    while j < chars.len() && (chars[j].is_ascii_digit() || chars[j] == '.') {
        j += 1;
    }
    (chars[i..j].iter().collect(), j)
}

/// A whole number at `i`: its value and where it ends.
fn whole(chars: &[char], i: usize) -> Option<(f64, usize)> {
    let (text, end) = digits(chars, i);
    (!text.is_empty() && !text.contains('.')).then_some(())?;
    Some((text.parse::<u64>().ok()? as f64, end))
}

/// A fraction `a/b` of whole numbers at `i` (spaces round the `/`): its value and end.
fn fraction(chars: &[char], i: usize) -> Option<(f64, usize)> {
    let (top, mut j) = whole(chars, i)?;
    while j < chars.len() && chars[j] == ' ' {
        j += 1;
    }
    if chars.get(j) != Some(&'/') {
        return None;
    }
    j += 1;
    while j < chars.len() && chars[j] == ' ' {
        j += 1;
    }
    let (bottom, end) = whole(chars, j)?;
    // A fraction's bottom is a whole number followed by no point: `1/2.5` is a division.
    if chars.get(end) == Some(&'.') || bottom == 0.0 {
        return None;
    }
    Some((top / bottom, end))
}

/// A number literal at `i`: a mixed number `w a/b`, a fraction `a/b`, or a decimal.
fn number_literal(chars: &[char], i: usize) -> Option<(f64, usize)> {
    // A mixed number: a whole number, spaces, then a fraction.
    if let Some((w, end)) = whole(chars, i) {
        let mut j = end;
        let spaced = j < chars.len() && chars[j] == ' ';
        while j < chars.len() && chars[j] == ' ' {
            j += 1;
        }
        if spaced && let Some((f, end)) = fraction(chars, j) {
            return Some((w + f, end));
        }
        if let Some((f, end)) = fraction(chars, i) {
            return Some((f, end));
        }
    }
    let (text, end) = digits(chars, i);
    Some((text.parse::<f64>().ok()?, end))
}

struct Parser {
    tokens: Vec<Token>,
    at: usize,
    /// The field's units, for a plain number added to a length.
    units: Units,
}

impl Parser {
    fn peek(&self) -> Option<Token> {
        self.tokens.get(self.at).copied()
    }

    /// Terms joined by `+` and `-`.
    fn sum(&mut self) -> Option<Value> {
        let mut value = self.product()?;
        while let Some(op @ (Token::Plus | Token::Minus)) = self.peek() {
            self.at += 1;
            let right = self.product()?;
            value = add(value, right, op == Token::Minus, self.units);
        }
        Some(value)
    }

    /// Factors joined by `*` and `/`.
    fn product(&mut self) -> Option<Value> {
        let mut value = self.factor()?;
        while let Some(op @ (Token::Times | Token::Divide)) = self.peek() {
            self.at += 1;
            let right = self.factor()?;
            value = if op == Token::Times {
                times(value, right)?
            } else {
                divide(value, right)?
            };
        }
        Some(value)
    }

    /// A number, a bracketed sum, or either after a minus.
    fn factor(&mut self) -> Option<Value> {
        match self.peek()? {
            Token::Minus => {
                self.at += 1;
                Some(match self.factor()? {
                    Value::Bare(v) => Value::Bare(-v),
                    Value::Length(v) => Value::Length(-v),
                })
            }
            Token::Open => {
                self.at += 1;
                let value = self.sum()?;
                (self.peek()? == Token::Close).then_some(())?;
                self.at += 1;
                Some(value)
            }
            Token::Number(v, unit) => {
                self.at += 1;
                Some(match unit {
                    None => Value::Bare(v),
                    Some(unit) => Value::Length(to_mm(v, unit)),
                })
            }
            _ => None,
        }
    }
}

/// `a + b` or `a - b`. A plain number added to a length is in the field's `units`.
fn add(a: Value, b: Value, minus: bool, units: Units) -> Value {
    let sign = if minus { -1.0 } else { 1.0 };
    let mm = |v: Value| match v {
        Value::Bare(x) => to_mm(x, units),
        Value::Length(x) => x,
    };
    match (a, b) {
        (Value::Bare(x), Value::Bare(y)) => Value::Bare(x + sign * y),
        _ => Value::Length(mm(a) + sign * mm(b)),
    }
}

/// `a * b`: a length times a plain number; two lengths make no length.
fn times(a: Value, b: Value) -> Option<Value> {
    match (a, b) {
        (Value::Bare(x), Value::Bare(y)) => Some(Value::Bare(x * y)),
        (Value::Length(x), Value::Bare(y)) | (Value::Bare(y), Value::Length(x)) => {
            Some(Value::Length(x * y))
        }
        (Value::Length(_), Value::Length(_)) => None,
    }
}

/// `a / b`: by a plain number other than zero; a length divides nothing.
fn divide(a: Value, b: Value) -> Option<Value> {
    match (a, b) {
        // 0.0 matches -0.0 as well.
        (_, Value::Bare(0.0)) => None,
        (Value::Bare(x), Value::Bare(y)) => Some(Value::Bare(x / y)),
        (Value::Length(x), Value::Bare(y)) => Some(Value::Length(x / y)),
        (_, Value::Length(_)) => None,
    }
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
        for bad in ["", "abc", "1/0", "1/", "/4", "1 1", "inf", "NaN"] {
            assert_eq!(inch(bad), None, "{bad:?}");
        }
    }

    /// docs/specs/drawing-improvements.md §2.
    #[test]
    fn typed_lengths_take_arithmetic() {
        let mm = |t: &str| parse_length(t, Units::Mm);
        let inch = |t: &str| parse_length(t, Units::Inch);
        assert_eq!(mm("18+0.2"), Some(18.2));
        assert_eq!(mm("600/4"), Some(150.0));
        assert_eq!(mm("2*6.35"), Some(12.7));
        assert_eq!(mm("(100-6)/2"), Some(47.0));
        assert_eq!(mm("100 - 6 / 2"), Some(97.0), "* and / first");
        assert_eq!(mm("-(3+2)"), Some(-5.0));
        assert_eq!(mm("1.5/2"), Some(0.75));
        assert_eq!(mm("1/2.5"), Some(0.4));
        assert_eq!(mm("10 − 4"), Some(6.0), "the minus sign");
        // Units on any number; a plain number in a sum with a length is in the field's.
        assert_eq!(mm("1in+3mm"), Some(28.4));
        assert_eq!(mm("1in+3"), Some(28.4));
        assert_eq!(inch("1in+3"), Some(25.4 + 3.0 * 25.4));
        assert_eq!(mm("2*1/4in"), Some(12.7));
        assert_eq!(inch("1/2in/2"), Some(6.35));
        assert_eq!(inch("1 1/2 + 1/2"), Some(50.8));
        // Plain numbers and fractions give the bits they always did.
        assert_eq!(inch("1/4"), Some(0.25 * 25.4));
        for bad in [
            "1in*1in", "3/(1in)", "2/(1-1)", "1+", "(1", "1)", "*2", "2 3", "1in in", "1e3",
        ] {
            assert_eq!(mm(bad), None, "{bad:?}");
        }
        // Without units: angles and counts.
        assert_eq!(parse_number_expr("360/7"), Some(360.0 / 7.0));
        assert_eq!(parse_number_expr("90-12.5"), Some(77.5));
        assert_eq!(parse_number_expr("12mm"), None);
    }

    #[test]
    fn a_quarter_inch_is_exactly_6_35_mm() {
        assert_eq!(to_mm(0.25, Units::Inch), 6.35);
        assert_eq!(length_number(6.35, Units::Inch), "0.25");
    }
}
