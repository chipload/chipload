// SPDX-License-Identifier: GPL-3.0-or-later
//! Choosing a project's post-processor: built-in by name, or a user post from the posts
//! folder, copied into the project (docs/specs/user-posts.md).

use std::path::{Path, PathBuf};

use chipload_i18n::{Digits, Message, Value};
use chipload_job::{CustomPost, Job};
use chipload_post::{
    PostConfig, UserPost, list_user_posts, load_builtin_post, parse_user_post, read_user_post,
};

pub use chipload_post::builtin_posts;

use crate::PipelineError;

/// The posts folder's name inside the settings folder.
pub const POSTS_DIR: &str = "posts";

pub fn posts_dir(settings: &Path) -> PathBuf {
    settings.join(POSTS_DIR)
}

/// The post a job renders with: its copy of a user post, else the built-in it names.
pub fn job_post(job: &Job) -> Result<PostConfig, PipelineError> {
    Ok(match &job.machine.custom_post {
        Some(copy) => parse_user_post(&UserPost {
            config: copy.config.clone(),
            template: copy.template.clone(),
        })?,
        None => load_builtin_post(&job.machine.post)?,
    })
}

/// A post that can be chosen, for lists.
#[derive(Debug, Clone, PartialEq)]
pub struct PostChoice {
    pub name: String,
    pub builtin: bool,
    /// Why it can't be used; None: usable.
    pub problem: Option<Message>,
}

/// Every post: built-in ones first, then the user's (sorted), with problems.
pub fn post_choices(posts: Option<&Path>) -> Vec<PostChoice> {
    let mut choices: Vec<PostChoice> = builtin_posts()
        .into_iter()
        .map(|name| PostChoice {
            name: name.to_string(),
            builtin: true,
            problem: None,
        })
        .collect();
    if let Some(dir) = posts {
        choices.extend(
            list_user_posts(dir)
                .into_iter()
                .map(|(name, read)| PostChoice {
                    name,
                    builtin: false,
                    problem: read.err().map(|e| e.message()),
                }),
        );
    }
    choices
}

/// What a post-processor is, as lines for the user (docs/specs/this-computer.md §4): its
/// name in the file, built in or yours (with its files), the file extension, decimals,
/// arcs, the tool change modes and the spindle dwell. An error says why it can't be used.
pub fn post_facts(name: &str, posts: Option<&Path>) -> Result<Vec<String>, PipelineError> {
    Ok(post_fact_messages(name, posts)?
        .iter()
        .map(Message::english)
        .collect())
}

/// [`post_facts`] as messages.
pub fn post_fact_messages(name: &str, posts: Option<&Path>) -> Result<Vec<Message>, PipelineError> {
    let mut lines = Vec::new();
    let config = if builtin_posts().contains(&name) {
        lines.push(Message::new("pipeline-post-builtin"));
        load_builtin_post(name)?
    } else {
        let dir = posts.ok_or_else(|| {
            PipelineError::Job(Message::new("pipeline-post-no-settings").arg("name", name))
        })?;
        let read = read_user_post(dir, name)?;
        let config = parse_user_post(&read)?;
        lines.push(Message::new("pipeline-post-yours"));
        lines.push(Message::new("pipeline-post-settings").arg(
            "path",
            dir.join(format!("{name}.toml")).display().to_string(),
        ));
        lines.push(
            Message::new("pipeline-post-template")
                .arg("path", dir.join(&config.template).display().to_string()),
        );
        config
    };
    lines.insert(
        0,
        Message::new("pipeline-post-name").arg("name", &config.name),
    );
    lines.push(Message::new("pipeline-post-extension").arg("extension", &config.file_extension));
    lines.push(
        Message::new("pipeline-post-decimals")
            .arg("mm", config.decimals)
            .arg("inch", config.inch_decimals),
    );
    let units: Vec<&str> = config.units.iter().map(|u| u.name()).collect();
    lines.push(Message::new("pipeline-post-units-list").arg("units", units.join(", ")));
    lines.push(
        Message::new("pipeline-post-arcs").arg("arcs", if config.arcs { "arcs" } else { "lines" }),
    );
    let modes: Vec<&str> = config.tool_changes.iter().map(|c| c.name()).collect();
    lines.push(Message::new("pipeline-post-tool-changes").arg("modes", modes.join(", ")));
    lines.push(if config.spindle_dwell_s > 0.0 {
        Message::new("pipeline-post-dwell").arg(
            "seconds",
            Value::Number(config.spindle_dwell_s, Digits::Shortest),
        )
    } else {
        Message::new("pipeline-post-no-dwell")
    });
    Ok(lines)
}

/// Make `name` the job's post: a built-in by name, or a user post from `posts`, copied in
/// (again, if it was already: that updates the copy).
pub fn choose_post(job: &mut Job, name: &str, posts: Option<&Path>) -> Result<(), PipelineError> {
    if builtin_posts().contains(&name) {
        job.machine.post = name.to_string();
        job.machine.custom_post = None;
        return Ok(());
    }
    let Some(dir) = posts else {
        return Err(PipelineError::Job(
            Message::new("pipeline-post-no-folder").arg("name", name),
        ));
    };
    let read = read_user_post(dir, name)?;
    job.machine.post = name.to_string();
    job.machine.custom_post = Some(CustomPost {
        config: read.config,
        template: read.template,
    });
    Ok(())
}

/// How a project's copy of a user post compares with the file it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CopyState {
    /// A built-in post: nothing copied.
    Builtin,
    Same,
    /// The file in the posts folder has changed since it was copied.
    Changed,
    /// The file is gone or can't be read (the project still renders from its copy).
    Missing(Message),
}

pub fn copy_state(job: &Job, posts: Option<&Path>) -> CopyState {
    let Some(copy) = &job.machine.custom_post else {
        return CopyState::Builtin;
    };
    let Some(dir) = posts else {
        return CopyState::Missing(Message::new("pipeline-post-no-posts-folder"));
    };
    match read_user_post(dir, &job.machine.post) {
        Ok(read) if read.config == copy.config && read.template == copy.template => CopyState::Same,
        Ok(_) => CopyState::Changed,
        Err(err) => CopyState::Missing(err.message()),
    }
}

/// Whether a machine profile's post can be used: built in, or a readable user post.
pub fn post_exists(name: &str, posts: Option<&Path>) -> bool {
    builtin_posts().contains(&name) || posts.is_some_and(|dir| read_user_post(dir, name).is_ok())
}
