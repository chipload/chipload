// SPDX-License-Identifier: GPL-3.0-or-later
//! Why a project couldn't be changed, planned or written.

use chipload_dxf::ImportError;
use chipload_i18n::Message;
use chipload_post::PostError;
use chipload_toolpath::ToolpathError;
use chipload_toolpath::model::Program;

/// Why a project couldn't be changed, planned or written. Its text is a message, shown in
/// the app's language; `Display` writes it in English for the command line.
#[derive(Debug, Clone, PartialEq)]
pub enum PipelineError {
    /// A change or run the project refuses (`pipeline.ftl`, or the problems of `job.ftl`).
    Job(Message),
    Import(ImportError),
    Toolpath(ToolpathError),
    Image(chipload_image::ImageError),
    Text(chipload_text::TextError),
    Post(PostError),
    /// The independent checker rejected the program. No G-code is returned; `program` is
    /// the rejected toolpath, for display only, and must never be posted or written out.
    /// `messages` are `errors` as messages, for the app (docs/specs/translations.md §7).
    CheckFailed {
        errors: Vec<String>,
        messages: Vec<Message>,
        program: Box<Program>,
    },
}

impl PipelineError {
    /// The error as a message.
    pub fn message(&self) -> Message {
        match self {
            PipelineError::Job(message) => message.clone(),
            PipelineError::Import(e) => e.message(),
            PipelineError::Toolpath(e) => e.message(),
            PipelineError::Image(e) => e.message(),
            PipelineError::Text(e) => e.message(),
            PipelineError::Post(e) => e.message(),
            PipelineError::CheckFailed { messages, .. } => messages.iter().fold(
                Message::new("pipeline-error-check-failed"),
                |list, finding| {
                    Message::new("pipeline-error-check-finding")
                        .arg("list", list)
                        .arg("finding", finding.clone())
                },
            ),
        }
    }
}

impl std::fmt::Display for PipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message().english())
    }
}

impl std::error::Error for PipelineError {}

impl From<ImportError> for PipelineError {
    fn from(e: ImportError) -> Self {
        PipelineError::Import(e)
    }
}

impl From<ToolpathError> for PipelineError {
    fn from(e: ToolpathError) -> Self {
        PipelineError::Toolpath(e)
    }
}

impl From<chipload_image::ImageError> for PipelineError {
    fn from(e: chipload_image::ImageError) -> Self {
        PipelineError::Image(e)
    }
}

impl From<chipload_text::TextError> for PipelineError {
    fn from(e: chipload_text::TextError) -> Self {
        PipelineError::Text(e)
    }
}

impl From<PostError> for PipelineError {
    fn from(e: PostError) -> Self {
        PipelineError::Post(e)
    }
}
