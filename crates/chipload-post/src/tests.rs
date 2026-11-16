// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[test]
fn a_part_comment_is_written_before_its_move_even_if_that_move_goes_nowhere() {
    // docs/specs/parts.md §6: the comment on a move that is dropped (already at safe Z)
    // goes with the next move written.
    let mut start = Move::rapid_z(5.0);
    start.comment = Some(0);
    let program = Program {
        tool_id: 1,
        tool_diameter: 6.0,
        rpm: 18000.0,
        spindle: Spindle::Cw,
        safe_z: 5.0,
        moves: vec![Move::rapid_z(5.0), start, Move::rapid_xy((1.0, 2.0))],
        comments: vec!["Part 2: Lådor (box), 2 copies".into()],
    };
    let config = load_builtin_post("grbl").unwrap();
    let text = render_program(&program, &config).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    let at = lines
        .iter()
        .position(|l| *l == "(Part 2: Lador [box], 2 copies)")
        .unwrap_or_else(|| panic!("{text}"));
    assert_eq!(lines[at + 1], "G0 X1.000 Y2.000");
}

#[test]
fn comments_are_ascii_and_never_end_early() {
    assert_eq!(comment_safe("60° V-bit (fine)"), "60deg V-bit [fine]");
    assert_eq!(comment_safe("Lådor; Öppning"), "Lador, Oppning");
    assert_eq!(comment_safe("Ø6 ×2 ☃"), "O6 x2 ?");
    assert!(comment_safe("\u{1f600}\nx").is_ascii());
}
use chipload_toolpath::model::Move;

#[test]
fn numbers_never_show_negative_zero() {
    assert_eq!(format_number(-0.0004, 3), "0.000");
    assert_eq!(format_number(-0.0006, 3), "-0.001");
    assert_eq!(format_number(0.125, 2), "0.12"); // ties to even
    assert_eq!(format_number(18000.0, 0), "18000");
}

#[test]
fn grbl_config_loads() {
    let config = load_builtin_post("grbl").unwrap();
    assert_eq!(config.name, "GRBL");
    assert_eq!(config.decimals, 3);
    assert_eq!(config.spindle_dwell_s, 3.0);
}

#[test]
fn posts_not_tested_on_a_machine_say_so_in_their_note() {
    // Shown on the setup sheet (docs/specs/setup-sheet.md §2).
    assert_eq!(load_builtin_post("grbl").unwrap().note, None);
    for name in ["linuxcnc", "mach3"] {
        let note = load_builtin_post(name).unwrap().note.unwrap();
        assert!(note.starts_with("Not yet tested on a real"), "{note}");
    }
}

#[test]
fn unknown_or_unsafe_post_names_are_refused() {
    assert!(load_builtin_post("../grbl").is_err());
    assert!(load_builtin_post("nope").is_err());
}

#[test]
fn versions_are_checked() {
    let body = "name = \"X\"\ntemplate = \"grbl.nc.j2\"\nspindle_dwell_s = 0\n";
    for version in [
        "",
        "schema_version = 2\n",
        "schema_version = true\n",
        "schema_version = 1.0\n",
    ] {
        let err = parse_post_config(&format!("{version}{body}")).unwrap_err();
        assert!(
            matches!(err, PostError::UnsupportedVersion { .. }),
            "{version}: {err}"
        );
    }
    assert!(parse_post_config(&format!("schema_version = 1\n{body}")).is_ok());
}

#[test]
fn dwell_is_required_and_template_must_be_a_plain_name() {
    let missing = "schema_version = 1\nname = \"X\"\ntemplate = \"grbl.nc.j2\"\n";
    assert!(parse_post_config(missing).is_err());
    let path = "schema_version = 1\nname = \"X\"\ntemplate = \"../x\"\nspindle_dwell_s = 0\n";
    assert!(parse_post_config(path).is_err());
}

#[test]
fn output_has_unix_line_endings_on_every_platform() {
    // A Windows checkout may give the template CRLF; the G-code must not change.
    let program = Program {
        tool_id: 1,
        tool_diameter: 6.0,
        rpm: 18000.0,
        spindle: Spindle::Cw,
        safe_z: 5.0,
        moves: vec![Move::rapid_z(5.0), Move::rapid_xy((1.0, 2.0))],
        comments: Vec::new(),
    };
    let config = load_builtin_post("grbl").unwrap();
    let text = render_program_with(&program, &config, Some("x (y); z"), Units::Mm).unwrap();
    assert!(!text.contains('\r'), "{text:?}");
    assert!(text.contains("(Work zero: x [y], z)\n"), "{text}");
}

#[test]
fn modal_compression() {
    let program = Program {
        tool_id: 1,
        tool_diameter: 6.0,
        rpm: 18000.0,
        spindle: Spindle::Cw,
        safe_z: 5.0,
        moves: vec![
            Move::rapid_z(5.0),
            Move::rapid_xy((1.0, 2.0)),
            Move::feed_z(-1.0, 300.0),
            Move::feed_xy((10.0, 2.0), 1000.0),
            Move::feed_xy((10.0, 2.0), 1000.0), // goes nowhere: dropped
            Move::feed_xy((10.0, 5.0), 1000.0),
            Move::rapid_z(5.0),
        ],
        comments: Vec::new(),
    };
    let config = load_builtin_post("grbl").unwrap();
    let text = render_program(&program, &config).unwrap();
    let expected = "(Chipload - GRBL)\n(Tool 1: 6.000 mm)\nG21 G90 G17 G94\nM3 S18000\n\
                    G4 P3.000\nG0 Z5.000\nG0 X1.000 Y2.000\nG1 Z-1.000 F300.000\n\
                    G1 X10.000 F1000.000\nG1 Y5.000\nG0 Z5.000\nM5\nM30\n";
    assert_eq!(text, expected);
}
