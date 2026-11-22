// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload post …`: the built-in and the user's post-processors.

use super::*;
use crate::load;
use crate::out::Out;
use chipload_i18n::Message;
use chipload_pipeline::posts::{
    CopyState, choose_post, copy_state, post_choices, post_facts, posts_dir,
};
use clap::Subcommand;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Subcommand)]
pub enum PostAction {
    /// List the built-in post-processors and yours, and where yours go.
    List,
    /// What a post-processor writes: its name, files, extension, decimals, arcs, tool change
    /// modes and spindle dwell.
    Show {
        /// The post's name (`chipload post list`).
        name: String,
    },
    /// Make a post-processor the project's; one of yours is copied into the project (run
    /// again after changing it, to update the copy).
    Use {
        /// The project (.chipload).
        project: PathBuf,
        /// The post's name (`chipload post list`).
        #[arg(long)]
        post: String,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

pub fn post(out: &mut Out, action: PostAction) -> ExitCode {
    let dir = match dir() {
        Ok(dir) => dir,
        Err(err) => return out.fail(err),
    };
    let posts = posts_dir(&dir);
    match action {
        PostAction::Show { name } => match post_facts(&name, Some(&posts)) {
            Ok(lines) => {
                out.line(format!("Post-processor \"{name}\":"));
                for line in &lines {
                    out.line(format!("  {line}"));
                }
                out.set("name", serde_json::json!(name));
                out.set("facts", serde_json::json!(lines));
                ExitCode::SUCCESS
            }
            Err(err) => out.fail(err),
        },
        PostAction::List => {
            out.line(format!("Your post-processors: {}", posts.display()));
            let choices = post_choices(Some(&posts));
            out.set("path", serde_json::json!(posts.display().to_string()));
            out.set(
                "posts",
                serde_json::json!(
                    choices
                        .iter()
                        .map(|c| serde_json::json!({
                            "name": c.name,
                            "builtin": c.builtin,
                            "controllers": c.controllers,
                            "problem": c.problem.as_ref().map(Message::english),
                        }))
                        .collect::<Vec<_>>()
                ),
            );
            for choice in choices {
                let kind = if choice.builtin { "built in" } else { "yours" };
                let for_ = if choice.controllers.is_empty() {
                    String::new()
                } else {
                    format!(": {}", choice.controllers.join(", "))
                };
                match &choice.problem {
                    None => out.line(format!("  {} ({kind}){for_}", choice.name)),
                    Some(problem) => {
                        out.line(format!(
                            "  {} ({kind}) can't be used: {}",
                            choice.name,
                            problem.english()
                        ));
                    }
                }
            }
            ExitCode::SUCCESS
        }
        PostAction::Use {
            project,
            post,
            output,
        } => {
            let mut job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            let before = copy_state(&job, Some(&posts));
            let was = job.machine.post.clone();
            if let Err(err) = choose_post(&mut job, &post, Some(&posts)) {
                return out.fail(err);
            }
            let target = output.as_deref().unwrap_or(&project);
            if let Err(err) = crate::save_project(out, &job, target) {
                return out.fail(err);
            }
            match (&job.machine.custom_post, was == post, before) {
                (None, _, _) => out.line(format!("Post-processor: {post} (built in)")),
                (Some(_), true, CopyState::Same) => {
                    out.line(format!(
                        "Post-processor: {post} (yours; the copy was up to date)"
                    ));
                }
                (Some(_), true, _) => {
                    out.line(format!("Post-processor: {post} (yours; copy updated)"))
                }
                (Some(_), false, _) => {
                    out.line(format!("Post-processor: {post} (yours, copied in)"))
                }
            }
            out.wrote(target, "");
            ExitCode::SUCCESS
        }
    }
}
