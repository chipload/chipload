// SPDX-License-Identifier: GPL-3.0-or-later
//! Materials in a project (docs/specs/materials.md): each operation names the material its
//! feeds come from, the user's own values per tool, and its values can be saved back.

use chipload_i18n::Message;
use chipload_job::library::{Cutting, ToolLibrary};
use chipload_job::materials::{Material, MaterialLibrary};
use chipload_job::{Job, MaterialRef, Operation};

use crate::PipelineError;

/// Stepover for saved values when neither the operation (not a pocket) nor anything else
/// gives one: a new pocket's.
pub const DEFAULT_STEPOVER: f64 = 0.4;

/// The project's material in the library, if it names one that is there: what new
/// operations take.
pub fn project_material<'a>(job: &Job, materials: &'a MaterialLibrary) -> Option<&'a Material> {
    materials.find_uid(&job.stock.material.as_ref()?.library_uid)
}

/// Operation `index`'s material (0-based) in the library, if it names one that is there.
pub fn op_material<'a>(
    job: &Job,
    materials: &'a MaterialLibrary,
    index: usize,
) -> Option<&'a Material> {
    materials.find_uid(&job.operations.get(index)?.material()?.library_uid)
}

/// `material`'s values for project tool `tool`: None when the tool isn't from the library
/// or has no values in it.
pub fn tool_values(job: &Job, material: &Material, tool: i64) -> Option<Cutting> {
    material.values(job.tool(tool)?.library_uid.as_deref()?)
}

/// The project material's values for project tool `tool`, for a new operation: None
/// without a material, or when the tool has no values in it.
pub fn material_values(job: &Job, materials: &MaterialLibrary, tool: i64) -> Option<Cutting> {
    tool_values(job, project_material(job, materials)?, tool)
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
    /// The operation cuts with the material's values for its tool.
    From(String),
    /// The material has other values for the operation's tool.
    Changed(String, Cutting),
    /// The material has no values for the operation's tool.
    NotIn(String),
}

/// How operation `index`'s values (0-based) compare with its material.
pub fn material_state(job: &Job, materials: &MaterialLibrary, index: usize) -> MaterialState {
    let (Some(material), Some(op)) = (
        op_material(job, materials, index),
        job.operations.get(index),
    ) else {
        return MaterialState::NoMaterial;
    };
    let name = material.name.clone();
    match tool_values(job, material, op.tool()) {
        Some(c) if op.has_cutting(&c) => MaterialState::From(name),
        Some(c) => MaterialState::Changed(name, c),
        None => MaterialState::NotIn(name),
    }
}

/// The project with `material` (None: none) for the project and every operation, as one
/// change: new operations take it, and every operation whose tool has values in it takes
/// them. The stock is left as it is. One message per change, and per operation left as it
/// was.
pub fn use_material(job: &Job, material: Option<&Material>) -> (Job, Vec<Message>) {
    let mut out = job.clone();
    let mut messages = Vec::new();
    let Some(material) = material else {
        let old = out.stock.material.take();
        for op in &mut out.operations {
            op.set_material(None);
        }
        if let Some(old) = old {
            messages.push(Message::new("pipeline-material-none").arg("name", old.name));
        }
        return (out, messages);
    };
    out.stock.material = Some(material_ref(material));
    messages.push(Message::new("pipeline-material-chosen").arg("name", material.name.as_str()));
    for index in 0..out.operations.len() {
        if let Some(message) = set_op_material(&mut out, index, material) {
            messages.push(message);
        }
    }
    (out, messages)
}

/// The project with `material` (None: none) for operation `index` (0-based) alone: it takes
/// the material's values for its tool. The project's material and the stock are left as
/// they are.
pub fn use_material_for_op(
    job: &Job,
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
    let messages = set_op_material(&mut out, index, material)
        .into_iter()
        .collect();
    Ok((out, messages))
}

/// Operation `index` names `material` and takes its values for its tool: a message when
/// its feeds change, or when the tool has no values there.
fn set_op_material(job: &mut Job, index: usize, material: &Material) -> Option<Message> {
    let tool = job.operations.get(index)?.tool();
    let values = tool_values(job, material, tool);
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

/// After operation `index`'s tool changed: it takes its material's values for the new
/// tool, if the material has them. Whether its values changed.
pub fn follow_material(job: &mut Job, materials: &MaterialLibrary, index: usize) -> bool {
    let values = op_material(job, materials, index).and_then(|material| {
        let tool = job.operations.get(index)?.tool();
        tool_values(job, material, tool)
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

/// What saving operation `number`'s values (1-based) writes: the library tool's uid and
/// the values. The values the operation hasn't got (a profile's stepover, a hole's step
/// down) are kept from the material's values for the tool, else the library tool's own,
/// else a new operation's.
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
    let uid = job
        .tool(tool)
        .and_then(|t| t.library_uid.clone())
        .filter(|uid| tools.find_uid(uid).is_some())
        .ok_or_else(|| {
            PipelineError::Job(
                Message::new("pipeline-material-tool-not-in-library").arg("tool", tool),
            )
        })?;
    let base = material
        .values(&uid)
        .or_else(|| tools.find_uid(&uid).and_then(|t| t.cutting))
        .unwrap_or(Cutting {
            feed: 0.0,
            plunge_feed: 0.0,
            step_down: crate::DEFAULT_STEP_DOWN,
            stepover: DEFAULT_STEPOVER,
        });
    Ok((uid, op.cutting_over(base)))
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
            tools: Vec::new(),
        }
    }

    fn values() -> Cutting {
        Cutting {
            feed: 1500.0,
            plunge_feed: 500.0,
            step_down: 2.5,
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

    #[test]
    fn choosing_a_material_fills_every_operation_and_leaves_the_stock() {
        let mut material = ply();
        material.set_values(UID, values());
        let (out, messages) = use_material(&job(), Some(&material));
        assert_eq!(out.stock, {
            let mut stock = job().stock;
            stock.material = Some(material_ref(&material));
            stock
        });
        assert!(out.operations.iter().all(|op| op.has_cutting(&values())));
        assert!(
            out.operations
                .iter()
                .all(|op| op.material().map(|m| m.name.as_str()) == Some("Plywood"))
        );
        let text: Vec<String> = messages.iter().map(Message::english).collect();
        // Chosen, two operations' feeds.
        assert_eq!(text.len(), 3, "{text:?}");
        // Depths stay: the thickness didn't change.
        for (a, b) in out.operations.iter().zip(&job().operations) {
            assert_eq!(a.depths(), b.depths());
        }
        // Chosen again: nothing more changes.
        let (again, messages) = use_material(&out, Some(&material));
        assert_eq!(again, out);
        assert_eq!(messages.len(), 1, "only 'material chosen'");
        // None: the materials go, the feeds stay.
        let (none, _) = use_material(&out, None);
        assert_eq!(none.stock.material, None);
        assert!(none.operations.iter().all(|op| op.material().is_none()));
        for (a, b) in none.operations.iter().zip(&out.operations) {
            assert!(a.has_cutting(&b.cutting_over(values())));
        }
    }

    #[test]
    fn one_operation_takes_its_own_material() {
        let mut material = ply();
        material.set_values(UID, values());
        let (out, messages) = use_material_for_op(&job(), 1, Some(&material)).unwrap();
        assert_eq!(out.stock, job().stock, "the project's material stays");
        assert!(out.operations[1].has_cutting(&values()));
        assert_eq!(out.operations[0], job().operations[0]);
        assert_eq!(messages.len(), 1);
        let (none, _) = use_material_for_op(&out, 1, None).unwrap();
        assert_eq!(none.operations[1].material(), None);
        assert!(none.operations[1].has_cutting(&values()), "feeds stay");
        assert!(use_material_for_op(&job(), 9, Some(&material)).is_err());
    }

    #[test]
    fn a_new_tool_takes_the_operations_materials_values() {
        let mut materials = MaterialLibrary::default();
        let mut material = ply();
        material.set_values("b000000000000002", values());
        materials.materials.push(material.clone());
        let (mut job, _) = use_material_for_op(&job(), 0, Some(&material)).unwrap();
        let before = job.operations[0].clone();
        // Tool 1 has no values in Plywood: nothing to follow.
        assert!(!follow_material(&mut job, &materials, 0));
        assert_eq!(job.operations[0], before);
        // Tool 2, which has.
        let mut second = job.tools[0].clone();
        second.id = 2;
        second.library_uid = Some("b000000000000002".into());
        job.tools.push(second);
        job.operations[0].set_tool(2);
        assert!(follow_material(&mut job, &materials, 0));
        assert!(job.operations[0].has_cutting(&values()));
        // An operation without a material keeps its feeds.
        job.operations[1].set_tool(2);
        let before = job.operations[1].clone();
        assert!(!follow_material(&mut job, &materials, 1));
        assert_eq!(job.operations[1], before);
    }

    #[test]
    fn a_tool_without_values_keeps_its_feeds_and_says_so() {
        let (out, messages) = use_material(&job(), Some(&ply()));
        for (a, b) in out.operations.iter().zip(&job().operations) {
            assert!(a.has_cutting(&b.cutting_over(values())));
        }
        let text: Vec<String> = messages.iter().map(Message::english).collect();
        assert!(text.iter().any(|t| t.contains("tool 1")), "{text:?}");
    }

    #[test]
    fn the_state_and_saving_follow_the_operations_values() {
        let mut materials = MaterialLibrary::default();
        materials.materials.push(ply());
        let (job, _) = use_material(&job(), Some(&materials.materials[0]));
        assert_eq!(
            material_state(&job, &materials, 0),
            MaterialState::NotIn("Plywood".into())
        );
        let mut tools = ToolLibrary::default();
        let mut entry = LibraryTool::from_tool("6 mm", UID, &job.tools[0], None);
        entry.uid = UID.into();
        tools.tools.push(entry);
        // The pocket's values, saved: its own feeds, step down and stepover.
        let (uid, saved) = values_to_save(&job, 1, &tools, &materials.materials[0]).unwrap();
        assert_eq!(uid, UID);
        assert_eq!(
            (saved.feed, saved.step_down, saved.stepover),
            (1200.0, 2.0, 0.4)
        );
        materials.materials[0].set_values(&uid, saved);
        assert_eq!(
            material_state(&job, &materials, 0),
            MaterialState::From("Plywood".into())
        );
        // The profile cuts with other values.
        assert_eq!(
            material_state(&job, &materials, 1),
            MaterialState::Changed("Plywood".into(), saved)
        );
        // A tool that isn't in the library: refused.
        let mut other = job.clone();
        other.tools[0].library_uid = None;
        assert!(values_to_save(&other, 1, &tools, &materials.materials[0]).is_err());
        assert!(values_to_save(&job, 9, &tools, &materials.materials[0]).is_err());
    }
}
