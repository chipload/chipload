// SPDX-License-Identifier: GPL-3.0-or-later
//! An operation's tool, part, corners and tabs changed.

use super::*;
use chipload_job::Operation;
use clap::Args;

#[derive(Args)]
pub(crate) struct ToolArgs {
    /// The project (.chipload).
    project: PathBuf,
    /// The operation's number as the app lists it (1 = first; `get` and `set` paths
    /// count from 0).
    #[arg(long)]
    op: usize,
    /// The tool's id in the project.
    #[arg(long)]
    tool: i64,
    /// Where to write the result; default: the project itself.
    #[arg(short, long)]
    output: Option<PathBuf>,
}

impl ToolArgs {
    pub(crate) fn run(self, out: &mut Out) -> ExitCode {
        let ToolArgs {
            project,
            op,
            tool,
            output,
        } = self;
        let mut job = match load(out, &project) {
            Ok(job) => job,
            Err(code) => return code,
        };
        if let Err(err) = set_op_tool(&mut job, op, tool) {
            return out.fail(err);
        }
        // An operation with a material takes its values for the new tool
        // (docs/specs/materials.md §2).
        let followed = materials::follow_op_material(&mut job, op - 1);
        let target = output.as_deref().unwrap_or(&project);
        if let Err(err) = crate::save_project(out, &job, target) {
            return out.fail(err);
        }
        out.line(format!("Operation {op} now cuts with tool {tool}."));
        if let Some(name) = followed {
            out.line(format!("Its feeds are {name}'s values for tool {tool}."));
        }
        out.wrote(target, "");
        ExitCode::SUCCESS
    }
}

#[derive(Args)]
pub(crate) struct PartArgs {
    /// The project (.chipload).
    project: PathBuf,
    /// The operation's number as the app lists it (1 = first; `get` and `set` paths
    /// count from 0).
    #[arg(long)]
    op: usize,
    /// The part's id (as `part list` shows it).
    #[arg(long)]
    part: u64,
    /// Where to write the result; default: the project itself.
    #[arg(short, long)]
    output: Option<PathBuf>,
}

impl PartArgs {
    pub(crate) fn run(self, out: &mut Out) -> ExitCode {
        let PartArgs {
            project,
            op,
            part,
            output,
        } = self;
        let job = match load(out, &project) {
            Ok(job) => job,
            Err(code) => return code,
        };
        let Some(index) = op.checked_sub(1) else {
            return out.fail("operations are numbered from 1");
        };
        let (job, at) = match chipload_pipeline::parts::move_operation_to_part(&job, index, part) {
            Ok(done) => done,
            Err(err) => return out.fail(err),
        };
        let target = output.as_deref().unwrap_or(&project);
        if let Err(err) = crate::save_project(out, &job, target) {
            return out.fail(err);
        }
        out.line(format!(
            "Operation {op} is now operation {} in part {part}.",
            at + 1
        ));
        out.set("operation", json!(at));
        out.wrote(target, "");
        ExitCode::SUCCESS
    }
}

#[derive(Args)]
pub(crate) struct CornersArgs {
    /// The project (.chipload).
    project: PathBuf,
    /// The operation's number as the app lists it (1 = first).
    #[arg(long)]
    op: usize,
}

impl CornersArgs {
    pub(crate) fn run(self, out: &mut Out) -> ExitCode {
        let CornersArgs { project, op } = self;
        let job = match load(out, &project) {
            Ok(job) => job,
            Err(code) => return code,
        };
        let Some(index) = op.checked_sub(1) else {
            return out.fail("operations are numbered from 1");
        };
        let rows = match chipload_pipeline::corners::corners_of(&job, index) {
            Ok(rows) => rows,
            Err(err) => return out.fail(err),
        };
        if rows.is_empty() {
            out.line(format!("Operation {op} has no corners on the tool's side."));
        }
        let mut list = Vec::new();
        for row in &rows {
            let gets = gets_name(row.gets);
            let own = if row.chosen { " (its own choice)" } else { "" };
            out.line(format!(
                "({}, {})  {:.1}°  {gets}{own}",
                chipload_core::text::mm(row.at[0]),
                chipload_core::text::mm(row.at[1]),
                row.angle.to_degrees()
            ));
            list.push(json!({
                "at": row.at,
                "angle_deg": row.angle.to_degrees(),
                "gets": gets,
                "chosen": row.chosen,
            }));
        }
        out.set("corners", Value::Array(list));
        ExitCode::SUCCESS
    }
}

#[derive(Args)]
pub(crate) struct CornerArgs {
    /// The project (.chipload).
    project: PathBuf,
    /// The operation's number as the app lists it (1 = first).
    #[arg(long)]
    op: usize,
    /// The corner, X,Y in drawing coordinates (a plain number is mm).
    #[arg(long, value_parser = crate::lengths::point, allow_hyphen_values = true,
          required_unless_present = "reset")]
    at: Option<[f64; 2]>,
    /// What the corner gets.
    #[arg(long, value_enum, conflicts_with_all = ["as_operation", "reset"],
          required_unless_present_any = ["as_operation", "reset"])]
    bone: Option<BoneArg>,
    /// With a T-bone: the wall it runs along (the notch is in the other); default longer.
    #[arg(long, value_enum)]
    along: Option<WallArg>,
    /// Remove the corner's own choice: it gets what the operation gives every corner.
    #[arg(long, conflicts_with = "reset")]
    as_operation: bool,
    /// Remove every corner's own choice.
    #[arg(long)]
    reset: bool,
    /// Where to write the result; default: the project itself.
    #[arg(short, long)]
    output: Option<PathBuf>,
}

impl CornerArgs {
    pub(crate) fn run(self, out: &mut Out) -> ExitCode {
        let CornerArgs {
            project,
            op,
            at,
            bone,
            along,
            as_operation: _,
            reset,
            output,
        } = self;
        let mut job = match load(out, &project) {
            Ok(job) => job,
            Err(code) => return code,
        };
        let Some(index) = op.checked_sub(1) else {
            return out.fail("operations are numbered from 1");
        };
        let done = if reset {
            chipload_pipeline::corners::reset_corners(&mut job, index)
                .map(|count| format!("Removed the choices of {count} corners of operation {op}."))
        } else {
            let at = at.unwrap_or_default();
            let choice = bone.map(|bone| {
                let bone = match bone {
                    BoneArg::None => chipload_job::Bone::None,
                    BoneArg::Dogbone => chipload_job::Bone::Dogbone,
                    BoneArg::Tbone => chipload_job::Bone::Tbone,
                };
                let along = along.map(|wall| match wall {
                    WallArg::Longer => chipload_job::Wall::Longer,
                    WallArg::Shorter => chipload_job::Wall::Shorter,
                });
                (bone, along)
            });
            if along.is_some() && !matches!(bone, Some(BoneArg::Tbone)) {
                return out.fail("--along goes with --bone tbone");
            }
            chipload_pipeline::corners::choose_corner(&mut job, index, at, choice).map(|corner| {
                let (x, y) = (
                    chipload_core::text::mm(corner[0]),
                    chipload_core::text::mm(corner[1]),
                );
                match choice {
                    None => format!(
                        "The corner at ({x}, {y}) of operation {op} gets what the \
                             operation gives every corner."
                    ),
                    Some(_) => {
                        let gets = match bone {
                            Some(BoneArg::None) => "round",
                            Some(BoneArg::Dogbone) => "a dogbone",
                            _ => match along {
                                Some(WallArg::Shorter) => "a T-bone along the shorter wall",
                                _ => "a T-bone along the longer wall",
                            },
                        };
                        format!("The corner at ({x}, {y}) of operation {op} is now {gets}.")
                    }
                }
            })
        };
        let line = match done {
            Ok(line) => line,
            Err(err) => return out.fail(err),
        };
        let target = output.as_deref().unwrap_or(&project);
        if let Err(err) = crate::save_project(out, &job, target) {
            return out.fail(err);
        }
        out.line(line);
        out.wrote(target, "");
        ExitCode::SUCCESS
    }
}

#[derive(Args)]
pub(crate) struct TabsArgs {
    /// The project (.chipload).
    project: PathBuf,
    /// The operation's number as the app lists it (1 = first).
    #[arg(long)]
    op: usize,
    /// On the edges along this axis: x (bottom and top) or y (left and right), half on
    /// each.
    #[arg(long, conflicts_with = "around")]
    along: Option<AxisArg>,
    /// Evenly round each contour (as without --along).
    #[arg(long)]
    around: bool,
    /// Where to write the result; default: the project itself.
    #[arg(short, long)]
    output: Option<PathBuf>,
}

impl TabsArgs {
    pub(crate) fn run(self, out: &mut Out) -> ExitCode {
        let TabsArgs {
            project,
            op,
            along,
            around,
            output,
        } = self;
        let mut job = match load(out, &project) {
            Ok(job) => job,
            Err(code) => return code,
        };
        let Some(index) = op.checked_sub(1) else {
            return out.fail("operations are numbered from 1");
        };
        if along.is_some() || around {
            let Some(Operation::Profile(chipload_job::ProfileOp {
                tabs: Some(tabs), ..
            })) = job.operations.get_mut(index)
            else {
                return out.fail(format!("operation {op} is not a profile with tabs"));
            };
            tabs.along = along.map(|a| match a {
                AxisArg::X => chipload_job::Axis::X,
                AxisArg::Y => chipload_job::Axis::Y,
            });
            let target = output.as_deref().unwrap_or(&project);
            if let Err(err) = crate::save_project(out, &job, target) {
                return out.fail(err);
            }
            out.line(match along {
                Some(AxisArg::X) => "Tabs on the edges along X (bottom and top).",
                Some(AxisArg::Y) => "Tabs on the edges along Y (left and right).",
                None => "Tabs evenly round each contour.",
            });
            out.wrote(target, "");
        }
        let rows = match chipload_pipeline::tabs::tabs_of(&job, index) {
            Ok(rows) => rows,
            Err(err) => return out.fail(err),
        };
        let mut list = Vec::new();
        for row in &rows {
            let how = if row.by_hand {
                "placed by hand"
            } else {
                "spaced"
            };
            out.line(format!(
                "ring {}  ({}, {})  {how}",
                row.ring + 1,
                chipload_core::text::mm(row.at.0),
                chipload_core::text::mm(row.at.1)
            ));
            list.push(json!({
                "ring": row.ring + 1,
                "at": [row.at.0, row.at.1],
                "by_hand": row.by_hand,
            }));
        }
        out.set("tabs", Value::Array(list));
        ExitCode::SUCCESS
    }
}

#[derive(Args)]
pub(crate) struct TabArgs {
    /// The project (.chipload).
    project: PathBuf,
    /// The operation's number as the app lists it (1 = first).
    #[arg(long)]
    op: usize,
    /// The point, X,Y in drawing coordinates (a plain number is mm): on or near the
    /// contour, within a tool diameter.
    #[arg(long, value_parser = crate::lengths::point, allow_hyphen_values = true,
          required_unless_present = "reset")]
    at: Option<[f64; 2]>,
    /// Remove the tab nearest to --at.
    #[arg(long, conflicts_with_all = ["even", "reset"])]
    remove: bool,
    /// Place the tabs of the contour nearest to --at again, as the operation places them
    /// (evenly, or along an axis: `op tabs --along`).
    #[arg(long, conflicts_with = "reset")]
    even: bool,
    /// Place every contour's tabs again, as the operation places them.
    #[arg(long)]
    reset: bool,
    /// Where to write the result; default: the project itself.
    #[arg(short, long)]
    output: Option<PathBuf>,
}

impl TabArgs {
    pub(crate) fn run(self, out: &mut Out) -> ExitCode {
        let TabArgs {
            project,
            op,
            at,
            remove,
            even,
            reset,
            output,
        } = self;
        let mut job = match load(out, &project) {
            Ok(job) => job,
            Err(code) => return code,
        };
        let Some(index) = op.checked_sub(1) else {
            return out.fail("operations are numbered from 1");
        };
        let at = at.unwrap_or_default();
        let (x, y) = (
            chipload_core::text::mm(at[0]),
            chipload_core::text::mm(at[1]),
        );
        use chipload_pipeline::tabs::{add_tab, even_ring, remove_tab, reset_tabs};
        let done = if reset {
            reset_tabs(&mut job, index).map(|count| {
                format!(
                    "Operation {op}'s tabs are spaced evenly again ({count} were placed by hand)."
                )
            })
        } else if even {
            even_ring(&mut job, index, at).map(|count| {
                format!(
                    "The tabs of the contour at ({x}, {y}) are spaced evenly again ({count} were placed by hand)."
                )
            })
        } else if remove {
            remove_tab(&mut job, index, at)
                .map(|()| format!("Removed operation {op}'s tab nearest to ({x}, {y})."))
        } else {
            add_tab(&mut job, index, at)
                .map(|()| format!("Added a tab to operation {op} at ({x}, {y})."))
        };
        let line = match done {
            Ok(line) => line,
            Err(err) => return out.fail(err),
        };
        let target = output.as_deref().unwrap_or(&project);
        if let Err(err) = crate::save_project(out, &job, target) {
            return out.fail(err);
        }
        out.line(line);
        out.wrote(target, "");
        ExitCode::SUCCESS
    }
}
