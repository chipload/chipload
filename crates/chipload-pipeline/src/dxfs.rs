// SPDX-License-Identifier: GPL-3.0-or-later
//! A project's DXFs (docs/specs/drawing-sources.md §2): add one, update one from a newer
//! file, remove one, and the note for a layer that several of them share. Each DXF has its
//! own placement (placement.rs). The CLI and the GUI both go through here.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use chipload_dxf::shapes_from_file;
use chipload_i18n::Message;
use chipload_job::{Geometry, Job, Note, Placement, Shape, Units};

use crate::{PipelineError, UpdateReport, ids_text, placement, stock_fit_message};

/// Add `dxf` to the project as a new DXF, where the file has it (moved with the Drawing
/// panel or `place --dxf` afterwards), named `name` or after its file. Its shapes get new
/// ids after the project's.
pub fn add_dxf(
    job: &Job,
    dxf: &Path,
    units: Option<Units>,
) -> Result<(Job, UpdateReport), PipelineError> {
    add_named_dxf(job, dxf, units, None)
}

/// [`add_dxf`] with a name of its own.
pub fn add_named_dxf(
    job: &Job,
    dxf: &Path,
    units: Option<Units>,
    name: Option<&str>,
) -> Result<(Job, UpdateReport), PipelineError> {
    add_drawing_file(job, dxf, units, None, name)
}

/// Add a drawing file, DXF or SVG by its extension (docs/specs/svg-import.md), as
/// [`add_named_dxf`] does a DXF; `px_per_inch` is for an SVG without a real size.
pub fn add_drawing_file(
    job: &Job,
    dxf: &Path,
    units: Option<Units>,
    px_per_inch: Option<f64>,
    name: Option<&str>,
) -> Result<(Job, UpdateReport), PipelineError> {
    let mut imported = shapes_from_file(dxf, units, px_per_inch, 1.0)?;
    let id = job.drawing.dxfs.iter().map(|d| d.id).max().unwrap_or(0) + 1;
    imported.source.id = id;
    let wanted = name
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map_or_else(|| imported.source.name.clone(), str::to_string);
    imported.source.name = unique_name(job, &wanted);
    let mut out = job.clone();
    let mut report = UpdateReport::default();
    for mut shape in imported.shapes {
        shape.id = out.drawing.next_shape_id;
        out.drawing.next_shape_id += 1;
        set_dxf(&mut shape, id);
        report.added.push(shape.id);
        out.drawing.shapes.push(shape);
    }
    report.messages.push(
        Message::new("pipeline-dxf-added")
            .arg("file", dxf.display().to_string())
            .arg("kind", imported.source.format.name())
            .arg("id", id)
            .arg("name", imported.source.name.as_str())
            .arg("count", report.added.len()),
    );
    out.drawing.dxfs.push(imported.source);
    report.messages.extend(pin_layers(&mut out, id));
    report.messages.extend(stock_fit_message(&out));
    report
        .messages
        .extend(layer_clash_notes(&out).into_iter().map(|n| n.message));
    report.lines = english(&report.messages);
    Ok((out, report))
}

/// Operations that select a layer of one DXF, by its name alone, keep cutting that DXF
/// only when DXF `added` brings the same layer: their selector gets that DXF. Adding a part
/// never changes what an existing operation cuts. Returns the report's lines.
fn pin_layers(job: &mut Job, added: u64) -> Vec<Message> {
    let mut pinned: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let shapes = job.drawing.shapes.clone();
    for (i, op) in job.operations.iter_mut().enumerate() {
        let Some(select) = op.select_mut() else {
            continue;
        };
        let Some(layer) = select.layer.clone() else {
            continue;
        };
        if select.dxf.is_some() || select.image.is_some() || select.text.is_some() {
            continue;
        }
        let on = |s: &&Shape| s.layer.to_lowercase() == layer.to_lowercase();
        if !shapes.iter().filter(on).any(|s| s.dxf() == Some(added)) {
            continue;
        }
        // Where the layer was before: one DXF, and nothing else.
        let before: BTreeSet<Option<u64>> = shapes
            .iter()
            .filter(on)
            .filter(|s| s.dxf() != Some(added))
            .map(|s| if s.image().is_some() { None } else { s.dxf() })
            .collect();
        if let [Some(dxf)] = before.into_iter().collect::<Vec<_>>()[..] {
            select.dxf = Some(dxf);
            let name = job
                .drawing
                .dxfs
                .iter()
                .find(|d| d.id == dxf)
                .map_or_else(|| format!("DXF {dxf}"), |d| d.name.clone());
            pinned.entry(name).or_default().push(i + 1);
        }
    }
    pinned
        .into_iter()
        .map(|(name, ops)| {
            let list = ops
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            Message::new("pipeline-dxf-pinned")
                .arg("count", ops.len())
                .arg("operations", list)
                .arg("name", name)
        })
        .collect()
}

/// Remove DXF `id` and its shapes, and anything hidden of it. Operations that selected its
/// shapes stay; the report names them, and the project's problems show them.
pub fn remove_dxf(job: &Job, id: u64) -> Result<(Job, Vec<Message>), PipelineError> {
    let Some(source) = job.drawing.dxfs.iter().find(|d| d.id == id) else {
        return Err(placement::no_dxf(id));
    };
    let removed: BTreeSet<u64> = job
        .drawing
        .shapes
        .iter()
        .filter(|s| s.dxf() == Some(id))
        .map(|s| s.id)
        .collect();
    let mut out = job.clone();
    out.drawing.dxfs.retain(|d| d.id != id);
    out.drawing.shapes.retain(|s| s.dxf() != Some(id));
    let key = format!("dxf:{id}");
    out.drawing
        .hidden
        .retain(|h| h != &key && !h.starts_with(&format!("{key}/")));
    let mut lines = vec![
        Message::new("pipeline-dxf-removed")
            .arg("kind", source.format.name())
            .arg("id", id)
            .arg("name", source.name.as_str())
            .arg("count", removed.len()),
    ];
    for (i, op) in out.operations.iter().enumerate() {
        let select = op.select();
        let lost: Vec<u64> = select
            .shapes
            .iter()
            .flatten()
            .copied()
            .filter(|s| removed.contains(s))
            .collect();
        if !lost.is_empty() {
            lines.push(
                Message::new("pipeline-dxf-shapes-gone")
                    .arg("label", op.label_message(i))
                    .arg("shapes", ids_text(&lost)),
            );
        } else if select.dxf == Some(id) {
            lines.push(Message::new("pipeline-dxf-gone").arg("label", op.label_message(i)));
        }
    }
    Ok((out, lines))
}

/// A notice for each operation that selects a layer (with no DXF or image named) that shapes
/// from more than one source have: adding a part can then change what it cuts
/// (docs/specs/drawing-sources.md §2).
pub fn layer_clash_notes(job: &Job) -> Vec<Note> {
    let mut notes = Vec::new();
    for (i, op) in job.operations.iter().enumerate() {
        let select = op.select();
        let Some(layer) = &select.layer else {
            continue;
        };
        if select.dxf.is_some() || select.image.is_some() || select.text.is_some() || !op.enabled()
        {
            continue;
        }
        // In the order of their English names, as always listed.
        let mut sources: BTreeMap<String, Message> = BTreeMap::new();
        for shape in &job.drawing.shapes {
            if shape.layer.to_lowercase() != layer.to_lowercase() {
                continue;
            }
            let name = source_name(job, shape);
            sources.insert(name.english(), name);
        }
        if sources.len() > 1 {
            let mut names = sources.into_values();
            let Some(first) = names.next() else {
                continue;
            };
            let listed = names.fold(first, |list, name| {
                Message::new("pipeline-and")
                    .arg("list", list)
                    .arg("last", name)
            });
            notes.push(Note::at(
                i,
                Message::new("pipeline-layer-clash")
                    .arg("label", op.label_message(i))
                    .arg("layer", layer.as_str())
                    .arg("sources", listed),
            ));
        }
    }
    notes
}

/// Where a shape comes from, for messages: a DXF's or image's name, or "drawn shapes".
fn source_name(job: &Job, shape: &Shape) -> Message {
    let named = |name: &str| Message::new("pipeline-source-named").arg("name", name);
    if let Some(id) = shape.dxf() {
        return job.drawing.dxfs.iter().find(|d| d.id == id).map_or_else(
            || Message::new("pipeline-source-dxf").arg("id", id),
            |d| named(&d.name),
        );
    }
    if let Some(id) = shape.image() {
        return job.drawing.images.iter().find(|i| i.id == id).map_or_else(
            || Message::new("pipeline-source-image").arg("id", id),
            |i| named(&i.file),
        );
    }
    Message::new("pipeline-source-drawn")
}

/// `name`, or with " 2", " 3", … when another DXF has it.
fn unique_name(job: &Job, name: &str) -> String {
    let taken = |n: &str| job.drawing.dxfs.iter().any(|d| d.name == n);
    if !taken(name) {
        return name.to_string();
    }
    (2..)
        .map(|k| format!("{name} {k}"))
        .find(|n| !taken(n))
        .unwrap_or_else(|| name.to_string())
}

pub(crate) fn set_dxf(shape: &mut Shape, id: u64) {
    if let Geometry::Path { dxf, .. } = &mut shape.geometry {
        *dxf = Some(id);
    }
}

/// Replace DXF `id`'s shapes with those of `dxf`, keeping ids of shapes that match by
/// (DXF handle, n-th shape with that handle). Other DXFs' shapes, drawn and traced shapes
/// are kept. `units` overrides the units recorded at the last import (needed only if they
/// differ).
pub fn update_dxf(
    job: &Job,
    id: u64,
    dxf: &Path,
    units: Option<Units>,
) -> Result<(Job, UpdateReport), PipelineError> {
    update_drawing_file(job, id, dxf, units, None)
}

/// [`update_dxf`] for a drawing file, DXF or SVG by the new file's extension; an SVG without
/// a real size is read at `px_per_inch`, else at the pixels per inch chosen before.
pub fn update_drawing_file(
    job: &Job,
    id: u64,
    dxf: &Path,
    units: Option<Units>,
    px_per_inch: Option<f64>,
) -> Result<(Job, UpdateReport), PipelineError> {
    let Some(previous) = job.drawing.dxfs.iter().find(|d| d.id == id) else {
        return Err(placement::no_dxf(id));
    };
    let expected = units.or(Some(previous.units));
    let px_per_inch = px_per_inch.or(previous.px_per_inch);
    // The new drawing goes where the old one was put (docs/specs/drawing-placement.md), read
    // at its scale so curves are approximated at that size (docs/specs/drawing-scale.md).
    let placed = previous.placement;
    let mut imported = shapes_from_file(dxf, expected, px_per_inch, placed.scale)?;
    placement::place_scaled_import(&mut imported.shapes, placed);
    imported.source.placement = placed;
    imported.source.id = id;
    imported.source.name.clone_from(&previous.name);
    for shape in &mut imported.shapes {
        set_dxf(shape, id);
    }
    let old = &job.drawing;
    // This DXF's old shapes by handle, in order; each can be matched once.
    let mut by_handle: BTreeMap<&str, Vec<&Shape>> = BTreeMap::new();
    for shape in old.shapes.iter().filter(|s| s.dxf() == Some(id)) {
        if let Some(handle) = shape.dxf_handle() {
            by_handle.entry(handle).or_default().push(shape);
        }
    }
    // How many shapes with each handle the new drawing has used so far.
    let mut taken: BTreeMap<String, usize> = BTreeMap::new();
    let mut next = old.next_shape_id;
    let mut report = UpdateReport::default();
    let mut shapes: Vec<Shape> = Vec::new();
    for mut shape in imported.shapes {
        let handle = shape.dxf_handle().unwrap_or_default().to_string();
        let k = taken.entry(handle.clone()).or_default();
        let matched = by_handle
            .get(handle.as_str())
            .and_then(|list| list.get(*k))
            .copied();
        *k += 1;
        match matched {
            Some(previous) => {
                shape.id = previous.id;
                let same = placement::same_shape(previous, &shape);
                if same {
                    report.unchanged.push(shape.id);
                } else {
                    report.changed.push(shape.id);
                }
            }
            None => {
                shape.id = next;
                next += 1;
                report.added.push(shape.id);
            }
        }
        shapes.push(shape);
    }
    let kept: BTreeSet<u64> = shapes.iter().map(|s| s.id).collect();
    report.removed = old
        .shapes
        .iter()
        .filter(|s| s.dxf() == Some(id) && !kept.contains(&s.id))
        .map(|s| s.id)
        .collect();
    // Every other shape stays as it is, and where it was: the new shapes take the old ones'
    // place in the list (so the order, and what is cut first, doesn't change).
    let first = old
        .shapes
        .iter()
        .position(|s| s.dxf() == Some(id))
        .unwrap_or(old.shapes.len());
    let others = |range: &[Shape]| -> Vec<Shape> {
        range
            .iter()
            .filter(|s| s.dxf() != Some(id))
            .cloned()
            .collect()
    };
    let mut all = others(&old.shapes[..first]);
    all.extend(shapes);
    all.extend(others(&old.shapes[first..]));

    let mut out = job.clone();
    out.drawing.shapes = all;
    out.drawing.next_shape_id = next;
    if let Some(entry) = out.drawing.dxfs.iter_mut().find(|d| d.id == id) {
        *entry = imported.source;
    }
    report.messages.push(
        Message::new("pipeline-dxf-updated")
            .arg("file", dxf.display().to_string())
            .arg("unchanged", report.unchanged.len())
            .arg("changed", report.changed.len())
            .arg("added", report.added.len())
            .arg("removed", report.removed.len()),
    );
    if placed != Placement::default() {
        report.messages.push(
            Message::new("pipeline-dxf-placed-as-before")
                .arg("placement", placement::placement_message(placed)),
        );
    }
    report.messages.extend(stock_fit_message(&out));
    report
        .messages
        .extend(layer_clash_notes(&out).into_iter().map(|n| n.message));
    for (i, op) in out.operations.iter().enumerate() {
        let label = op.label_message(i);
        let select = op.select();
        if let Some(ids) = &select.shapes {
            let lost: Vec<u64> = ids
                .iter()
                .copied()
                .filter(|id| report.removed.contains(id))
                .collect();
            let changed: Vec<u64> = ids
                .iter()
                .copied()
                .filter(|id| report.changed.contains(id))
                .collect();
            if !lost.is_empty() {
                report.lost.push(i);
                report.messages.push(
                    Message::new("pipeline-dxf-shapes-gone")
                        .arg("label", label)
                        .arg("shapes", ids_text(&lost)),
                );
            } else if !changed.is_empty() {
                report.messages.push(
                    Message::new("pipeline-dxf-geometry-changed")
                        .arg("label", label)
                        .arg("shapes", ids_text(&changed)),
                );
            }
        } else if let Some(layer) = &select.layer
            && select.dxf.is_none_or(|d| d == id)
            && select.image.is_none()
            && select.text.is_none()
        {
            report.messages.push(
                Message::new("pipeline-dxf-layer-updated")
                    .arg("label", label)
                    .arg("layer", layer.as_str()),
            );
        }
    }
    report.lines = english(&report.messages);
    Ok((out, report))
}

/// Messages in English, for the command line.
fn english(messages: &[Message]) -> Vec<String> {
    messages.iter().map(Message::english).collect()
}
