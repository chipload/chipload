// SPDX-License-Identifier: GPL-3.0-or-later
//! JSON Schemas of the persisted formats people write or share (docs/specs/scripting.md §2):
//! the project file, the tool library and the machine profiles. Generated from the models,
//! so they say what loads at the current version: types, required fields and the allowed
//! values. Ranges and rules between fields stay in validation and the problem list.

use schemars::{JsonSchema, SchemaGenerator};
use serde_json::{Value, json};

use crate::CURRENT_VERSION;
use crate::Job;
use crate::library::{MACHINES_VERSION, MachineLibrary, TOOLS_VERSION, ToolLibrary};

/// The formats with a schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Project,
    Tools,
    Machines,
}

impl Format {
    pub fn name(self) -> &'static str {
        match self {
            Format::Project => "project",
            Format::Tools => "tools",
            Format::Machines => "machines",
        }
    }
}

/// The schema of `format`, its `schema_version` fixed to the current one.
pub fn schema(format: Format) -> Value {
    let (root, version, title) = match format {
        Format::Project => (root::<Job>(), CURRENT_VERSION, "Chipload project"),
        Format::Tools => (
            root::<ToolLibrary>(),
            TOOLS_VERSION,
            "Chipload tool library",
        ),
        Format::Machines => (
            root::<MachineLibrary>(),
            MACHINES_VERSION,
            "Chipload machine profiles",
        ),
    };
    let mut value = root;
    if let Some(object) = value.as_object_mut() {
        object.insert("title".into(), json!(title));
        object.insert(
            "description".into(),
            json!(format!(
                "Schema version {version}. Older files load through Chipload's migrations; \
                 this schema describes only the current version."
            )),
        );
    }
    if let Some(field) = value.pointer_mut("/properties/schema_version") {
        *field = json!({
            "const": version,
            "description": "The version of the file format. Chipload upgrades older files when it \
                            reads them, and refuses newer ones."
        });
    }
    value
}

fn root<T: JsonSchema>() -> Value {
    SchemaGenerator::default()
        .into_root_schema_for::<T>()
        .to_value()
}
