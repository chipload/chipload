// SPDX-License-Identifier: GPL-3.0-or-later
//! The setup sheet (docs/specs/setup-sheet.md): golden sheets of frozen corpus projects, the
//! zero in words, escaping.
//!
//! When a change is intended, regenerate with `CHIPLOAD_UPDATE_GOLDEN=1 cargo test -p
//! chipload-pipeline --test sheet`, review the diff, and explain it in the commit message.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use chipload_job::{StockCorner, StockOrigin, WorkZero, ZZero, load_job};
use chipload_pipeline::sheet::{SheetInfo, setup_sheet, write_setup_sheet, xy_zero_text};
use chipload_pipeline::{empty_project, run_job};

fn corpus(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus")
        .join(path)
}

fn info(project: &str) -> SheetInfo {
    SheetInfo {
        project: project.into(),
        // Not the real version: the golden sheets stay the same from release to release.
        written_by: "chipload (test)".into(),
        units: chipload_job::Units::Mm,
        language: chipload_i18n::Language::En,
        parts: Vec::new(),
    }
}

/// (golden name, frozen project, G-code output as given to `run -o`).
const GOLDEN: [(&str, &str, &str); 13] = [
    // Two files, one per tool.
    ("v11_two_tools", "jobs/v11/two_tools.chipload", "part.nc"),
    // The same in one file with M6.
    (
        "v12_two_tools_program",
        "jobs/v12/two_tools_program.chipload",
        "part.nc",
    ),
    // Helix, leads, tabs, a cut into the spoilboard and a post with a note.
    (
        "v15_linuxcnc_helix",
        "jobs/v15/linuxcnc_helix.chipload",
        "part.ngc",
    ),
    // Follow paths: open and closed shapes, a V-bit groove and a ramped endmill.
    (
        "v17_follow_path",
        "jobs/v17/follow_path.chipload",
        "part.nc",
    ),
    // A finishing pass and an onion skin in the depth column.
    (
        "v17_finish_skin",
        "jobs/v17/finish_skin.chipload",
        "part.nc",
    ),
    // Drilled and bored holes: the drill's point, pecks and the helix.
    ("v18_holes", "jobs/v18/holes.chipload", "part.nc"),
    // A surface: the whole stock, back and forth, how far past the ends it runs.
    ("v29_surface", "jobs/v29/surface.chipload", "part.nc"),
    // Dogbones and T-bones in the kind column.
    ("v18_dogbones", "jobs/v18/dogbones.chipload", "part.nc"),
    // A rest pocket after a larger tool (docs/specs/rest-machining.md §6).
    ("v18_rest", "jobs/v18/rest.chipload", "part.nc"),
    (
        "v19_image_vcarve",
        "jobs/v19/image_vcarve.chipload",
        "part.nc",
    ),
    (
        "v20_image_centre_line",
        "jobs/v20/image_centre_line.chipload",
        "part.nc",
    ),
    ("v21_two_dxfs", "jobs/v21/two_dxfs.chipload", "part.nc"),
    // Two parts (docs/specs/parts.md §6): each part named over its operations.
    ("v22_two_parts", "jobs/v22/two_parts.chipload", "part.nc"),
];

#[test]
fn sheets_match_their_golden_files() {
    let update = std::env::var_os("CHIPLOAD_UPDATE_GOLDEN").is_some();
    for (name, project, output) in GOLDEN {
        let job = load_job(&corpus(project)).unwrap();
        let result = run_job(&job).unwrap();
        let sheet = setup_sheet(&job, &result, Path::new(output), &info(project));
        let again = setup_sheet(
            &job,
            &run_job(&job).unwrap(),
            Path::new(output),
            &info(project),
        );
        assert_eq!(sheet, again, "{name}: not deterministic");
        assert!(sheet.is_ascii(), "{name}: not ASCII");
        let golden = corpus(&format!("expected/sheets/{name}.html"));
        if update {
            std::fs::create_dir_all(golden.parent().unwrap()).unwrap();
            std::fs::write(&golden, &sheet).unwrap();
            continue;
        }
        let expected = std::fs::read_to_string(&golden)
            .unwrap_or_else(|e| panic!("{}: {e}", golden.display()));
        assert!(
            sheet == expected,
            "{name}: the sheet differs from {}; if intended, regenerate with \
             CHIPLOAD_UPDATE_GOLDEN=1 and review the diff",
            golden.display()
        );
    }
}

#[test]
fn the_sheet_says_what_the_operator_needs() {
    let job = load_job(&corpus("jobs/v11/two_tools.chipload")).unwrap();
    let result = run_job(&job).unwrap();
    let sheet = setup_sheet(&job, &result, Path::new("out/part.nc"), &info("two_tools"));
    // The files in order, with the tool to load before each.
    let first = sheet.find("1. part-1-T2.nc").unwrap();
    let second = sheet.find("2. part-2-T1.nc").unwrap();
    assert!(first < second);
    assert!(sheet.contains("Load tool T2 (3 mm end mill, 2 flutes)"));
    assert!(sheet.contains("Change to tool T1 (6 mm end mill, 2 flutes). Set Z0 again"));
    assert!(sheet.contains("Set X0 Y0 at the stock&#39;s bottom left corner"));
    assert!(sheet.contains("Cuts down to Z-12: 12 mm below the stock top, through the stock"));
    // The times are the run's.
    assert!(sheet.contains("6 min 11 s"), "{sheet}");
    assert!(sheet.contains("Estimated time: "));
}

#[test]
fn the_sheet_warns_of_the_spoilboard_and_untested_posts() {
    let job = load_job(&corpus("jobs/v15/linuxcnc_helix.chipload")).unwrap();
    let result = run_job(&job).unwrap();
    let sheet = setup_sheet(&job, &result, Path::new("part.ngc"), &info("helix"));
    assert!(sheet.contains("Post LinuxCNC: Not yet tested on a real LinuxCNC machine"));
    assert!(sheet.contains("0.3 mm into the spoilboard"));
    assert!(sheet.contains("tabs</span>"));
}

#[test]
fn the_zero_is_said_in_words_on_the_stock() {
    let mut job = empty_project();
    // 300 x 200, bottom left at the drawing origin
    job.stock_mut().origin = StockOrigin::BottomLeft;
    job.stock_mut().fit = None;
    assert_eq!(xy_zero_text(&job), "the stock's bottom left corner");
    *job.work_zero_mut() = WorkZero::Stock {
        corner: StockCorner::Center,
    };
    assert_eq!(xy_zero_text(&job), "the centre of the stock");
    *job.work_zero_mut() = WorkZero::Stock {
        corner: StockCorner::TopRight,
    };
    assert_eq!(xy_zero_text(&job), "the stock's top right corner");
    *job.work_zero_mut() = WorkZero::Point { x: 25.0, y: 10.5 };
    assert_eq!(
        xy_zero_text(&job),
        "25 mm right of and 10.5 mm above the stock's bottom left corner"
    );
    *job.work_zero_mut() = WorkZero::Point { x: 0.0, y: 40.0 };
    assert_eq!(
        xy_zero_text(&job),
        "40 mm above the stock's bottom left corner"
    );
    *job.work_zero_mut() = WorkZero::Point { x: -5.0, y: 0.0 };
    assert_eq!(
        xy_zero_text(&job),
        "5 mm left of the stock's bottom left corner (outside the stock)"
    );
    // A centred stock: the drawing origin is its centre.
    job.stock_mut().origin = StockOrigin::Center;
    *job.work_zero_mut() = WorkZero::DrawingOrigin {};
    assert_eq!(
        xy_zero_text(&job),
        "150 mm right of and 100 mm above the stock's bottom left corner"
    );
    *job.work_zero_mut() = WorkZero::Point { x: 0.0, y: -120.0 };
    assert_eq!(
        xy_zero_text(&job),
        "150 mm right of and 20 mm below the stock's bottom left corner (outside the stock)"
    );
}

#[test]
fn z_zero_on_the_spoilboard_is_said_so() {
    let mut job = load_job(&corpus("jobs/v11/two_tools.chipload")).unwrap();
    job.stock_mut().z_zero = ZZero::Bottom;
    job.machine.safe_z += job.stock().thickness;
    let result = run_job(&job).unwrap();
    let sheet = setup_sheet(&job, &result, Path::new("part.nc"), &info("two_tools"));
    assert!(sheet.contains("the bottom of the stock (the spoilboard)"));
    // Z0 at the bottom: the outline ends at Z0, still 12 mm below the top.
    assert!(sheet.contains("Cuts down to Z0: 12 mm below the stock top, through the stock"));
}

#[test]
fn names_from_the_drawing_are_escaped() {
    let mut job = load_job(&corpus("jobs/v11/two_tools.chipload")).unwrap();
    *job.operations[0].name_mut() = "<b>Holes & \"slots\"</b> \u{e5}".into();
    job.tools[0].name = Some("<script>".into());
    let result = run_job(&job).unwrap();
    let sheet = setup_sheet(
        &job,
        &result,
        Path::new("part.nc"),
        &info("a <test> & \u{f6}"),
    );
    assert!(sheet.is_ascii());
    assert!(!sheet.contains("<b>") && !sheet.contains("<script>"));
    assert!(sheet.contains("&lt;b&gt;Holes &amp; &quot;slots&quot;&lt;/b&gt; &#xe5;"));
    assert!(sheet.contains("<title>Setup sheet: a &lt;test&gt; &amp; &#xf6;</title>"));
}

#[test]
fn the_picture_covers_the_stock_and_every_cut() {
    let job = load_job(&corpus("jobs/v7/copies.chipload")).unwrap();
    let result = run_job(&job).unwrap();
    let sheet = setup_sheet(&job, &result, Path::new("part.nc"), &info("copies"));
    let view = sheet.split("viewBox=\"").nth(1).unwrap();
    let numbers: Vec<f64> = view
        .split('"')
        .next()
        .unwrap()
        .split(' ')
        .map(|n| n.parse().unwrap())
        .collect();
    let (x0, top, w, h) = (numbers[0], numbers[1], numbers[2], numbers[3]);
    // SVG y runs down: drawing y is -svg y.
    let (y0, y1, x1) = (-(top + h), -top, x0 + w);
    let ((sx0, sy0), (sx1, sy1)) = job.stock_bounds();
    assert!(x0 < sx0 && y0 < sy0 && x1 > sx1 && y1 > sy1);
    for m in &result.program.moves {
        if let (Some(x), Some(y)) = (m.x, m.y) {
            assert!(
                x > x0 && x < x1 && y > y0 && y < y1,
                "({x}, {y}) outside the picture"
            );
        }
    }
}

#[test]
fn the_sheet_is_written_next_to_the_gcode() {
    let job = load_job(&corpus("jobs/v11/two_tools.chipload")).unwrap();
    let result = run_job(&job).unwrap();
    let dir = std::env::temp_dir().join(format!("chipload-sheet-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = write_setup_sheet(&job, &result, &dir.join("part.nc"), &info("two_tools")).unwrap();
    assert_eq!(path, dir.join("part-sheet.html"));
    assert!(
        std::fs::read_to_string(&path)
            .unwrap()
            .contains("part-1-T2.nc")
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Sheets in inches (docs/specs/inch-units.md §5.4): shown in inches, and one whose G-code
/// is in inches too.
#[test]
fn inch_sheets_match_their_golden_files() {
    let update = std::env::var_os("CHIPLOAD_UPDATE_GOLDEN").is_some();
    for (name, project, gcode_in_inches) in [
        (
            "v15_linuxcnc_helix",
            "jobs/v15/linuxcnc_helix.chipload",
            false,
        ),
        ("v18_holes", "jobs/v18/holes.chipload", true),
        ("v22_two_parts", "jobs/v22/two_parts.chipload", false),
    ] {
        let mut job = load_job(&corpus(project)).unwrap();
        if gcode_in_inches {
            job.machine.gcode_units = chipload_job::Units::Inch;
        }
        let result = run_job(&job).unwrap();
        let info = SheetInfo {
            units: chipload_job::Units::Inch,
            ..info(project)
        };
        let sheet = setup_sheet(&job, &result, Path::new("part.nc"), &info);
        assert!(sheet.contains("lengths in in, feeds in in/min"), "{name}");
        assert_eq!(
            sheet.contains("G-code in inches (G20)"),
            gcode_in_inches,
            "{name}"
        );
        // Every length on it in inches, messages included (docs/specs/translations.md §7).
        let left: Vec<&str> = sheet.lines().filter(|l| l.contains(" mm")).collect();
        assert!(left.is_empty(), "{name}: lengths left in mm: {left:?}");
        let golden = corpus(&format!("expected/sheets/inch/{name}.html"));
        if update {
            std::fs::create_dir_all(golden.parent().unwrap()).unwrap();
            std::fs::write(&golden, &sheet).unwrap();
            continue;
        }
        let expected = std::fs::read_to_string(&golden)
            .unwrap_or_else(|e| panic!("{}: {e}", golden.display()));
        assert!(
            sheet == expected,
            "{name}: differs from {}",
            golden.display()
        );
    }
}

/// A sheet in German (docs/specs/translations.md step 3): its words, the notes and the
/// estimate in the language; names, numbers and G-code as they are. Still ASCII.
#[test]
fn a_german_sheet_matches_its_golden_file() {
    let update = std::env::var_os("CHIPLOAD_UPDATE_GOLDEN").is_some();
    let (name, project) = ("v15_linuxcnc_helix", "jobs/v15/linuxcnc_helix.chipload");
    let job = load_job(&corpus(project)).unwrap();
    let result = run_job(&job).unwrap();
    let info = SheetInfo {
        language: chipload_i18n::Language::De,
        parts: Vec::new(),
        ..info(project)
    };
    let sheet = setup_sheet(&job, &result, Path::new("part.ngc"), &info);
    assert!(sheet.is_ascii(), "{name}: not ASCII");
    assert!(sheet.contains("<html lang=\"de\">"), "{name}");
    let golden = corpus(&format!("expected/sheets/de/{name}.html"));
    if update {
        std::fs::create_dir_all(golden.parent().unwrap()).unwrap();
        std::fs::write(&golden, &sheet).unwrap();
        return;
    }
    let expected =
        std::fs::read_to_string(&golden).unwrap_or_else(|e| panic!("{}: {e}", golden.display()));
    assert!(
        sheet == expected,
        "{name}: differs from {}",
        golden.display()
    );
}
