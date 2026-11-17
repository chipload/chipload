// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload part instance …`: a part placed again, moved, turned and perhaps mirrored
//! (docs/specs/instances.md §5). `array` lays out a grid of them, as `copies` does.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chipload_job::{Instance, Job};
use chipload_pipeline::instances::{
    InstanceChange, add_instance, move_instance, remove_instances, set_instances,
};
use clap::Subcommand;
use serde_json::{Value, json};

use crate::out::Out;
use crate::parts::MirrorChoice;
use crate::placing::{CopiesArgs, OrderArg, copies, warn_overlap};
use crate::{load, point_arg, write_project};

#[derive(Subcommand)]
pub enum InstanceAction {
    /// A part's instances: number, position, turn, mirror, on or off.
    List {
        /// The project (.chipload).
        project: PathBuf,
        /// Which part (as `part list` shows it); needed when the project has several.
        #[arg(long)]
        part: Option<u64>,
    },
    /// Place the part again: a new instance, linked to the part. Prints its number.
    Add {
        /// The project (.chipload).
        project: PathBuf,
        /// Which part; needed when the project has several.
        #[arg(long)]
        part: Option<u64>,
        /// The setup (board) it goes on, as `setup list` shows it; default: the one the
        /// part is cut from.
        #[arg(long)]
        setup: Option<u64>,
        /// Its move "DX,DY" from the part as drawn (any length unit; plain numbers are mm);
        /// default: beside everything the part cuts, 10 mm away.
        #[arg(long, allow_hyphen_values = true)]
        at: Option<String>,
        /// Turned about the middle of the part's box, degrees counter-clockwise.
        #[arg(long, allow_hyphen_values = true)]
        angle: Option<f64>,
        /// Mirrored about the middle of the part's box first: left-right, top-bottom or
        /// none. Climb stays climb.
        #[arg(long, value_parser = crate::parts::mirror)]
        mirror: Option<MirrorChoice>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Move, turn, mirror, or turn off or on, instances of a part.
    Set {
        /// The project (.chipload).
        project: PathBuf,
        /// Which part; needed when the project has several.
        #[arg(long)]
        part: Option<u64>,
        /// The instances' numbers (as `part instance list` shows them).
        #[arg(required = true)]
        ids: Vec<u32>,
        /// Its move "DX,DY" from the part as drawn; one instance only.
        #[arg(long, allow_hyphen_values = true)]
        at: Option<String>,
        /// Turned about the middle of the part's box, degrees counter-clockwise.
        #[arg(long, allow_hyphen_values = true)]
        angle: Option<f64>,
        /// Mirrored: left-right, top-bottom or none.
        #[arg(long, value_parser = crate::parts::mirror)]
        mirror: Option<MirrorChoice>,
        /// Keep it, but don't cut it.
        #[arg(long, conflicts_with = "on")]
        off: bool,
        /// Cut it again.
        #[arg(long)]
        on: bool,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Remove instances of a part; the others keep their numbers.
    Remove {
        /// The project (.chipload).
        project: PathBuf,
        /// Which part; needed when the project has several.
        #[arg(long)]
        part: Option<u64>,
        /// The instances' numbers.
        #[arg(required_unless_present = "all")]
        ids: Vec<u32>,
        /// Every instance but the first: the part is cut once.
        #[arg(long, conflicts_with = "ids")]
        all: bool,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// An instance onto another setup (board), keeping its number and placement
    /// (docs/specs/parts-and-setups.md §10).
    Move {
        /// The project (.chipload).
        project: PathBuf,
        /// Which part; needed when the project has several.
        #[arg(long)]
        part: Option<u64>,
        /// The instance's number.
        id: u32,
        /// The setup it goes on, as `setup list` shows it.
        #[arg(long)]
        setup: u64,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// The part's instances made a grid: its cells but the first, in place of the
    /// instances it has, numbered row by row from 2 (what `chipload copies` does).
    Array {
        /// The project (.chipload).
        project: PathBuf,
        /// Which part; needed when the project has several.
        #[arg(long)]
        part: Option<u64>,
        /// The setup (board) it goes on, as `setup list` shows it; default: the one the
        /// part is cut from.
        #[arg(long)]
        setup: Option<u64>,
        /// Columns x rows, e.g. "3x2".
        #[arg(long)]
        grid: String,
        /// Pitch "DX,DY" from cell to cell; negative grows left or down.
        #[arg(long, allow_hyphen_values = true, conflicts_with = "gap")]
        pitch: Option<String>,
        /// Set the pitch so that this many mm are left between the cuts of neighbours.
        #[arg(value_parser = crate::lengths::length, long)]
        gap: Option<f64>,
        /// The cut order.
        #[arg(long)]
        order: Option<OrderArg>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

/// The part a command means: `--part`, or the only one.
pub(crate) fn part_of(job: &Job, part: Option<u64>) -> Result<u64, String> {
    let id = match (part, job.parts.as_slice()) {
        (Some(id), _) => id,
        (None, [only]) => only.id,
        (None, _) => {
            return Err("the project has several parts: give --part ID (see `part list`)".into());
        }
    };
    if job.parts.iter().any(|p| p.id == id) {
        Ok(id)
    } else {
        Err(format!("no part {id} in the project (see `part list`)"))
    }
}

/// Degrees from the command line, as radians in −π…π.
fn radians(degrees: f64) -> Result<f64, String> {
    if !degrees.is_finite() {
        return Err(format!("--angle takes degrees, got {degrees}"));
    }
    let a = (degrees + 180.0).rem_euclid(360.0) - 180.0;
    Ok(a.to_radians())
}

fn at_arg(at: Option<&str>) -> Result<Option<[f64; 2]>, String> {
    at.map(|text| point_arg(text).map_err(|e| format!("--at: {e}")))
        .transpose()
}

pub fn instance(out: &mut Out, action: InstanceAction) -> ExitCode {
    match action {
        InstanceAction::List { project, part } => list(out, &project, part),
        InstanceAction::Array {
            project,
            part,
            setup,
            grid,
            pitch,
            gap,
            order,
            output,
        } => copies(
            out,
            &project,
            CopiesArgs {
                part,
                setup,
                grid: Some(grid),
                pitch,
                gap,
                order,
                off: false,
            },
            output.as_deref(),
        ),
        InstanceAction::Move {
            project,
            part,
            id,
            setup,
            output,
        } => {
            let job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            let moved = part_of(&job, part).and_then(|part| {
                move_instance(&job, part, id, setup)
                    .map(|job| (job, part))
                    .map_err(|e| e.to_string())
            });
            match moved {
                Ok((job, part)) => {
                    let name = job.setup(setup).map_or("", |s| s.name.as_str());
                    out.line(format!("Instance {id} is cut on setup {setup} \"{name}\""));
                    out.set("setup", json!(setup));
                    finish(out, &job, part, output.as_deref().unwrap_or(&project))
                }
                Err(err) => out.fail(err),
            }
        }
        InstanceAction::Add {
            project,
            part,
            setup,
            at,
            angle,
            mirror,
            output,
        } => {
            let job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            let made = (|| {
                let id = part_of(&job, part)?;
                let at = at_arg(at.as_deref())?;
                let angle = radians(angle.unwrap_or(0.0))?;
                let mirror = mirror.and_then(|MirrorChoice(m)| m);
                let (job, number) =
                    add_instance(&job, id, setup, at, angle, mirror).map_err(|e| e.to_string())?;
                Ok::<_, String>((job, id, number))
            })();
            match made {
                Ok((job, id, number)) => {
                    if let Some(instance) = instance_of(&job, id, number) {
                        out.line(format!("Added instance {}", describe(&instance)));
                    }
                    out.set("instance", json!(number));
                    finish(out, &job, id, output.as_deref().unwrap_or(&project))
                }
                Err(err) => out.fail(err),
            }
        }
        InstanceAction::Set {
            project,
            part,
            ids,
            at,
            angle,
            mirror,
            off,
            on,
            output,
        } => {
            let job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            let changed = (|| {
                let id = part_of(&job, part)?;
                if at.is_some() && ids.len() > 1 {
                    return Err("--at moves one instance: give one number".to_string());
                }
                let change = InstanceChange {
                    at: at_arg(at.as_deref())?,
                    angle: angle.map(radians).transpose()?,
                    mirror: mirror.map(|MirrorChoice(m)| m),
                    enabled: match (off, on) {
                        (true, _) => Some(false),
                        (_, true) => Some(true),
                        _ => None,
                    },
                };
                let job = set_instances(&job, id, &ids, change).map_err(|e| e.to_string())?;
                Ok((job, id))
            })();
            match changed {
                Ok((job, id)) => {
                    for &number in &ids {
                        if let Some(instance) = instance_of(&job, id, number) {
                            out.line(format!("Instance {}", describe(&instance)));
                        }
                    }
                    finish(out, &job, id, output.as_deref().unwrap_or(&project))
                }
                Err(err) => out.fail(err),
            }
        }
        InstanceAction::Remove {
            project,
            part,
            ids,
            all,
            output,
        } => {
            let job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            let removed = (|| {
                let id = part_of(&job, part)?;
                let ids: Vec<u32> = match all {
                    // Every one but the first: the part is cut once.
                    true => job
                        .instances_of(id)
                        .iter()
                        .map(|(_, i)| i.id)
                        .skip(1)
                        .collect(),
                    false => ids,
                };
                let job = remove_instances(&job, id, &ids).map_err(|e| e.to_string())?;
                Ok::<_, String>((job, ids))
            })();
            match removed {
                Ok((job, ids)) => {
                    let numbers: Vec<String> = ids.iter().map(u32::to_string).collect();
                    out.line(match numbers.as_slice() {
                        [] => "No instances to remove".to_string(),
                        [one] => format!("Removed instance {one}"),
                        _ => format!("Removed instances {}", numbers.join(", ")),
                    });
                    write_project(out, &job, output.as_deref().unwrap_or(&project))
                }
                Err(err) => out.fail(err),
            }
        }
    }
}

/// Write the project and say if the part's instances now cut into each other or into
/// another part: refused when the project is run.
fn finish(out: &mut Out, job: &Job, part: u64, target: &Path) -> ExitCode {
    if let Err(err) = crate::save_project(out, job, target) {
        return out.fail(err);
    }
    warn_overlap(out, job, part);
    out.wrote(target, "");
    ExitCode::SUCCESS
}

/// Instance `number` of the part, on whichever board it is.
fn instance_of(job: &Job, part: u64, number: u32) -> Option<Instance> {
    job.instance(part, number).copied()
}

/// ", placed at 120.000, 0.000 mm, turned 90°" for a part placed elsewhere than drawn
/// (docs/specs/part-placement.md); nothing for one cut where drawn.
pub(crate) fn placed_text(job: &Job, part: &chipload_job::Part) -> String {
    let Some(own) = job.instance(part.id, 1).filter(|_| job.is_placed(part.id)) else {
        return String::new();
    };
    let mut text = format!(", placed at {}, {} mm", mm3(own.at[0]), mm3(own.at[1]));
    if own.angle != 0.0 {
        text += &format!(", turned {}°", degrees_text(own.angle));
    }
    text
}

/// `part place`: the part itself moved and turned, or back where drawn.
pub(crate) fn place(
    out: &mut Out,
    project: &Path,
    part: Option<u64>,
    at: Option<&str>,
    angle: Option<f64>,
    reset: bool,
    target: &Path,
) -> ExitCode {
    let job = match load(out, project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let placed = (|| {
        let id = part_of(&job, part)?;
        if at.is_none() && angle.is_none() && !reset {
            return Err("give --at, --angle or --reset".to_string());
        }
        let at = at_arg(at)?;
        let angle = angle.map(radians).transpose()?;
        let job = chipload_pipeline::instances::place_part(&job, id, at, angle, reset)
            .map_err(|e| e.to_string())?;
        Ok((job, id))
    })();
    match placed {
        Ok((job, id)) => {
            if let (Some(part), Some(own)) = (job.part(id), job.instance(id, 1)) {
                let text = placed_text(&job, part);
                out.line(match text.strip_prefix(", ") {
                    Some(placed) => format!("Part {} {}: {placed}", part.id, part.name),
                    None => format!("Part {} {}: cut where drawn", part.id, part.name),
                });
                out.set("angle", json!(own.angle));
                out.set("at", json!(own.at));
            }
            finish(out, &job, id, target)
        }
        Err(err) => out.fail(err),
    }
}

/// "3 at 120.000, 0.000 mm, turned 90°, mirrored left-right, off".
fn describe(instance: &Instance) -> String {
    let mut text = format!(
        "{} at {}, {} mm",
        instance.id,
        mm3(instance.at[0]),
        mm3(instance.at[1])
    );
    if instance.angle != 0.0 {
        text += &format!(", turned {}°", degrees_text(instance.angle));
    }
    if let Some(mirror) = instance.mirror {
        text += &format!(", mirrored {}", mirror.name());
    }
    if !instance.enabled {
        text += ", off";
    }
    text
}

/// A length with 3 decimals; one that rounds to nothing is 0.000, never -0.000.
pub(crate) fn mm3(value: f64) -> String {
    let text = format!("{value:.3}");
    if text == "-0.000" {
        "0.000".to_string()
    } else {
        text
    }
}

/// An angle in degrees, with as few decimals as it needs (at most 3).
pub(crate) fn degrees_text(radians: f64) -> String {
    let text = format!("{:.3}", radians.to_degrees());
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn list(out: &mut Out, project: &Path, part: Option<u64>) -> ExitCode {
    let job = match load(out, project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let id = match part_of(&job, part) {
        Ok(id) => id,
        Err(err) => return out.fail(err),
    };
    let Some(part) = job.part(id) else {
        return out.fail(format!("no part {id} in the project (see `part list`)"));
    };
    let all = job.instances_of(part.id);
    if all.len() < 2 && !job.is_placed(part.id) {
        out.line(format!(
            "Part {} {}: no instances, cut once",
            part.id, part.name
        ));
    } else {
        out.line(format!(
            "Part {} {}: {}",
            part.id,
            part.name,
            chipload_pipeline::copies::instances_message_everywhere(&job, part).english()
        ));
    }
    // Each instance's board, once there are several (docs/specs/parts-and-setups.md §10).
    let board = |setup: u64| {
        if job.setups.len() > 1 {
            format!(", setup {setup}")
        } else {
            String::new()
        }
    };
    // Every instance alike, once there is more than the part where drawn.
    if all.len() > 1 || job.is_placed(part.id) {
        for (setup, instance) in &all {
            out.line(format!("  {}{}", describe(instance), board(*setup)));
        }
    }
    let rows: Vec<Value> = all
        .iter()
        .map(|(setup, i)| {
            json!({
                "id": i.id,
                "setup": setup,
                "enabled": i.enabled,
                "mirror": i.mirror,
                "angle": i.angle,
                "at": i.at,
            })
        })
        .collect();
    out.set("part", json!(part.id));
    out.set("order", json!(part.order));
    out.set("instances", Value::Array(rows));
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn angles_come_in_degrees_and_go_out_as_few_decimals() {
        let quarter = radians(90.0).unwrap();
        assert!((quarter - std::f64::consts::FRAC_PI_2).abs() < 1e-15);
        assert!((radians(270.0).unwrap() + std::f64::consts::FRAC_PI_2).abs() < 1e-12);
        assert_eq!(degrees_text(quarter), "90");
        assert_eq!(degrees_text(0.5_f64.to_radians()), "0.5");
        assert!(radians(f64::NAN).is_err());
    }
}
