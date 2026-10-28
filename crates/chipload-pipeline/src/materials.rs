// SPDX-License-Identifier: GPL-3.0-or-later
//! Materials in a project (docs/specs/materials.md): choosing one fills the operations'
//! feeds from the user's own values per tool, and an operation's values can be saved back.

use chipload_i18n::{Digits, Message, Value};
use chipload_job::library::{Cutting, ToolLibrary};
use chipload_job::materials::{Material, MaterialLibrary};
use chipload_job::{Job, MaterialRef, Operation};

use crate::PipelineError;

/// Stepover for saved values when neither the operation (not a pocket) nor anything else
/// gives one: a new pocket's.
pub const DEFAULT_STEPOVER: f64 = 0.4;

/// The project's material in the library, if it names one that is there.
pub fn project_material<'a>(job: &Job, materials: &'a MaterialLibrary) -> Option<&'a Material> {
    materials.find_uid(&job.stock.material.as_ref()?.library_uid)
}

/// The project material's values for project tool `tool`: None without a material, or
/// when the tool isn't from the library or has no values in it.
pub fn material_values(job: &Job, materials: &MaterialLibrary, tool: i64) -> Option<Cutting> {
    let material = project_material(job, materials)?;
    material.values(job.tool(tool)?.library_uid.as_deref()?)
}

/// Where an operation's values stand against the project's material.
#[derive(Debug, Clone, PartialEq)]
pub enum MaterialState {
    /// The project has no material, or its material isn't in this library.
    NoMaterial,
    /// The operation cuts with the material's values for its tool.
    From(String),
    /// The material has other values for the operation's tool.
    Changed(String, Cutting),
    /// The material has no values for the operation's tool.
    NotIn(String),
}

/// How operation `index`'s values (0-based) compare with the project's material.
pub fn material_state(job: &Job, materials: &MaterialLibrary, index: usize) -> MaterialState {
    let (Some(material), Some(op)) = (project_material(job, materials), job.operations.get(index))
    else {
        return MaterialState::NoMaterial;
    };
    let name = material.name.clone();
    match material_values(job, materials, op.tool()) {
        Some(c) if op.has_cutting(&c) => MaterialState::From(name),
        Some(c) => MaterialState::Changed(name, c),
        None => MaterialState::NotIn(name),
    }
}

/// The project with `material` chosen (None: none), as one change: the stock takes its
/// thickness, and every operation whose tool has values in it takes them. One message per
/// change, and per operation left as it was.
pub fn use_material(job: &Job, material: Option<&Material>) -> (Job, Vec<Message>) {
    let mut out = job.clone();
    let mut messages = Vec::new();
    let Some(material) = material else {
        if let Some(old) = out.stock.material.take() {
            messages.push(Message::new("pipeline-material-none").arg("name", old.name));
        }
        return (out, messages);
    };
    out.stock.material = Some(MaterialRef {
        name: material.name.clone(),
        library_uid: material.uid.clone(),
    });
    messages.push(Message::new("pipeline-material-chosen").arg("name", material.name.as_str()));
    if let Some(thickness) = material.thickness
        && thickness != out.stock.thickness
    {
        let before = out.stock.thickness;
        out.stock.thickness = thickness;
        messages.push(
            Message::new("pipeline-material-thickness")
                .arg("thickness", Value::Length(thickness, Digits::Shortest)),
        );
        // A cut that went through the old board still goes through the new one, as far
        // below it as before (the user tried 18 mm ply on a 12.3 mm project, 2026-10-28).
        for (i, op) in out.operations.iter_mut().enumerate() {
            let (_, end) = op.depths();
            if end >= before {
                let depth = thickness + (end - before);
                op.set_end_depth(depth);
                messages.push(
                    op_message("pipeline-material-op-through", i + 1, op, &material.name)
                        .arg("depth", Value::Length(depth, Digits::Shortest)),
                );
            }
        }
    }
    let tools = out.tools.clone();
    for (i, op) in out.operations.iter_mut().enumerate() {
        let tool = tools.iter().find(|t| t.id == op.tool());
        let values = tool
            .and_then(|t| t.library_uid.as_deref())
            .and_then(|uid| material.values(uid));
        let number = i + 1;
        match values {
            Some(c) if op.has_cutting(&c) => {}
            Some(c) => {
                op.apply_cutting(&c);
                messages.push(op_message(
                    "pipeline-material-op-set",
                    number,
                    op,
                    &material.name,
                ));
            }
            None => messages.push(
                op_message("pipeline-material-op-kept", number, op, &material.name)
                    .arg("tool", op.tool()),
            ),
        }
    }
    (out, messages)
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
            thickness: Some(18.0),
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
    fn choosing_a_material_takes_its_thickness_and_fills_the_operations() {
        let mut material = ply();
        material.set_values(UID, values());
        let (out, messages) = use_material(&job(), Some(&material));
        assert_eq!(out.stock.thickness, 18.0);
        assert_eq!(out.stock.material.as_ref().unwrap().name, "Plywood");
        assert!(out.operations.iter().all(|op| op.has_cutting(&values())));
        let text: Vec<String> = messages.iter().map(Message::english).collect();
        // Chosen, thickness, the profile still through, two operations' feeds.
        assert_eq!(text.len(), 5, "{text:?}");
        // The badge's outline went 0.3 mm below its 12 mm board: still, below 18 mm.
        assert_eq!(job().stock.thickness, 12.0);
        assert_eq!(out.operations[1].depths().1, 18.3);
        assert_eq!(out.operations[0].depths(), job().operations[0].depths());
        // Chosen again: nothing more changes.
        let (again, messages) = use_material(&out, Some(&material));
        assert_eq!(again, out);
        assert_eq!(messages.len(), 1, "only 'material chosen'");
        // None: the material goes, the feeds stay.
        let (none, _) = use_material(&out, None);
        assert_eq!(none.stock.material, None);
        assert_eq!(none.operations, out.operations);
    }

    #[test]
    fn a_tool_without_values_keeps_its_feeds_and_says_so() {
        // No thickness either: nothing about the cut changes.
        let mut material = ply();
        material.thickness = None;
        let (out, messages) = use_material(&job(), Some(&material));
        assert_eq!(out.operations, job().operations);
        assert_eq!(out.stock.thickness, job().stock.thickness);
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
