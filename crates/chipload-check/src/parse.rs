// SPDX-License-Identifier: GPL-3.0-or-later
//! Tokenizer for G-code lines.

/// Lines separated by \n, \r\n or a lone \r (controllers end a line at either).
pub fn split_lines(text: &str) -> Vec<&str> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut lines: Vec<&str> = text.split('\n').collect();
    if text.ends_with('\n') {
        lines.pop();
    }
    lines
        .into_iter()
        .flat_map(|line| {
            let line = line.strip_suffix('\r').unwrap_or(line);
            line.split('\r')
        })
        .collect()
}

/// Replace every `(...)` comment (which can't contain parentheses) with a space, left to
/// right. Anything left over is unbalanced or nested.
fn strip_comments(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '(' {
            let close = chars[i + 1..].iter().position(|&c| c == '(' || c == ')');
            if let Some(offset) = close
                && chars[i + 1 + offset] == ')'
            {
                out.push(' ');
                i += offset + 2;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// Words of one line as (upper-case letter, value), or the parse problem.
pub fn parse_line(raw: &str) -> Result<Vec<(char, f64)>, String> {
    let before_semicolon = raw.split(';').next().unwrap_or("");
    let line = strip_comments(before_semicolon);
    if line.contains('(') || line.contains(')') {
        return Err("Unbalanced or nested comment".into());
    }
    let chars: Vec<char> = line.chars().collect();
    let mut words = Vec::new();
    let mut pos = 0;
    while pos < chars.len() {
        if chars[pos].is_whitespace() {
            pos += 1;
            continue;
        }
        match match_word(&chars, pos) {
            Some((letter, value, end)) => {
                words.push((letter.to_ascii_uppercase(), value));
                pos = end;
            }
            None => {
                let rest: String = chars[pos..].iter().collect();
                return Err(format!("Cannot parse {:?}", rest.trim()));
            }
        }
    }
    Ok(words)
}

/// `\s*([A-Za-z])\s*([+-]?(?:\d+\.?\d*|\.\d+))` anchored at `pos`.
fn match_word(chars: &[char], mut pos: usize) -> Option<(char, f64, usize)> {
    while pos < chars.len() && chars[pos].is_whitespace() {
        pos += 1;
    }
    let letter = *chars.get(pos)?;
    if !letter.is_ascii_alphabetic() {
        return None;
    }
    pos += 1;
    while pos < chars.len() && chars[pos].is_whitespace() {
        pos += 1;
    }
    let start = pos;
    if matches!(chars.get(pos), Some('+' | '-')) {
        pos += 1;
    }
    let digits = |p: &mut usize| {
        let from = *p;
        while *p < chars.len() && chars[*p].is_ascii_digit() {
            *p += 1;
        }
        *p > from
    };
    if digits(&mut pos) {
        if chars.get(pos) == Some(&'.') {
            pos += 1;
            digits(&mut pos);
        }
    } else if chars.get(pos) == Some(&'.') {
        pos += 1;
        if !digits(&mut pos) {
            return None;
        }
    } else {
        return None;
    }
    let text: String = chars[start..pos].iter().collect();
    let value = text.parse::<f64>().ok()?;
    Some((letter, value, pos))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_lines_at_any_line_end() {
        assert_eq!(split_lines("a\nb\r\nc\rd"), ["a", "b", "c", "d"]);
        assert_eq!(split_lines("a\n"), ["a"]);
        assert_eq!(split_lines("a\r\n"), ["a"]);
        assert_eq!(split_lines(""), Vec::<&str>::new());
        assert_eq!(split_lines("\n\nx"), ["", "", "x"]);
    }

    #[test]
    fn strips_comments() {
        assert_eq!(strip_comments("G0 (a) X1 (b)"), "G0   X1  ");
        assert_eq!(strip_comments("((a)"), "( ");
    }

    #[test]
    fn unparseable_text_is_quoted_in_the_message() {
        assert_eq!(
            parse_line("G1 X1 ?Y2").unwrap_err(),
            r#"Cannot parse "?Y2""#
        );
    }

    #[test]
    fn number_forms() {
        let words = parse_line("G1 X1. Y.5 Z-.25 F+600").unwrap();
        assert_eq!(
            words,
            [
                ('G', 1.0),
                ('X', 1.0),
                ('Y', 0.5),
                ('Z', -0.25),
                ('F', 600.0)
            ]
        );
    }
}
