// SPDX-License-Identifier: GPL-3.0-or-later
//! Choosing a project's post-processor: built-in by name, or a user post from the posts
//! folder, copied into the project (docs/specs/user-posts.md).

use std::path::{Path, PathBuf};

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
    pub problem: Option<String>,
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
                    problem: read.err().map(|e| e.to_string()),
                }),
        );
    }
    choices
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
        return Err(PipelineError::Job(format!(
            "No post-processor \"{name}\": it isn't built in, and there is no posts folder"
        )));
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
    Missing(String),
}

pub fn copy_state(job: &Job, posts: Option<&Path>) -> CopyState {
    let Some(copy) = &job.machine.custom_post else {
        return CopyState::Builtin;
    };
    let Some(dir) = posts else {
        return CopyState::Missing("no posts folder".into());
    };
    match read_user_post(dir, &job.machine.post) {
        Ok(read) if read.config == copy.config && read.template == copy.template => CopyState::Same,
        Ok(_) => CopyState::Changed,
        Err(err) => CopyState::Missing(err.to_string()),
    }
}

/// Whether a machine profile's post can be used: built in, or a readable user post.
pub fn post_exists(name: &str, posts: Option<&Path>) -> bool {
    builtin_posts().contains(&name) || posts.is_some_and(|dir| read_user_post(dir, name).is_ok())
}
