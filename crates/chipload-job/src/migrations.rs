// SPDX-License-Identifier: GPL-3.0-or-later
//! Schema migrations. Each step upgrades version N to N+1 and is a pure function (no I/O,
//! no clock). Their effect must never change: frozen samples in tests/corpus/jobs depend on it.

use serde_json::{Map, Value};

use crate::JobError;
use crate::models::CURRENT_VERSION;

type Migration = fn(Value) -> Value;

/// `MIGRATIONS[n - 1]` upgrades version n to n + 1.
const MIGRATIONS: [Migration; 2] = [v1_to_v2, v2_to_v3];

/// Apply migrations step by step. Missing, non-integer (including `true`), < 1 or newer
/// versions are refused.
pub fn migrate(mut data: Value) -> Result<Value, JobError> {
    let raw = data.get("schema_version").cloned();
    let version = match raw.as_ref().and_then(Value::as_i64) {
        // as_i64 is None for floats, bools and strings, matching the Python check.
        Some(v) if (1..=CURRENT_VERSION).contains(&v) => v,
        _ => {
            return Err(JobError::UnsupportedVersion {
                found: describe(raw.as_ref()),
                current: CURRENT_VERSION,
            });
        }
    };
    for step in version..CURRENT_VERSION {
        let index = usize::try_from(step - 1).unwrap_or(usize::MAX);
        let Some(migration) = MIGRATIONS.get(index) else {
            return Err(JobError::UnsupportedVersion {
                found: step.to_string(),
                current: CURRENT_VERSION,
            });
        };
        data = migration(data);
    }
    Ok(data)
}

/// The value as Python's `repr` would show it in the error message.
fn describe(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => "None".into(),
        Some(Value::Bool(b)) => if *b { "True" } else { "False" }.into(),
        Some(Value::String(s)) => format!("'{s}'"),
        Some(other) => other.to_string(),
    }
}

fn operations_mut(data: &mut Value) -> Vec<&mut Map<String, Value>> {
    match data.get_mut("operations") {
        Some(Value::Array(ops)) => ops.iter_mut().filter_map(Value::as_object_mut).collect(),
        _ => Vec::new(),
    }
}

/// v2 adds spindle direction and plunge feed. v1 never produced G-code, so no behaviour is
/// preserved. Decision 2026-10-08: spindle "cw", plunge_feed = feed / 3.
fn v1_to_v2(mut data: Value) -> Value {
    if let Some(Value::Object(machine)) = data.get_mut("machine") {
        machine.insert("spindle".into(), Value::from("cw"));
    }
    for op in operations_mut(&mut data) {
        let kind = op.get("type").and_then(Value::as_str);
        if !matches!(kind, Some("profile" | "pocket")) {
            continue;
        }
        // Booleans are not numbers here (as_f64 is None for them), matching Python.
        if let Some(feed) = op.get("feed").and_then(Value::as_f64) {
            op.insert("plunge_feed".into(), Value::from(feed / 3.0));
        }
    }
    set_version(data, 2)
}

/// v3 makes the pocket direction explicit. Pocket generation didn't exist in v2.
/// Decision 2026-10-08: existing pocket ops become "climb".
fn v2_to_v3(mut data: Value) -> Value {
    for op in operations_mut(&mut data) {
        if op.get("type").and_then(Value::as_str) == Some("pocket") {
            op.insert("direction".into(), Value::from("climb"));
        }
    }
    set_version(data, 3)
}

fn set_version(mut data: Value, version: i64) -> Value {
    if let Value::Object(map) = &mut data {
        map.insert("schema_version".into(), Value::from(version));
    }
    data
}
