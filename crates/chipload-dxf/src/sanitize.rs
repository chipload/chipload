// SPDX-License-Identifier: GPL-3.0-or-later
//! Remove parts of an ASCII DXF that we never use and that the `dxf` crate can't parse
//! safely.
//!
//! dxf 0.6.2 builds every local-time date from the epoch 1900-01-01 00:00 in the machine's
//! time zone and unwraps it. Where that instant is ambiguous (e.g. Europe/Stockholm, which
//! switched from local mean time to CET then) any file containing such a date panics. The
//! local-time dates are `$TDCREATE`/`$TDUPDATE` in the header and timestamps in the OBJECTS
//! section; neither affects geometry, so both are dropped before parsing.

/// Header variables parsed as local time.
const LOCAL_DATE_VARIABLES: [&[u8]; 2] = [b"$TDCREATE", b"$TDUPDATE"];

/// Binary DXF starts with this sentinel; it is passed through unchanged.
const BINARY_SENTINEL: &[u8] = b"AutoCAD Binary DXF";

fn trim(line: &[u8]) -> &[u8] {
    line.trim_ascii()
}

/// The file with OBJECTS and local-time header dates removed. Line endings and every other
/// byte are kept as they were.
pub fn strip_unused(bytes: &[u8]) -> Vec<u8> {
    if bytes.starts_with(BINARY_SENTINEL) {
        return bytes.to_vec();
    }
    let lines: Vec<&[u8]> = bytes.split_inclusive(|&b| b == b'\n').collect();
    let mut out = Vec::with_capacity(bytes.len());
    let mut section: &[u8] = b"";
    let mut skipping_variable = false;
    let mut i = 0;
    while i + 1 < lines.len() {
        let (code, value) = (trim(lines[i]), trim(lines[i + 1]));
        // A section starts with 0/SECTION followed by 2/<name>.
        if code == b"0" && value == b"SECTION" && i + 3 < lines.len() && trim(lines[i + 2]) == b"2"
        {
            section = trim(lines[i + 3]);
        }
        if code == b"9" {
            skipping_variable = section == b"HEADER" && LOCAL_DATE_VARIABLES.contains(&value);
        } else if code == b"0" {
            skipping_variable = false;
        }
        let in_objects = section == b"OBJECTS";
        if code == b"0" && value == b"ENDSEC" {
            section = b"";
        }
        if !in_objects && !skipping_variable {
            out.extend_from_slice(lines[i]);
            out.extend_from_slice(lines[i + 1]);
        }
        i += 2;
    }
    // An odd trailing line (normally none) is kept as is.
    for line in &lines[i..] {
        out.extend_from_slice(line);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_local_dates_and_objects_only() {
        let input = "  0\r\nSECTION\r\n  2\r\nHEADER\r\n  9\r\n$INSUNITS\r\n 70\r\n4\r\n  9\r\n$TDCREATE\r\n 40\r\n2460000.5\r\n  9\r\n$TDUCREATE\r\n 40\r\n2460000.5\r\n  0\r\nENDSEC\r\n  0\r\nSECTION\r\n  2\r\nOBJECTS\r\n  0\r\nDICTIONARY\r\n  0\r\nENDSEC\r\n  0\r\nEOF\r\n";
        let out = String::from_utf8(strip_unused(input.as_bytes())).unwrap();
        assert_eq!(
            out,
            "  0\r\nSECTION\r\n  2\r\nHEADER\r\n  9\r\n$INSUNITS\r\n 70\r\n4\r\n  9\r\n$TDUCREATE\r\n 40\r\n2460000.5\r\n  0\r\nENDSEC\r\n  0\r\nEOF\r\n"
        );
    }

    #[test]
    fn binary_is_untouched() {
        let input = b"AutoCAD Binary DXF\r\n\x1a\x00rest";
        assert_eq!(strip_unused(input), input.to_vec());
    }
}
