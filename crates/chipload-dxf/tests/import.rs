// SPDX-License-Identifier: GPL-3.0-or-later
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use chipload_dxf::{ImportError, UnitsFrom, import_dxf};
use chipload_job::Units;

fn corpus(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus/dxf")
        .join(name)
}

fn codes(result: &chipload_dxf::ImportResult) -> Vec<&'static str> {
    result.report.issues.iter().map(|i| i.code).collect()
}

#[test]
fn messy_2_has_two_parts_with_open_centrelines() {
    let result = import_dxf(&corpus("messy_2.dxf"), None).unwrap();
    assert_eq!(result.report.units, Units::Mm);
    assert_eq!(result.report.units_from, UnitsFrom::Dxf);
    assert_eq!(result.contours.len(), 22);
    let open: Vec<_> = result.contours.iter().filter(|c| !c.closed).collect();
    assert_eq!(open.len(), 2);
    assert_eq!(codes(&result), ["open_contour", "open_contour"]);
}

#[test]
fn ezdxf_file_with_local_dates_loads() {
    // simple_part.dxf has $TDCREATE, which crashes dxf 0.6.2 in some time zones.
    let result = import_dxf(&corpus("simple_part.dxf"), Some(Units::Mm)).unwrap();
    assert_eq!(result.contours.len(), 4);
    assert!(result.report.issues.is_empty());
}

#[test]
fn messy_reports_every_repair_and_skip() {
    let result = import_dxf(&corpus("messy.dxf"), Some(Units::Mm)).unwrap();
    assert_eq!(result.report.units_from, UnitsFrom::Job);
    assert_eq!(
        codes(&result),
        [
            "entity_skipped",
            "units_from_job",
            "nonzero_z",
            "zero_length",
            "duplicate",
            "open_contour",
            "open_contour",
            "open_contour",
        ]
    );
    assert_eq!(result.report.ignored.get("TEXT"), Some(&1));
    // Three block inserts of the same hole, each its own contour on the INSERT's layer.
    let holes: Vec<_> = result
        .contours
        .iter()
        .filter(|c| c.layers() == ["HOLES"])
        .collect();
    assert_eq!(holes.len(), 3);
    assert!(holes.iter().all(|c| c.nesting_depth == Some(1)));
}

#[test]
fn unitless_file_needs_units() {
    assert!(matches!(
        import_dxf(&corpus("messy.dxf"), None),
        Err(ImportError::UnitsMissing)
    ));
}

#[test]
fn declared_units_must_match_the_job() {
    assert!(matches!(
        import_dxf(&corpus("messy_2.dxf"), Some(Units::Inch)),
        Err(ImportError::UnitsMismatch { .. })
    ));
}

#[test]
fn garbage_is_an_error_not_a_crash() {
    let dir = std::env::temp_dir().join(format!("chipload-dxf-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("garbage.dxf");
    std::fs::write(
        &path,
        "  0\nSECTION\n  2\nENTITIES\n  0\nLINE\n 10\nnot a number\n",
    )
    .unwrap();
    let result = import_dxf(&path, Some(Units::Mm));
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(matches!(result, Err(ImportError::Unreadable { .. })));
}
