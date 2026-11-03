// SPDX-License-Identifier: GPL-3.0-or-later
//! Materials in a project (docs/specs/materials.md, docs/specs/cutting-data.md): each
//! operation names the material its feeds come from; the values are its tool's row of cutting
//! data for that material (else for any material), and an operation's values can be saved
//! back into that row.

use chipload_i18n::{Digits, Message, Value};
use chipload_job::library::{Cutting, LibraryTool, ToolLibrary};
use chipload_job::materials::{Material, MaterialLibrary};
use chipload_job::{Job, MaterialRef, Operation};

use crate::PipelineError;

/// Stepover for saved values when neither the operation (not a pocket) nor anything else
/// gives one: a new pocket's.
pub const DEFAULT_STEPOVER: f64 = 0.4;

/// The project's material in the library, if it names one that is there: what new
/// operations take.
pub fn project_material<'a>(job: &Job, materials: &'a MaterialLibrary) -> Option<&'a Material> {
    materials.find_uid(&job.stock().material.as_ref()?.library_uid)
}

/// Operation `index`'s material (0-based) in the library, if it names one that is there.
pub fn op_material<'a>(
    job: &Job,
    materials: &'a MaterialLibrary,
    index: usize,
) -> Option<&'a Material> {
    materials.find_uid(&job.operations.get(index)?.material()?.library_uid)
}

/// The library entry project tool `tool` is linked to, if it is in `tools`.
pub fn library_tool<'a>(job: &Job, tools: &'a ToolLibrary, tool: i64) -> Option<&'a LibraryTool> {
    tools.find_uid(job.tool(tool)?.library_uid.as_deref()?)
}

/// The values project tool `tool` cuts `material` with (None: no material): its library
/// entry's row for the material, else its row for any material. None when the tool isn't
/// from the library or has neither row.
pub fn tool_values(
    job: &Job,
    tools: &ToolLibrary,
    material: Option<&Material>,
    tool: i64,
) -> Option<Cutting> {
    library_tool(job, tools, tool)?.cutting_for(material.map(|m| m.uid.as_str()))
}

/// The values a new operation with project tool `tool` takes: for the project's material,
/// else for any material.
pub fn material_values(
    job: &Job,
    materials: &MaterialLibrary,
    tools: &ToolLibrary,
    tool: i64,
) -> Option<Cutting> {
    tool_values(job, tools, project_material(job, materials), tool)
}

/// How a library material is named in a project.
pub fn material_ref(material: &Material) -> MaterialRef {
    MaterialRef {
        name: material.name.clone(),
        library_uid: material.uid.clone(),
    }
}

/// Where an operation's values stand against its own material.
#[derive(Debug, Clone, PartialEq)]
pub enum MaterialState {
    /// The operation has no material, or its material isn't in this library.
    NoMaterial,
    /// The operation cuts with its tool's row for the material.
    From(String),
    /// The tool's row for the material has other values.
    Changed(String, Cutting),
    /// The tool has no row for the material.
    NotIn(String),
}

/// How operation `index`'s values (0-based) compare with its tool's row for its material.
pub fn material_state(
    job: &Job,
    materials: &MaterialLibrary,
    tools: &ToolLibrary,
    index: usize,
) -> MaterialState {
    let (Some(material), Some(op)) = (
        op_material(job, materials, index),
        job.operations.get(index),
    ) else {
        return MaterialState::NoMaterial;
    };
    let name = material.name.clone();
    let row = library_tool(job, tools, op.tool()).and_then(|t| t.row(Some(&material.uid)));
    match row.map(|r| r.values) {
        Some(c) if op.has_cutting(&c) => MaterialState::From(name),
        Some(c) => MaterialState::Changed(name, c),
        None => MaterialState::NotIn(name),
    }
}

/// The speed of operation `index`'s row for its own material when it differs from its
/// project tool's (docs/specs/cutting-data.md §6): (the row's, the tool's), rpm.
pub fn speed_differs(
    job: &Job,
    materials: &MaterialLibrary,
    tools: &ToolLibrary,
    index: usize,
) -> Option<(f64, f64)> {
    let material = op_material(job, materials, index)?;
    let tool = job.operations.get(index)?.tool();
    let row = tool_values(job, tools, Some(material), tool)?.rpm?;
    let runs = job.tool(tool)?.rpm?;
    (row != runs).then_some((row, runs))
}

/// The project with `material` (None: none) for the project and every operation, as one
/// change: new operations take it, every operation takes its tool's values for it, and
/// every project tool its row's speed (docs/specs/cutting-data.md §6). The stock is left as
/// it is. One message per change, and per operation left as it was.
pub fn use_material(
    job: &Job,
    tools: &ToolLibrary,
    material: Option<&Material>,
) -> (Job, Vec<Message>) {
    let mut out = job.clone();
    let mut messages = Vec::new();
    let Some(material) = material else {
        let old = out.stock_mut().material.take();
        for op in &mut out.operations {
            op.set_material(None);
        }
        if let Some(old) = old {
            messages.push(Message::new("pipeline-material-none").arg("name", old.name));
        }
        return (out, messages);
    };
    out.stock_mut().material = Some(material_ref(material));
    messages.push(Message::new("pipeline-material-chosen").arg("name", material.name.as_str()));
    for index in 0..out.operations.len() {
        if let Some(message) = set_op_material(&mut out, tools, index, material) {
            messages.push(message);
        }
    }
    for i in 0..out.tools.len() {
        let id = out.tools[i].id;
        let speed = tool_values(&out, tools, Some(material), id).and_then(|c| c.rpm);
        if let Some(rpm) = speed.filter(|rpm| out.tools[i].rpm != Some(*rpm)) {
            out.tools[i].rpm = Some(rpm);
            messages.push(
                Message::new("pipeline-material-tool-speed")
                    .arg("tool", id)
                    .arg("rpm", Value::Number(rpm, Digits::Shortest))
                    .arg("material", material.name.as_str()),
            );
        }
    }
    (out, messages)
}

/// The project with `material` (None: none) for operation `index` (0-based) alone: it takes
/// its tool's values for the material. The project's material, the tools' speeds and the
/// stock are left as they are.
pub fn use_material_for_op(
    job: &Job,
    tools: &ToolLibrary,
    index: usize,
    material: Option<&Material>,
) -> Result<(Job, Vec<Message>), PipelineError> {
    let count = job.operations.len();
    let mut out = job.clone();
    let op = out
        .operations
        .get_mut(index)
        .ok_or_else(|| crate::no_operation(index + 1, count))?;
    let Some(material) = material else {
        op.set_material(None);
        return Ok((out, Vec::new()));
    };
    let messages = set_op_material(&mut out, tools, index, material)
        .into_iter()
        .collect();
    Ok((out, messages))
}

/// Operation `index` names `material` and takes its tool's values for it: a message when
/// its feeds change, or when the tool has no values for it.
fn set_op_material(
    job: &mut Job,
    tools: &ToolLibrary,
    index: usize,
    material: &Material,
) -> Option<Message> {
    let tool = job.operations.get(index)?.tool();
    let values = tool_values(job, tools, Some(material), tool);
    let op = job.operations.get_mut(index)?;
    op.set_material(Some(material_ref(material)));
    let number = index + 1;
    match values {
        Some(c) if op.has_cutting(&c) => None,
        Some(c) => {
            op.apply_cutting(&c);
            Some(op_message(
                "pipeline-material-op-set",
                number,
                op,
                &material.name,
            ))
        }
        None => Some(
            op_message("pipeline-material-op-kept", number, op, &material.name).arg("tool", tool),
        ),
    }
}

/// After operation `index`'s tool changed: it takes the new tool's values for its material,
/// if the tool has them. Whether its values changed.
pub fn follow_material(
    job: &mut Job,
    materials: &MaterialLibrary,
    tools: &ToolLibrary,
    index: usize,
) -> bool {
    let values = op_material(job, materials, index).and_then(|material| {
        let tool = job.operations.get(index)?.tool();
        tool_values(job, tools, Some(material), tool)
    });
    match (values, job.operations.get_mut(index)) {
        (Some(values), Some(op)) if !op.has_cutting(&values) => {
            op.apply_cutting(&values);
            true
        }
        _ => false,
    }
}

fn op_message(id: &'static str, number: usize, op: &Operation, material: &str) -> Message {
    Message::new(id)
        .arg("op", number)
        .arg("name", op.name())
        .arg("material", material)
}

/// A first row from an operation alone: its own values for both kinds of cut, as an old
/// set of values became a row (docs/specs/cutting-data.md §8); a step down it hasn't got
/// (a drill) is a new operation's.
fn first_row(op: &Operation) -> Cutting {
    let own = op.cutting_over(Cutting::uniform(0.0, 0.0, 0.0, DEFAULT_STEPOVER));
    let either = |a: f64, b: f64, default: f64| {
        if a > 0.0 {
            a
        } else if b > 0.0 {
            b
        } else {
            default
        }
    };
    Cutting {
        slot_feed: either(own.slot_feed, own.feed, 0.0),
        slot_step_down: either(own.slot_step_down, own.step_down, crate::DEFAULT_STEP_DOWN),
        feed: either(own.feed, own.slot_feed, 0.0),
        step_down: either(own.step_down, own.slot_step_down, crate::DEFAULT_STEP_DOWN),
        ..own
    }
}

/// What saving operation `number`'s values (1-based) into its tool's row for `material`
/// writes: the library tool's uid and the row's values (docs/specs/cutting-data.md §7).
/// The values the operation doesn't take (a profile's clearing values, a hole's step downs)
/// are kept from the row, else the tool's row for any material, else a new operation's; the
/// speed is the project tool's when it differs from the library tool's.
pub fn values_to_save(
    job: &Job,
    number: usize,
    tools: &ToolLibrary,
    material: &Material,
) -> Result<(String, Cutting), PipelineError> {
    let count = job.operations.len();
    let op = number
        .checked_sub(1)
        .and_then(|i| job.operations.get(i))
        .ok_or_else(|| crate::no_operation(number, count))?;
    let tool = op.tool();
    let not_in_library = || {
        PipelineError::Job(Message::new("pipeline-material-tool-not-in-library").arg("tool", tool))
    };
    let entry = library_tool(job, tools, tool).ok_or_else(not_in_library)?;
    let base = entry
        .cutting_for(Some(&material.uid))
        .unwrap_or_else(|| first_row(op));
    let mut values = op.cutting_over(base);
    let runs = job.tool(tool).and_then(|t| t.rpm);
    if runs != entry.rpm {
        values.rpm = runs;
    }
    Ok((entry.uid.clone(), values))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chipload_job::library::LibraryTool;

    const UID: &str = "a1b2c3d4e5f60718";

    fn ply() -> Material {
        Material {
            name: "Plywood".into(),
            uid: "3f9c0a1b2d4e5f60".into(),
        }
    }

    fn values() -> Cutting {
        Cutting {
            rpm: None,
            plunge_feed: 500.0,
            slot_feed: 1100.0,
            slot_step_down: 2.0,
            feed: 1500.0,
            step_down: 4.0,
            stepover: 0.45,
        }
    }

    /// A pocket and a profile cutting with tool 1, linked to the library tool UID.
    fn job() -> Job {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/corpus/jobs/v31/svg_badge.chipload");
        let mut job = chipload_job::load_job(&path).unwrap();
        job.tools[0].library_uid = Some(UID.into());
        job
    }

    /// The library with tool 1's entry, and its rows for `rows` (material uid, values).
    fn tools(rows: &[(Option<&str>, Cutting)]) -> ToolLibrary {
        let mut tools = ToolLibrary::default();
        let mut entry = LibraryTool::from_tool("6 mm", UID, &job().tools[0], Vec::new());
        for (material, values) in rows {
            entry.set_row(*material, *values);
        }
        tools.tools.push(entry);
        tools
    }

    fn pocket(job: &Job) -> &chipload_job::PocketOp {
        job.operations
            .iter()
            .find_map(|op| match op {
                Operation::Pocket(p) => Some(p),
                _ => None,
            })
            .unwrap()
    }

    #[test]
    fn choosing_a_material_fills_every_operation_with_slot_or_clearing_values() {
        let material = ply();
        let library = tools(&[(Some(&material.uid), values())]);
        let (out, messages) = use_material(&job(), &library, Some(&material));
        assert_eq!(out.stock(), &{
            let mut stock = job().stock().clone();
            stock.material = Some(material_ref(&material));
            stock
        });
        assert!(out.operations.iter().all(|op| op.has_cutting(&values())));
        // The pocket takes both, the profile the slot values.
        let p = pocket(&out);
        assert_eq!((p.feed, p.step_down, p.stepover), (1500.0, 4.0, 0.45));
        assert_eq!((p.slot_feed, p.slot_step_down), (Some(1100.0), Some(2.0)));
        for op in &out.operations {
            if let Operation::Profile(f) = op {
                assert_eq!((f.feed, f.step_down), (1100.0, 2.0));
            }
        }
        let text: Vec<String> = messages.iter().map(Message::english).collect();
        assert_eq!(text.len(), 3, "chosen, two operations: {text:?}");
        for (a, b) in out.operations.iter().zip(&job().operations) {
            assert_eq!(a.depths(), b.depths());
        }
        let (again, messages) = use_material(&out, &library, Some(&material));
        assert_eq!(again, out);
        assert_eq!(messages.len(), 1, "only 'material chosen'");
        let (none, _) = use_material(&out, &library, None);
        assert_eq!(none.stock().material, None);
        assert!(none.operations.iter().all(|op| op.material().is_none()));
        assert!(none.operations.iter().all(|op| op.has_cutting(&values())));
    }

    #[test]
    fn without_a_row_for_the_material_the_row_for_any_material_is_used() {
        let any = Cutting::uniform(900.0, 300.0, 1.5, 0.4);
        let library = tools(&[(None, any)]);
        let (out, _) = use_material(&job(), &library, Some(&ply()));
        assert!(out.operations.iter().all(|op| op.has_cutting(&any)));
        // ...but the operation's state says the tool has no Plywood row.
        let mut materials = MaterialLibrary::default();
        materials.materials.push(ply());
        assert_eq!(
            material_state(&out, &materials, &library, 0),
            MaterialState::NotIn("Plywood".into())
        );
        // Neither row: the feeds stay, the report says so.
        let (kept, messages) = use_material(&job(), &tools(&[]), Some(&ply()));
        assert_eq!(kept.operations, {
            let mut ops = job().operations;
            for op in &mut ops {
                op.set_material(Some(material_ref(&ply())));
            }
            ops
        });
        let text: Vec<String> = messages.iter().map(Message::english).collect();
        assert!(text.iter().any(|t| t.contains("tool 1")), "{text:?}");
    }

    #[test]
    fn a_project_material_sets_the_tools_speed_and_an_operations_own_does_not() {
        let material = ply();
        let fast = Cutting {
            rpm: Some(20000.0),
            ..values()
        };
        let library = tools(&[(Some(&material.uid), fast)]);
        let (out, messages) = use_material(&job(), &library, Some(&material));
        assert_eq!(out.tools[0].rpm, Some(20000.0));
        let text: Vec<String> = messages.iter().map(Message::english).collect();
        assert!(text.iter().any(|t| t.contains("20000")), "{text:?}");
        let (one, _) = use_material_for_op(&job(), &library, 1, Some(&material)).unwrap();
        assert_eq!(one.tools[0].rpm, job().tools[0].rpm);
        let mut materials = MaterialLibrary::default();
        materials.materials.push(material);
        assert_eq!(
            speed_differs(&one, &materials, &library, 1),
            Some((20000.0, job().tools[0].rpm.unwrap()))
        );
        assert_eq!(speed_differs(&out, &materials, &library, 1), None);
    }

    #[test]
    fn one_operation_takes_its_own_material() {
        let material = ply();
        let library = tools(&[(Some(&material.uid), values())]);
        let (out, messages) = use_material_for_op(&job(), &library, 1, Some(&material)).unwrap();
        assert_eq!(out.stock(), job().stock(), "the project's material stays");
        assert!(out.operations[1].has_cutting(&values()));
        assert_eq!(out.operations[0], job().operations[0]);
        assert_eq!(messages.len(), 1);
        let (none, _) = use_material_for_op(&out, &library, 1, None).unwrap();
        assert_eq!(none.operations[1].material(), None);
        assert!(none.operations[1].has_cutting(&values()), "feeds stay");
        assert!(use_material_for_op(&job(), &library, 9, Some(&material)).is_err());
    }

    #[test]
    fn a_new_tool_takes_its_values_for_the_operations_material() {
        let material = ply();
        let mut materials = MaterialLibrary::default();
        materials.materials.push(material.clone());
        let mut library = tools(&[]);
        let mut second =
            LibraryTool::from_tool("3 mm", "b000000000000002", &job().tools[0], Vec::new());
        second.set_row(Some(&material.uid), values());
        library.tools.push(second);
        let (mut job, _) = use_material_for_op(&job(), &library, 0, Some(&material)).unwrap();
        let before = job.operations[0].clone();
        // Tool 1 has no values for Plywood: nothing to follow.
        assert!(!follow_material(&mut job, &materials, &library, 0));
        assert_eq!(job.operations[0], before);
        let mut tool = job.tools[0].clone();
        tool.id = 2;
        tool.library_uid = Some("b000000000000002".into());
        job.tools.push(tool);
        job.operations[0].set_tool(2);
        assert!(follow_material(&mut job, &materials, &library, 0));
        assert!(job.operations[0].has_cutting(&values()));
        // An operation without a material keeps its feeds.
        job.operations[1].set_tool(2);
        let before = job.operations[1].clone();
        assert!(!follow_material(&mut job, &materials, &library, 1));
        assert_eq!(job.operations[1], before);
    }

    #[test]
    fn saving_writes_the_values_the_operation_takes_into_its_row() {
        let mut materials = MaterialLibrary::default();
        materials.materials.push(ply());
        let library = tools(&[]);
        let (job, _) = use_material(&job(), &library, Some(&materials.materials[0]));
        assert_eq!(
            material_state(&job, &materials, &library, 0),
            MaterialState::NotIn("Plywood".into())
        );
        // The pocket: no row yet, so its own values for both kinds of cut.
        let p = pocket(&job).clone();
        let (uid, saved) = values_to_save(&job, 1, &library, &materials.materials[0]).unwrap();
        assert_eq!(uid, UID);
        assert_eq!(
            (
                saved.feed,
                saved.step_down,
                saved.stepover,
                saved.slot_feed,
                saved.slot_step_down
            ),
            (p.feed, p.step_down, p.stepover, p.feed, p.step_down)
        );
        assert!(
            saved.problems("row").is_empty(),
            "{:?}",
            saved.problems("row")
        );
        let mut library = library;
        library.tools[0].set_row(Some(&materials.materials[0].uid), saved);
        assert_eq!(
            material_state(&job, &materials, &library, 0),
            MaterialState::From("Plywood".into())
        );
        // The profile, saved over the row: its slot values only, the clearing ones stay.
        let (_, profile) = values_to_save(&job, 2, &library, &materials.materials[0]).unwrap();
        assert_eq!(
            (profile.feed, profile.step_down),
            (saved.feed, saved.step_down)
        );
        // Not in the library: refused.
        let mut other = job.clone();
        other.tools[0].library_uid = None;
        assert!(values_to_save(&other, 1, &library, &materials.materials[0]).is_err());
        assert!(values_to_save(&job, 9, &library, &materials.materials[0]).is_err());
    }
}
