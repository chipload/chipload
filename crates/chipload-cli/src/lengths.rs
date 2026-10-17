// SPDX-License-Identifier: GPL-3.0-or-later
//! Lengths and feeds typed on the command line (docs/specs/inch-units.md §5.3): a number is
//! mm (mm/min), as always; a number with a unit is converted: `0.25in`, `1/4in`, `3/8"`,
//! `6.35mm`; feeds `50in/min`, `1270mm/min`.

use chipload_job::Units;
use chipload_pipeline::units::{parse_length, to_mm};

/// A length in mm: `6`, `6mm`, `0.25in`, `1/4in`, `1 1/2"`.
pub fn length(text: &str) -> Result<f64, String> {
    parse_length(text, Units::Mm).ok_or_else(|| {
        format!("expected a length in mm, or with a unit (0.25in, 1/4in, 6mm), got {text:?}")
    })
}

/// A feed in mm/min: `1200`, `1200mm/min`, `47.24in/min`.
pub fn feed(text: &str) -> Result<f64, String> {
    let text = text.trim();
    let (number, units) = if let Some(rest) = text.strip_suffix("in/min") {
        (rest, Units::Inch)
    } else {
        (text.strip_suffix("mm/min").unwrap_or(text), Units::Mm)
    };
    number
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .map(|v| to_mm(v, units))
        .ok_or_else(|| {
            format!("expected a feed in mm/min, or with a unit (47in/min), got {text:?}")
        })
}

/// A point "X,Y", each a length.
pub fn point(text: &str) -> Result<[f64; 2], String> {
    let parsed = text
        .split_once(',')
        .and_then(|(x, y)| Some([length(x).ok()?, length(y).ok()?]));
    parsed.ok_or_else(|| format!("expected a point as X,Y (mm, or with a unit), got {text:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_mm_and_units_convert() {
        assert_eq!(length("6"), Ok(6.0));
        assert_eq!(length("-2.5"), Ok(-2.5));
        assert_eq!(length("1/4in"), Ok(6.35));
        assert_eq!(length("0.25\""), Ok(6.35));
        assert!(length("1/4").is_ok(), "a fraction is a fraction of a mm");
        assert!(length("6 cm").is_err());
        assert_eq!(feed("1200"), Ok(1200.0));
        assert_eq!(feed("50in/min"), Ok(1270.0));
        assert_eq!(feed("1270 mm/min"), Ok(1270.0));
        assert!(feed("fast").is_err());
        assert_eq!(point("1in,-2"), Ok([25.4, -2.0]));
        assert!(point("1in").is_err());
    }
}
