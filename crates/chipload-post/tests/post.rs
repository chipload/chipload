// SPDX-License-Identifier: GPL-3.0-or-later
//! Post output rules.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chipload_check::{CheckLimits, check_gcode};
use chipload_job::{Spindle, Tool, ToolChange, ToolType};
use chipload_post::{
    ToolPart, load_builtin_post, parse_post_config, render_program, render_with_tool_changes,
};
use chipload_toolpath::model::{Move, Program};

fn program(spindle: Spindle) -> Program {
    Program {
        tool_id: 1,
        tool_diameter: 6.0,
        rpm: 18000.0,
        spindle,
        safe_z: 5.0,
        moves: vec![
            Move::rapid_z(5.0),
            Move::rapid_xy((10.0, 10.0)),
            Move::feed_z(-1.0, 300.0),
            Move::feed_xy((50.0, 10.0), 1000.0),
            Move::feed_xy((50.0, 40.0), 1000.0),
            Move::feed_xy((10.0, 40.0), 1000.0),
            Move::feed_xy((10.0, 10.0), 1000.0),
            Move::rapid_z(5.0),
        ],
        comments: Vec::new(),
    }
}

#[test]
fn output_is_deterministic() {
    let post = load_builtin_post("grbl").unwrap();
    let a = render_program(&program(Spindle::Cw), &post).unwrap();
    let b = render_program(&program(Spindle::Cw), &post).unwrap();
    assert_eq!(a, b);
}

#[test]
fn ccw_spindle_uses_m4() {
    let post = load_builtin_post("grbl").unwrap();
    let text = render_program(&program(Spindle::Ccw), &post).unwrap();
    assert!(text.lines().any(|l| l == "M4 S18000"), "{text}");
    assert!(!text.lines().any(|l| l.starts_with("M3 ")), "{text}");
}

#[test]
fn zero_dwell_is_omitted() {
    let config =
        "schema_version = 1\nname = \"T\"\ntemplate = \"grbl.nc.j2\"\nspindle_dwell_s = 0\n";
    let post = parse_post_config(config).unwrap();
    let text = render_program(&program(Spindle::Cw), &post).unwrap();
    assert!(!text.contains("G4"), "{text}");
}

#[test]
fn rendered_program_passes_the_checker() {
    let post = load_builtin_post("grbl").unwrap();
    let text = render_program(&program(Spindle::Cw), &post).unwrap();
    let limits = CheckLimits {
        safe_z: 5.0,
        stock_top_z: 0.0,
        stock_bottom_z: -18.0,
        max_overcut: 0.0,
        stock_min: (0.0, 0.0),
        stock_max: (600.0, 400.0),
        tool_radius: 3.0,
        tool_radii: std::collections::BTreeMap::from([(1, 3.0), (2, 1.5)]),
        travel: Some((800.0, 600.0, 100.0)),
        units: chipload_job::Units::Mm,
    };
    let report = check_gcode(&text, &limits);
    assert!(report.ok(), "{:?}", report.errors);
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
}

#[test]
fn a_post_without_tool_changes_refuses_one_file_with_m6() {
    // docs/specs/tool-per-operation.md §3.2: a config that doesn't list "program" (missing
    // means files only) is never asked to write M6.
    let mut post = load_builtin_post("grbl").unwrap();
    assert_eq!(post.tool_changes, [ToolChange::Files, ToolChange::Program]);
    let text = "schema_version = 1\nname = \"Old\"\ntemplate = \"grbl.nc.j2\"\n\
                spindle_dwell_s = 0\n";
    let old = parse_post_config(text).unwrap();
    assert_eq!(old.tool_changes, [ToolChange::Files]);
    let tool = |id: i64, diameter: f64| Tool {
        id,
        name: None,
        library_uid: None,
        kind: ToolType::Endmill,
        diameter,
        flutes: None,
        center_cutting: None,
        rpm: Some(18000.0),
        angle: None,
        tip_diameter: None,
    };
    let parts = [
        ToolPart {
            program: program(Spindle::Cw),
            tool: tool(1, 6.0),
            operations: vec![chipload_job::OpRef::op(0)],
        },
        ToolPart {
            program: program(Spindle::Cw),
            tool: tool(2, 3.0),
            operations: vec![chipload_job::OpRef::op(1)],
        },
    ];
    let err = render_with_tool_changes(&parts, &old, None, chipload_job::Units::Mm).unwrap_err();
    assert!(
        err.to_string()
            .contains("can't write tool changes in one file"),
        "{err}"
    );
    post.arcs = false;
    let text = render_with_tool_changes(&parts, &post, None, chipload_job::Units::Mm).unwrap();
    assert_eq!(text.matches(" M6\n").count(), 2, "{text}");
}

// -- user posts and the LinuxCNC and Mach posts (docs/specs/user-posts.md)

fn posts_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("chipload-posts-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

const MY_CONFIG: &str =
    "schema_version = 1\nname = \"My router\"\ntemplate = \"my.nc.j2\"\nspindle_dwell_s = 2\n";

fn checked(text: &str) {
    let limits = CheckLimits {
        safe_z: 5.0,
        stock_top_z: 0.0,
        stock_bottom_z: -18.0,
        max_overcut: 0.0,
        stock_min: (0.0, 0.0),
        stock_max: (600.0, 400.0),
        tool_radius: 3.0,
        tool_radii: std::collections::BTreeMap::from([(1, 3.0)]),
        travel: Some((800.0, 600.0, 100.0)),
        units: chipload_job::Units::Mm,
    };
    let report = check_gcode(text, &limits);
    assert!(report.ok(), "{:?}\n{text}", report.errors);
}

#[test]
fn every_builtin_post_renders_a_program_the_checker_passes() {
    for name in chipload_post::builtin_posts() {
        let post = load_builtin_post(name).unwrap();
        let text = render_program(&program(Spindle::Cw), &post).unwrap();
        checked(&text);
        assert!(text.contains("S18000"), "{name}: {text}");
    }
    let linuxcnc = render_program(
        &program(Spindle::Cw),
        &load_builtin_post("linuxcnc").unwrap(),
    )
    .unwrap();
    assert!(
        linuxcnc.starts_with("%\n") && linuxcnc.ends_with("M30\n%\n"),
        "{linuxcnc}"
    );
    assert!(linuxcnc.contains("G64 P0.01") && linuxcnc.contains("G91.1"));
    assert!(linuxcnc.contains("Not yet tested"));
    let mach = render_program(&program(Spindle::Cw), &load_builtin_post("mach3").unwrap()).unwrap();
    assert!(mach.contains("G91.1") && !mach.contains("G64"), "{mach}");
    assert!(mach.contains("G4 P3.000"));
}

#[test]
fn a_user_post_is_read_from_its_folder_and_renders_from_its_own_template() {
    let dir = posts_dir("read");
    std::fs::write(dir.join("my.toml"), MY_CONFIG).unwrap();
    let template = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/grbl.nc.j2"),
    )
    .unwrap()
    .replace(
        "(Chipload - {{ post_name }})",
        "(My own post: {{ post_name }})",
    );
    std::fs::write(dir.join("my.nc.j2"), &template).unwrap();
    let post = chipload_post::read_user_post(&dir, "my").unwrap();
    assert_eq!(post.config, MY_CONFIG);
    let config = chipload_post::parse_user_post(&post).unwrap();
    let text = render_program(&program(Spindle::Cw), &config).unwrap();
    assert!(text.starts_with("(My own post: My router)\n"), "{text}");
    checked(&text);
    let listed = chipload_post::list_user_posts(&dir);
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].0, "my");
    assert!(listed[0].1.is_ok());
}

#[test]
fn broken_user_posts_say_why() {
    let dir = posts_dir("broken");
    let read = |name: &str| {
        chipload_post::read_user_post(&dir, name)
            .unwrap_err()
            .to_string()
    };
    // No template file.
    std::fs::write(dir.join("my.toml"), MY_CONFIG).unwrap();
    assert!(read("my").contains("my.nc.j2"), "{}", read("my"));
    // A template name that is a path.
    std::fs::write(
        dir.join("path.toml"),
        MY_CONFIG.replace("my.nc.j2", "../etc/x"),
    )
    .unwrap();
    assert!(read("path").contains("plain file name"), "{}", read("path"));
    // A built-in's name, and a name that is a path.
    std::fs::write(dir.join("grbl.toml"), MY_CONFIG).unwrap();
    assert!(read("grbl").contains("built-in"), "{}", read("grbl"));
    assert!(read("../my").contains("Invalid"));
    // A syntax error shows when the post is read.
    std::fs::write(dir.join("my.nc.j2"), "G21 {% for x in %}\n").unwrap();
    assert!(read("my").contains("can't be read"), "{}", read("my"));
    let listed = chipload_post::list_user_posts(&dir);
    assert_eq!(listed.len(), 3);
    assert!(listed.iter().all(|(_, post)| post.is_err()));
}

#[test]
fn a_template_that_never_ends_fails_instead_of_hanging() {
    let post = chipload_post::UserPost {
        config: MY_CONFIG.into(),
        template: "{% for a in range(100000) %}{% for b in range(100000) %}{% endfor %}\
                   {% endfor %}G21\n"
            .into(),
    };
    let config = chipload_post::parse_user_post(&post).unwrap();
    let started = std::time::Instant::now();
    let err = render_program(&program(Spindle::Cw), &config).unwrap_err();
    assert!(err.to_string().contains("fuel"), "{err}");
    assert!(started.elapsed().as_secs() < 10);
}

// -- inches (docs/specs/inch-units.md §4.2)

#[test]
fn a_post_writes_inches_only_if_its_config_says_its_template_can() {
    use chipload_job::Units;
    // A user post from before inches: its template writes G21 itself. An inch machine is
    // refused rather than given inch numbers under G21.
    let old = chipload_post::UserPost {
        config: MY_CONFIG.into(),
        template: "G21 G90\n{% for s in segments %}{% for m in s.moves %}{{ m.code }}\n\
                   {% endfor %}{% endfor %}M30\n"
            .into(),
    };
    let config = chipload_post::parse_user_post(&old).unwrap();
    assert_eq!(config.units, [Units::Mm]);
    let err = chipload_post::render_program_with(&program(Spindle::Cw), &config, None, Units::Inch)
        .unwrap_err();
    assert!(
        err.to_string().contains("can't write G-code in inch"),
        "{err}"
    );
    assert!(
        chipload_post::render_program_with(&program(Spindle::Cw), &config, None, Units::Mm).is_ok()
    );
    // The built-in posts write either, with 4 decimals in inches.
    for name in ["grbl", "linuxcnc", "mach3"] {
        let post = load_builtin_post(name).unwrap();
        assert_eq!(post.units, [Units::Mm, Units::Inch]);
        let text =
            chipload_post::render_program_with(&program(Spindle::Cw), &post, None, Units::Inch)
                .unwrap();
        assert!(
            text.contains("G20 G90") && !text.contains("G21"),
            "{name}: {text}"
        );
        assert!(text.contains("inch)"), "{name}: the tool comment says inch");
    }
}

#[test]
fn inch_decimals_are_checked_and_default_to_4() {
    let config = parse_post_config(&format!("{MY_CONFIG}inch_decimals = 7\n"));
    assert!(config.unwrap_err().to_string().contains("inch_decimals"));
    assert_eq!(parse_post_config(MY_CONFIG).unwrap().inch_decimals, 4);
    let none = parse_post_config(&format!("{MY_CONFIG}units = []\n"));
    assert!(none.unwrap_err().to_string().contains("units"));
}
