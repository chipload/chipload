// SPDX-License-Identifier: GPL-3.0-or-later
//! Using a style on a selection (docs/specs/styles.md §3.2): the operations a new one of its
//! kind gets (`new_ops`), with the style's settings over them, its tools taken from the
//! library.

use std::collections::BTreeMap;

use chipload_core::chain::Contour;
use chipload_i18n::{Language, Message, Units};
use chipload_job::library::{ToolLibrary, use_tool};
use chipload_job::styles::{Style, StyleSide, StyleTool, side_of};
use chipload_job::{Job, Operation, Problem, Selector, Side, Tool};

use crate::PipelineError;
use crate::new_ops::{NewOp, NewOps, NewRequest, OpKind, by_shapes, new_operations, tool_cutting};

/// What using a style makes: the library tools to copy into the project first, and the
/// operations, to insert as `new_ops::insert` does.
#[derive(Debug, Clone, PartialEq)]
pub struct StyleOps {
    pub tools: Vec<Tool>,
    pub made: NewOps,
}

/// The kind of operation `op` is.
pub fn kind_of(op: &Operation) -> OpKind {
    match op {
        Operation::Profile(_) => OpKind::Profile,
        Operation::Pocket(_) => OpKind::Pocket,
        Operation::Vcarve(_) => OpKind::Vcarve,
        Operation::Path(_) => OpKind::Path,
        Operation::Holes(_) => OpKind::Holes,
        Operation::Surface(_) => OpKind::Surface,
    }
}

/// The operations `style` makes for `select` in `job`, named in `lang`. Refused when its
/// operation can't be read or one of its tools isn't in `library`.
pub fn style_operations(
    job: &Job,
    style: &Style,
    select: &Selector,
    contours: &[Contour],
    library: &ToolLibrary,
    lang: Language,
) -> Result<StyleOps, PipelineError> {
    let mut settings = style
        .operation()
        .map_err(|e| PipelineError::Job(e.message()))?;
    // Its tools: the project's copy of each library tool, else the library tool copied in.
    let mut with_tools = job.clone();
    let mut ids = BTreeMap::new();
    for tool in &style.tools {
        let linked = with_tools
            .tools
            .iter()
            .find(|t| t.library_uid.as_deref() == Some(tool.uid.as_str()))
            .map(|t| t.id);
        let id = match linked {
            Some(id) => id,
            None => {
                let entry = library.find_uid(&tool.uid).ok_or_else(|| {
                    PipelineError::Job(
                        Message::new("pipeline-style-tool-missing")
                            .arg("style", style.name.as_str())
                            .arg("tool", tool.name.as_str()),
                    )
                })?;
                use_tool(&mut with_tools, entry).id
            }
        };
        ids.insert(tool.id, id);
    }
    settings.remap_tools(&ids);
    let request = NewRequest {
        kind: kind_of(&settings),
        select: select.clone(),
        tool: Some(settings.tool()),
    };
    let fresh = new_operations(&with_tools, &request, contours, Some(library), lang);
    let names = names(job, &style.name, fresh.ops.len(), lang);
    // The note about two profiles names them as the style does.
    let messages = match names.as_slice() {
        [holes, outline] if !fresh.messages.is_empty() => vec![by_shapes(holes, outline)],
        _ => fresh.messages.clone(),
    };
    let cutting = tool_cutting(&with_tools, Some(library), settings.tool());
    let ops = fresh
        .ops
        .into_iter()
        .zip(names)
        .map(|(new, name)| {
            let mut op = settings.clone();
            dress(&mut op, &new.op, name, style.side);
            if style.side == Some(StyleSide::BySelection) {
                drop_inner_tabs(&mut op);
            }
            if let Some(values) = &cutting {
                op.apply_cutting(values);
            }
            op.fit_helixes(&with_tools.tools);
            let point = with_tools.drill_point(&op);
            op.follow_thickness(with_tools.stock().thickness, point);
            NewOp {
                op,
                side_chosen: new.side_chosen,
            }
        })
        .collect();
    let added = with_tools.tools[job.tools.len()..].to_vec();
    Ok(StyleOps {
        tools: added,
        made: NewOps {
            index: fresh.index,
            ops,
            messages,
        },
    })
}

/// The style's settings `op` take what the new operation `fresh` has of the selection: its
/// shapes, part and material, and a profile's side unless the style keeps its own.
fn dress(op: &mut Operation, fresh: &Operation, name: String, side: Option<StyleSide>) {
    *op.name_mut() = name;
    op.set_part(fresh.part());
    op.set_enabled(true);
    op.set_material(fresh.material().cloned());
    if let Some(select) = op.select_mut() {
        *select = fresh.select().clone();
    }
    if let (Operation::Profile(p), Operation::Profile(f)) = (op, fresh)
        && side != Some(StyleSide::AsSaved)
    {
        p.side = f.side;
    }
}

/// A profile cut inside, by the selection, gets no tabs: the holes' slugs aren't parts
/// (docs/specs/styles.md §2.1).
fn drop_inner_tabs(op: &mut Operation) {
    if let Operation::Profile(p) = op
        && p.side == Side::Inside
    {
        p.tabs = None;
    }
}

/// The names of `count` operations of a style called `base`: the name itself, numbered when
/// the project has it; two are "(holes)" and "(outline)".
fn names(job: &Job, base: &str, count: usize, lang: Language) -> Vec<String> {
    let taken = |n: &str| job.operations.iter().any(|op| op.name() == n);
    let free = (1..)
        .map(|k| {
            if k == 1 {
                base.to_string()
            } else {
                format!("{base} {k}")
            }
        })
        .find(|n| {
            !taken(n)
                && !taken(&suffixed("pipeline-name-holes-of", n, lang))
                && !taken(&suffixed("pipeline-name-outline-of", n, lang))
        })
        .unwrap_or_else(|| base.to_string());
    if count > 1 {
        vec![
            suffixed("pipeline-name-holes-of", &free, lang),
            suffixed("pipeline-name-outline-of", &free, lang),
        ]
    } else {
        vec![free]
    }
}

fn suffixed(id: &'static str, name: &str, lang: Language) -> String {
    Message::new(id).arg("name", name).render(lang, Units::Mm)
}

/// Copy the tools in and insert the operations; the first one's index.
pub fn insert(job: &mut Job, ops: StyleOps) -> usize {
    job.tools.extend(ops.tools);
    crate::new_ops::insert(job, ops.made)
}

/// A style laid out for the operation form (docs/specs/styles.md §9, step 3b): its
/// operation with its tools numbered by their place in the library (from 1), and the
/// library's tools as project tools by those numbers. A style tool the library no longer
/// has is numbered after them and has no tool in the list.
#[derive(Debug, Clone, PartialEq)]
pub struct StyleDraft {
    pub operation: Operation,
    pub tools: Vec<Tool>,
}

/// Style `style` laid out for editing with the tools of `library`.
pub fn style_draft(style: &Style, library: &ToolLibrary) -> Result<StyleDraft, PipelineError> {
    let mut operation = style
        .operation()
        .map_err(|e| PipelineError::Job(e.message()))?;
    let tools = library
        .tools
        .iter()
        .zip(1..)
        .map(|(entry, id)| entry.tool(id))
        .collect::<Vec<_>>();
    let missing = |k: usize| i64::try_from(library.tools.len() + 1 + k).unwrap_or(i64::MAX);
    let ids = style
        .tools
        .iter()
        .enumerate()
        .map(|(k, tool)| {
            let id = library
                .tools
                .iter()
                .position(|t| t.uid == tool.uid)
                .and_then(|i| i64::try_from(i + 1).ok())
                .unwrap_or_else(|| missing(k));
            (tool.id, id)
        })
        .collect::<BTreeMap<_, _>>();
    operation.remap_tools(&ids);
    Ok(StyleDraft { operation, tools })
}

/// `style` with the operation `op` of its draft (made by [`style_draft`] with the same
/// `library`): its tools named by uid again, its side by [`side_of`].
pub fn style_from_draft(
    style: &Style,
    op: &Operation,
    library: &ToolLibrary,
) -> Result<Style, PipelineError> {
    let count = library.tools.len();
    let mut tools = Vec::new();
    for id in op.tool_ids() {
        let place = usize::try_from(id).ok().and_then(|k| k.checked_sub(1));
        let tool = match place {
            Some(k) if k < count => library.tools.get(k).map(|entry| StyleTool {
                id,
                uid: entry.uid.clone(),
                name: entry.name.clone(),
            }),
            Some(k) => style
                .tools
                .get(k - count)
                .map(|kept| StyleTool { id, ..kept.clone() }),
            None => None,
        };
        let Some(tool) = tool else {
            return Err(PipelineError::Job(
                Message::new("pipeline-style-tool-unknown").arg("tool", id),
            ));
        };
        tools.push(tool);
    }
    let operation = serde_json::to_value(op).map_err(|e| {
        PipelineError::Job(
            Message::new("job-style-unreadable")
                .arg("name", style.name.as_str())
                .arg("reason", e.to_string()),
        )
    })?;
    Ok(Style {
        tools,
        side: side_of(op),
        operation,
        ..style.clone()
    })
}

/// The problems of a style's operation `op` with `tools` (a draft's), as the operation's
/// own form shows them: tried in an empty project, its shapes left out.
pub fn style_problems(op: &Operation, tools: &[Tool]) -> Vec<Problem> {
    let mut job = crate::empty_project();
    job.tools = tools.to_vec();
    let mut op = op.clone();
    if let Some(part) = job.parts.first() {
        op.set_part(part.id);
    }
    job.operations = vec![op];
    crate::job_problems(&job)
        .into_iter()
        .filter(|p| p.operation() == Some(0))
        .filter(|p| {
            let field = p.field();
            field != "select" && !field.starts_with("select.")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chipload_job::library::LibraryTool;
    use chipload_job::styles::style_of;
    use chipload_job::{Source, load_job};
    use std::path::PathBuf;

    fn corpus(file: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/corpus")
            .join(file)
    }

    /// The golden `style_use`: a pocket of the circle (shape 2), then the DXF's holes and
    /// outline profiled, all with tool 2 from the library.
    fn project() -> (Job, ToolLibrary, Vec<Contour>) {
        let job = load_job(&corpus("jobs/v35/style_use.chipload")).unwrap();
        let tool = job.tool(2).unwrap().clone();
        let mut library = ToolLibrary::default();
        let uid = tool.library_uid.clone().unwrap();
        library
            .tools
            .push(LibraryTool::from_tool("6 mm 2F", &uid, &tool, Vec::new()));
        let contours = crate::load_drawing(&job).unwrap().contours;
        (job, library, contours)
    }

    fn shapes(ids: &[u64]) -> Selector {
        Selector {
            shapes: Some(ids.to_vec()),
            ..Selector::default()
        }
    }

    #[test]
    fn a_style_used_again_makes_the_operation_it_was_saved_from() {
        let (job, library, contours) = project();
        let style = style_of(&job, 0, "Pocket 3 mm", &library).unwrap();
        let mut empty = job.clone();
        empty.operations.clear();
        empty.tools.retain(|t| t.id == 1);
        let made = style_operations(
            &empty,
            &style,
            &shapes(&[2]),
            &contours,
            &library,
            Language::En,
        )
        .unwrap();
        // The library tool copied in, under the next free number: 2, as in the project.
        assert_eq!(made.tools.len(), 1);
        assert_eq!(made.tools[0], *job.tool(2).unwrap());
        assert_eq!(made.made.ops.len(), 1);
        assert_eq!(made.made.ops[0].op, job.operations[0]);
        // Used where the tool is already in: no copy, and numbered when the name is taken.
        let again = style_operations(
            &job,
            &style,
            &shapes(&[2]),
            &contours,
            &library,
            Language::En,
        )
        .unwrap();
        assert!(again.tools.is_empty());
        assert_eq!(again.made.ops[0].op.name(), "Pocket 3 mm 2");
    }

    #[test]
    fn a_profile_style_follows_the_selection_and_holes_get_no_tabs() {
        let (job, library, contours) = project();
        let style = style_of(&job, 2, "Cut out", &library).unwrap();
        let made = style_operations(
            &job,
            &style,
            &Selector::of(Source::Dxf(1)),
            &contours,
            &library,
            Language::En,
        )
        .unwrap();
        let ops: Vec<&Operation> = made.made.ops.iter().map(|n| &n.op).collect();
        let sides: Vec<(Side, bool)> = ops
            .iter()
            .map(|op| match op {
                Operation::Profile(p) => (p.side, p.tabs.is_some()),
                _ => panic!("profiles"),
            })
            .collect();
        assert_eq!(sides, [(Side::Inside, false), (Side::Outside, true)]);
        assert_eq!(
            ops[0].name(),
            "Cut out 2 (holes)",
            "the project has Cut out (holes)"
        );
        assert!(
            made.made.messages[0]
                .english()
                .contains("Cut out 2 (outline)")
        );
    }

    #[test]
    fn a_style_whose_tool_is_gone_is_refused_by_name() {
        let (job, library, contours) = project();
        let style = style_of(&job, 0, "Pocket 3 mm", &library).unwrap();
        let mut elsewhere = job.clone();
        elsewhere.tools.retain(|t| t.id == 1);
        elsewhere.operations.clear();
        let err = style_operations(
            &elsewhere,
            &style,
            &shapes(&[2]),
            &contours,
            &ToolLibrary::default(),
            Language::En,
        )
        .unwrap_err();
        assert!(err.to_string().contains("6 mm 2F"), "{err}");
    }

    /// A style laid out for the form and put back: its tool by the library's place, its
    /// values changed, its tools named by uid again; a profile put on the line keeps it.
    #[test]
    fn a_style_draft_edited_goes_back_with_its_tools_by_uid() {
        let (job, mut library, _) = project();
        // A second library tool before the style's, so the numbers change.
        let mut other = library.tools[0].clone();
        other.uid = "00000000000000aa".into();
        other.name = "3 mm 1F".into();
        other.diameter = 3.0;
        library.tools.insert(0, other);
        let style = style_of(&job, 2, "Cut out", &library).unwrap();
        assert_eq!(style.side, Some(StyleSide::BySelection));
        let draft = style_draft(&style, &library).unwrap();
        assert_eq!(draft.tools.len(), 2);
        assert_eq!(draft.operation.tool(), 2, "the second in the library");
        let mut op = draft.operation.clone();
        op.set_end_depth(6.5);
        let Operation::Profile(p) = &mut op else {
            panic!("a profile");
        };
        p.side = Side::On;
        op.set_tool(1);
        let saved = style_from_draft(&style, &op, &library).unwrap();
        assert_eq!(saved.name, style.name);
        assert_eq!(saved.uid, style.uid);
        assert_eq!(saved.tools.len(), 1);
        assert_eq!(saved.tools[0].uid, "00000000000000aa");
        assert_eq!(saved.side, Some(StyleSide::AsSaved));
        assert_eq!(saved.operation().unwrap().depths().1, 6.5);
        // Unchanged, it is the style it was.
        let same = style_from_draft(&style, &draft.operation, &library).unwrap();
        assert_eq!(same.operation().unwrap(), style.operation().unwrap());
        assert_eq!(same.tools[0].uid, style.tools[0].uid);
    }

    /// A style tool the library no longer has keeps its place after the library's, and is
    /// named as before when written back.
    #[test]
    fn a_missing_tool_stays_in_the_draft() {
        let (job, library, _) = project();
        let style = style_of(&job, 0, "Pocket 3 mm", &library).unwrap();
        let empty = ToolLibrary::default();
        let draft = style_draft(&style, &empty).unwrap();
        assert!(draft.tools.is_empty());
        assert_eq!(draft.operation.tool(), 1);
        let back = style_from_draft(&style, &draft.operation, &empty).unwrap();
        assert_eq!(back.tools[0].uid, style.tools[0].uid);
        assert_eq!(back.tools[0].name, style.tools[0].name);
    }

    /// A style's problems are its operation's, by field; having no shapes isn't one.
    #[test]
    fn a_style_has_its_operations_problems_but_not_for_shapes() {
        let (job, library, _) = project();
        let style = style_of(&job, 0, "Pocket 3 mm", &library).unwrap();
        let draft = style_draft(&style, &library).unwrap();
        assert_eq!(style_problems(&draft.operation, &draft.tools), Vec::new());
        let mut op = draft.operation.clone();
        let Operation::Pocket(p) = &mut op else {
            panic!("a pocket");
        };
        p.step_down = 0.0;
        let problems = style_problems(&op, &draft.tools);
        assert!(
            problems.iter().any(|p| p.field() == "step_down"),
            "{problems:?}"
        );
    }
}
