// SPDX-License-Identifier: GPL-3.0-or-later
//! Tokenizer for G-code lines. Mirrors the Python checker's regex-based parsing exactly,
//! including Python's definitions of line breaks and whitespace.

/// Python `str.splitlines()`: splits on \n, \r\n, \r and the other Unicode line boundaries.
pub fn split_lines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        let boundary = matches!(
            c,
            '\n' | '\r'
                | '\x0b'
                | '\x0c'
                | '\x1c'
                | '\x1d'
                | '\x1e'
                | '\u{85}'
                | '\u{2028}'
                | '\u{2029}'
        );
        if !boundary {
            continue;
        }
        lines.push(&text[start..i]);
        let mut end = i + c.len_utf8();
        if c == '\r'
            && let Some(&(j, '\n')) = chars.peek()
        {
            chars.next();
            end = j + 1;
        }
        start = end;
    }
    if start < text.len() {
        lines.push(&text[start..]);
    }
    lines
}

/// Python `str.isspace()`.
fn is_space(c: char) -> bool {
    c.is_whitespace() || ('\x1c'..='\x1f').contains(&c)
}

/// Replace every `(...)` comment that contains no parentheses with a space, left to right,
/// like Python's `re.sub(r"\([^()]*\)", " ", line)`.
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
        if is_space(chars[pos]) {
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
                return Err(format!(
                    "Cannot parse {}",
                    py_str_repr(rest.trim_matches(is_space))
                ));
            }
        }
    }
    Ok(words)
}

/// `\s*([A-Za-z])\s*([+-]?(?:\d+\.?\d*|\.\d+))` anchored at `pos`.
fn match_word(chars: &[char], mut pos: usize) -> Option<(char, f64, usize)> {
    while pos < chars.len() && is_space(chars[pos]) {
        pos += 1;
    }
    let letter = *chars.get(pos)?;
    if !letter.is_ascii_alphabetic() {
        return None;
    }
    pos += 1;
    while pos < chars.len() && is_space(chars[pos]) {
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

/// Python `repr(float)` for the values that appear in messages (e.g. `0.0`, `-5.0`, `0.5`).
pub fn py_float(value: f64) -> String {
    let debug = format!("{value:?}");
    // Rust writes 1e16 and 1e-7; Python writes 1e+16 and 1e-07.
    match debug.split_once('e') {
        Some((mantissa, exp)) => {
            let (sign, digits) = match exp.strip_prefix('-') {
                Some(d) => ('-', d),
                None => ('+', exp),
            };
            format!("{mantissa}e{sign}{digits:0>2}")
        }
        None => debug,
    }
}

/// Python `repr(str)`: single quotes unless the text contains ' and no ".
fn py_str_repr(text: &str) -> String {
    let quote = if text.contains('\'') && !text.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::new();
    out.push(quote);
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_lines_like_python() {
        assert_eq!(split_lines("a\nb\r\nc\rd\x0ce"), ["a", "b", "c", "d", "e"]);
        assert_eq!(split_lines("a\n"), ["a"]);
        assert_eq!(split_lines(""), Vec::<&str>::new());
        assert_eq!(split_lines("\n\nx"), ["", "", "x"]);
    }

    #[test]
    fn strips_comments_like_python() {
        assert_eq!(strip_comments("G0 (a) X1 (b)"), "G0   X1  ");
        assert_eq!(strip_comments("((a)"), "( ");
    }

    #[test]
    fn floats_print_like_python_repr() {
        assert_eq!(py_float(0.0), "0.0");
        assert_eq!(py_float(-5.0), "-5.0");
        assert_eq!(py_float(0.5), "0.5");
        assert_eq!(py_float(1e-7), "1e-07");
        assert_eq!(py_float(1e16), "1e+16");
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
