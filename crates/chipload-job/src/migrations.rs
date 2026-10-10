// SPDX-License-Identifier: GPL-3.0-or-later
//! Schema migrations. Each step upgrades version N to N+1 and is a pure function (no I/O,
//! no clock). Their effect must never change: frozen samples in tests/corpus/jobs depend on it.
//! Versions before projects (1-3) were never released and are refused (decision 2026-10-10).

use serde_json::{Map, Value};

use crate::JobError;
use crate::models::{CURRENT_VERSION, OLDEST_VERSION};

type Migration = fn(Value) -> Result<Value, String>;

/// `MIGRATIONS[n - OLDEST_VERSION]` upgrades version n to n + 1.
const MIGRATIONS: [Migration; 2] = [v4_to_v5, v5_to_v6];

/// Apply migrations step by step. Missing, non-integer (including `true`) and newer versions
/// are refused, as are versions older than [`OLDEST_VERSION`] and files a migration can't
/// carry forward without changing what they mean.
pub fn migrate(mut data: Value) -> Result<Value, JobError> {
    let raw = data.get("schema_version").cloned();
    let version = match raw.as_ref().and_then(Value::as_i64) {
        // as_i64 is None for floats, bools and strings.
        Some(v) if (OLDEST_VERSION..=CURRENT_VERSION).contains(&v) => v,
        Some(v) if (1..OLDEST_VERSION).contains(&v) => {
            return Err(JobError::TooOld { found: v });
        }
        _ => {
            return Err(JobError::UnsupportedVersion {
                found: describe(raw.as_ref()),
                current: CURRENT_VERSION,
            });
        }
    };
    for step in version..CURRENT_VERSION {
        let index = usize::try_from(step - OLDEST_VERSION).unwrap_or(usize::MAX);
        let Some(migration) = MIGRATIONS.get(index) else {
            return Err(JobError::UnsupportedVersion {
                found: step.to_string(),
                current: CURRENT_VERSION,
            });
        };
        data = migration(data).map_err(|reason| JobError::CannotMigrate { from: step, reason })?;
    }
    Ok(data)
}

/// The version as found in the file, for the error message.
fn describe(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => "missing".into(),
        Some(other) => other.to_string(),
    }
}

fn operations_mut(data: &mut Value) -> Vec<&mut Map<String, Value>> {
    match data.get_mut("operations") {
        Some(Value::Array(ops)) => ops.iter_mut().filter_map(Value::as_object_mut).collect(),
        _ => Vec::new(),
    }
}

/// v5 adds drawn shapes (circle, rectangle, polyline; docs/specs/drawing-tools.md). A v4
/// file only holds imported `path` shapes, which v5 reads unchanged: nothing to convert.
/// The bump exists so an older Chipload refuses v5 files as "newer" instead of tripping
/// over an unknown shape kind.
fn v4_to_v5(data: Value) -> Result<Value, String> {
    Ok(set_version(data, 5))
}

/// v6 removes what only jobs from before projects used, and the drill operation, which was
/// never implemented (decision 2026-10-10). v5 files write `drawing.external` and
/// `select.handles` as null; those keys are dropped. A file that really uses them, or has a
/// drill operation, is refused: dropping them would change what it means.
fn v5_to_v6(mut data: Value) -> Result<Value, String> {
    if let Some(Value::Object(drawing)) = data.get_mut("drawing") {
        match drawing.remove("external") {
            None | Some(Value::Null) => {}
            Some(_) => {
                return Err(
                    "its drawing is not embedded (a job from before projects); open \
                            it with Chipload 0.3.0, embed the drawing and save it"
                        .into(),
                );
            }
        }
    }
    for (i, op) in operations_mut(&mut data).into_iter().enumerate() {
        if op.get("type").and_then(Value::as_str) == Some("drill") {
            return Err(format!(
                "operation {} is a drill operation, which this version doesn't have",
                i + 1
            ));
        }
        if let Some(Value::Object(select)) = op.get_mut("select") {
            match select.remove("handles") {
                None | Some(Value::Null) => {}
                Some(Value::Array(list)) if list.is_empty() => {}
                Some(_) => {
                    return Err(format!(
                        "operation {} selects DXF handles, which only jobs from before \
                         projects used; open it with Chipload 0.3.0 and save it",
                        i + 1
                    ));
                }
            }
        }
    }
    Ok(set_version(data, 6))
}

fn set_version(mut data: Value, version: i64) -> Value {
    if let Value::Object(map) = &mut data {
        map.insert("schema_version".into(), Value::from(version));
    }
    data
}
