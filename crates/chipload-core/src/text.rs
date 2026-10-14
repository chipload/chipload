// SPDX-License-Identifier: GPL-3.0-or-later
//! Numbers as people read them in notes, lists and forms: a fixed number of decimals with
//! the trailing zeros taken off. G-code has its own formatting in the post-processor.

/// `value` with at most `decimals` decimals and no trailing zeros: 3.5, 12, 0.125.
/// Never "-0".
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

/// A length in mm for reading: at most 3 decimals, no trailing zeros.
pub fn mm(value: f64) -> String {
    trimmed(value, 3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trailing_zeros_go_and_whole_numbers_keep_their_digits() {
        assert_eq!(mm(3.5), "3.5");
        assert_eq!(mm(12.0), "12");
        assert_eq!(mm(0.125), "0.125");
        assert_eq!(mm(100.0), "100");
        assert_eq!(mm(-0.0001), "0");
        assert_eq!(trimmed(1.25, 1), "1.2");
        assert_eq!(trimmed(1200.0, 0), "1200");
    }
}
