// SPDX-License-Identifier: GPL-3.0-or-later
//! Schema migrations. Each step upgrades version N to N+1 and is a pure function (no I/O,
//! no clock). Their effect must never change: frozen samples in tests/corpus/jobs depend on it.
//! Versions before projects (1-3) were never released and are refused (decision 2026-10-10).

use serde_json::{Map, Value};

use crate::JobError;
use crate::models::{CURRENT_VERSION, OLDEST_VERSION};

type Migration = fn(Value) -> Result<Value, String>;

/// `MIGRATIONS[n - OLDEST_VERSION]` upgrades version n to n + 1.
const MIGRATIONS: [Migration; 15] = [
    v4_to_v5, v5_to_v6, v6_to_v7, v7_to_v8, v8_to_v9, v9_to_v10, v10_to_v11, v11_to_v12,
    v12_to_v13, v13_to_v14, v14_to_v15, v15_to_v16, v16_to_v17, v17_to_v18, v18_to_v19,
];

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

/// v7 gives operations a start depth and a clearance and the stock an overcut allowance
/// (docs/specs/depths-and-clearance.md). `depth` becomes `end_depth`; the new values are 0,
/// which is exactly what v6 did: cut from the stock top, on the drawn line plus the tool
/// radius, never below the stock.
fn v6_to_v7(mut data: Value) -> Result<Value, String> {
    if let Some(Value::Object(stock)) = data.get_mut("stock") {
        stock.insert("max_overcut".into(), Value::from(0.0));
    }
    for op in operations_mut(&mut data) {
        if let Some(depth) = op.remove("depth") {
            op.insert("end_depth".into(), depth);
        }
        op.insert("start_depth".into(), Value::from(0.0));
        op.insert("clearance".into(), Value::from(0.0));
    }
    Ok(set_version(data, 7))
}

/// v8 records where the imported drawing is placed (docs/specs/drawing-placement.md). A v7
/// drawing is where its DXF has it: placement (0, 0). Nothing moves.
fn v7_to_v8(mut data: Value) -> Result<Value, String> {
    if let Some(Value::Object(drawing)) = data.get_mut("drawing")
        && let Some(Value::Object(source)) = drawing.get_mut("source")
    {
        let mut placement = Map::new();
        placement.insert("dx".into(), Value::from(0.0));
        placement.insert("dy".into(), Value::from(0.0));
        source.insert("placement".into(), Value::Object(placement));
    }
    Ok(set_version(data, 8))
}

/// v9 lets the imported drawing be turned in 90° steps (docs/specs/drawing-rotation.md). A
/// v8 placement is a move only: 0 turns. Nothing moves.
fn v8_to_v9(mut data: Value) -> Result<Value, String> {
    if let Some(Value::Object(drawing)) = data.get_mut("drawing")
        && let Some(Value::Object(source)) = drawing.get_mut("source")
        && let Some(Value::Object(placement)) = source.get_mut("placement")
    {
        placement.insert("quarter_turns".into(), Value::from(0));
    }
    Ok(set_version(data, 9))
}

/// v10 adds the V-carve operation and a V-bit's `angle` and `tip_diameter`
/// (docs/specs/v-carve.md §9), all optional: a v9 file reads unchanged, and its V-bits get no
/// angle (unknown, never guessed). The bump makes an older Chipload refuse v10 files as
/// "newer" instead of as "unknown field".
fn v9_to_v10(data: Value) -> Result<Value, String> {
    Ok(set_version(data, 10))
}

/// v11 lets the imported drawing be scaled (docs/specs/drawing-scale.md). A v10 placement
/// has no scale: 1. Nothing moves or changes size.
fn v10_to_v11(mut data: Value) -> Result<Value, String> {
    if let Some(Value::Object(drawing)) = data.get_mut("drawing")
        && let Some(Value::Object(source)) = drawing.get_mut("source")
        && let Some(Value::Object(placement)) = source.get_mut("placement")
    {
        placement.insert("scale".into(), Value::from(1.0));
    }
    Ok(set_version(data, 11))
}

/// 11 -> 12: the machine's tool change setting (docs/specs/tool-per-operation.md §3.3).
/// Older files wrote one file per tool group, or refused several tools: "files" keeps that.
fn v11_to_v12(mut data: Value) -> Result<Value, String> {
    if let Some(Value::Object(machine)) = data.get_mut("machine") {
        machine.insert("tool_change".into(), Value::from("files"));
    }
    Ok(set_version(data, 12))
}

/// v13: adaptive pockets cut the pocket's `stepover` wide, and the strategy's own `load` goes
/// (docs/specs/adaptive-fronts.md §5). The stepover becomes the load, so the cut stays as
/// wide as it was: the old stepover was unused by adaptive.
fn v12_to_v13(mut data: Value) -> Result<Value, String> {
    for (i, op) in operations_mut(&mut data).into_iter().enumerate() {
        let Some(Value::Object(strategy)) = op.get_mut("strategy") else {
            continue;
        };
        if strategy.get("type").and_then(Value::as_str) != Some("adaptive") {
            continue;
        }
        let Some(load) = strategy.remove("load").as_ref().and_then(Value::as_f64) else {
            return Err(format!(
                "operations.{i}.strategy: adaptive without a numeric load"
            ));
        };
        op.insert("stepover".into(), Value::from(load));
    }
    Ok(set_version(data, 13))
}

/// v14: rings pockets may round the corners of their inner rings
/// (docs/specs/pocket-round-corners.md). Every older pocket had sharp corners: false, so
/// nothing changes.
fn v13_to_v14(mut data: Value) -> Result<Value, String> {
    for op in operations_mut(&mut data) {
        if op.get("type").and_then(Value::as_str) == Some("pocket") {
            op.insert("round_corners".into(), Value::Bool(false));
        }
    }
    Ok(set_version(data, 14))
}

/// 14 -> 15: a machine may carry a copy of a user post-processor (docs/specs/user-posts.md
/// §3). Every older project uses a built-in post by name: null, the same post, the same
/// G-code.
fn v14_to_v15(mut data: Value) -> Result<Value, String> {
    if let Some(machine) = data.get_mut("machine").and_then(Value::as_object_mut) {
        machine.insert("custom_post".into(), Value::Null);
    }
    Ok(set_version(data, 15))
}

/// 15 -> 16: tabs get a flank angle (docs/specs/profile-spiral.md §7). Tabs of a profile
/// that plunges stay straight (90°): the same G-code. A ramped profile can't keep its old
/// toolpath (the back-and-forth ramps after each tab are gone), so its tabs get the
/// default a new one would: 30°, or the ramp angle when the operation's tool is marked not
/// centre-cutting.
fn v15_to_v16(mut data: Value) -> Result<Value, String> {
    let not_centre_cutting: Vec<i64> = match data.get("tools") {
        Some(Value::Array(tools)) => tools
            .iter()
            .filter(|tool| tool.get("center_cutting") == Some(&Value::Bool(false)))
            .filter_map(|tool| tool.get("id").and_then(Value::as_i64))
            .collect(),
        _ => Vec::new(),
    };
    for (i, op) in operations_mut(&mut data).into_iter().enumerate() {
        if op.get("type").and_then(Value::as_str) != Some("profile") {
            continue;
        }
        let entry = op.get("entry");
        let ramp_angle = match entry.and_then(|e| e.get("type")).and_then(Value::as_str) {
            None | Some("plunge") => None,
            Some("ramp") => match entry
                .and_then(|e| e.get("ramp_angle"))
                .and_then(Value::as_f64)
            {
                Some(angle) => Some(angle),
                None => return Err(format!("operations.{i}.entry: a ramp without an angle")),
            },
            Some(other) => {
                return Err(format!(
                    "operations.{i}.entry: a profile can't have a {other} entry"
                ));
            }
        };
        let tool = op.get("tool").and_then(Value::as_i64);
        let Some(Value::Object(tabs)) = op.get_mut("tabs") else {
            continue;
        };
        let flank = match ramp_angle {
            None => std::f64::consts::FRAC_PI_2,
            Some(angle) if tool.is_some_and(|id| not_centre_cutting.contains(&id)) => angle,
            Some(_) => std::f64::consts::PI / 6.0,
        };
        tabs.insert("flank_angle".into(), Value::from(flank));
    }
    Ok(set_version(data, 16))
}

/// 16 -> 17: the follow-path operation (docs/specs/follow-path.md §2). A v16 file has none;
/// the bump makes an older Chipload refuse v17 files as "newer" instead of tripping over an
/// unknown operation type. Nothing changes.
fn v16_to_v17(data: Value) -> Result<Value, String> {
    Ok(set_version(data, 17))
}

/// 17 -> 18: the Holes operation (docs/specs/holes.md §2). A v17 file has none; the bump
/// makes an older Chipload refuse v18 files as "newer" instead of tripping over an unknown
/// operation type. Nothing changes.
fn v17_to_v18(data: Value) -> Result<Value, String> {
    Ok(set_version(data, 18))
}

/// 18 -> 19: pictures traced into shapes (docs/specs/image-trace.md §4). A v18 file has no
/// images: the drawing gets an empty list. The bump makes Chipload 0.14 refuse v19 files as
/// "newer" instead of dropping their images when it saves. Nothing else changes.
fn v18_to_v19(mut data: Value) -> Result<Value, String> {
    let drawing = data
        .get_mut("drawing")
        .and_then(Value::as_object_mut)
        .ok_or("the project has no drawing")?;
    drawing.insert("images".into(), Value::Array(Vec::new()));
    Ok(set_version(data, 19))
}

fn set_version(mut data: Value, version: i64) -> Value {
    if let Value::Object(map) = &mut data {
        map.insert("schema_version".into(), Value::from(version));
    }
    data
}
