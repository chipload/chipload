// SPDX-License-Identifier: GPL-3.0-or-later
//! Converting an old library on disk: the materials' values move into the tools, once
//! (docs/specs/cutting-data.md §8, §12).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use chipload_job::library::{Cutting, TOOLS_VERSION};
use chipload_job::materials::MATERIALS_VERSION;
use chipload_pipeline::library::Library;

fn corpus(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus")
        .join(rel)
}

#[test]
fn an_old_library_converts_once_with_backups() {
    let dir = std::env::temp_dir().join(format!("chipload-cutting-data-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(corpus("tools/v3/tools.json"), dir.join("tools.json")).unwrap();
    std::fs::copy(
        corpus("materials/v2/materials.json"),
        dir.join("materials.json"),
    )
    .unwrap();

    let library = Library::load(&dir);
    assert!(library.problems.is_empty(), "{:?}", library.problems);
    let said: Vec<String> = library.converted.iter().map(|m| m.english()).collect();
    assert!(said[0].contains("library was updated"), "{said:?}");
    assert!(said.iter().any(|l| l.contains("6 mm 2-flute")), "{said:?}");
    // The value for a tool the library hasn't got is named, and dropped.
    assert!(
        said.iter().any(|l| l.contains("ffffffffffffffff")),
        "{said:?}"
    );

    assert_eq!(library.tools.schema_version, TOOLS_VERSION);
    assert_eq!(library.materials.schema_version, MATERIALS_VERSION);
    let six = library.tools.find("6 mm 2-flute").unwrap();
    let plywood = library.materials.find("Plywood").unwrap();
    // Plywood's values, slot and clearing alike; the tool's own for any material.
    assert_eq!(
        six.row(Some(&plywood.uid)).map(|r| r.values),
        Some(Cutting::uniform(1500.0, 500.0, 2.5, 0.4))
    );
    assert_eq!(
        six.row(None).map(|r| r.values),
        Some(Cutting::uniform(1200.0, 400.0, 2.0, 0.45))
    );
    assert_eq!(
        library.materials.default_material.as_deref(),
        Some("Plywood")
    );
    for file in ["tools.json.bak", "materials.json.bak"] {
        assert!(dir.join(file).exists(), "{file}");
    }

    // Read again: nothing more to convert, and the same library.
    let again = Library::load(&dir);
    assert!(again.converted.is_empty());
    assert_eq!(again.tools, library.tools);
    std::fs::remove_dir_all(dir).unwrap();
}
