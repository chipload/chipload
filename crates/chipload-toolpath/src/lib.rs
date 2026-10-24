// SPDX-License-Identifier: GPL-3.0-or-later
//! Toolpath generation: selection, offsets, profiles and job planning.
//!
//! Headless, no I/O. Offsets use cavalier_contours with true arcs and are always verified
//! afterwards (ADR 0006).

pub mod arcs;
pub mod cancel;
pub(crate) mod cavalier;
pub mod centre;
pub mod corners;
pub mod geom;
pub mod helix;
pub mod holes;
pub mod lap;
pub mod lead;
pub mod left;
pub(crate) mod medial;
pub mod merge;
pub mod model;
pub mod offset;
pub mod path;
pub mod plan;
pub mod pocket;
pub mod profile;
pub mod ramp;
mod retry;
pub mod select;
pub mod start;
pub mod surface;
pub mod tabs;
pub mod vcarve;

pub use cancel::Cancel;
use chipload_i18n::Message;
pub use plan::{
    OpSpan, Plan, RingStart, ToolGroup, job_problems, op_tool, plan_job, plan_job_cancellable,
    tool_groups, validate_job,
};

/// Why no toolpath could be made. Each holds its message (`toolpath.ftl`), shown in the
/// app's language; `Display` writes it in English for the command line.
#[derive(Debug, Clone, PartialEq)]
pub enum ToolpathError {
    /// The job asks for something this version can't or won't do.
    Job(Message),
    /// A selector matches nothing, only part of a contour, or an invalid contour.
    Selector(Message),
    /// The geometry can't be machined as asked (tool too big, failed verification, ...).
    Geometry(Message),
    /// Copies or parts would cut into each other (docs/specs/copies-grid.md §4,
    /// docs/specs/parts.md §5).
    Overlap(Message),
    /// An error of the operation at `index` in `job.operations`: its message already names
    /// the operation, so this only says which one, for anything that leads the user to it.
    Operation {
        index: usize,
        source: Box<ToolpathError>,
    },
    /// The run was stopped by its [`cancel::Cancel`] flag.
    Cancelled,
}

impl std::fmt::Display for ToolpathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message().english())
    }
}

impl std::error::Error for ToolpathError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ToolpathError::Operation { source, .. } => Some(source.as_ref()),
            _ => None,
        }
    }
}

impl ToolpathError {
    /// The error as a message.
    pub fn message(&self) -> Message {
        match self {
            ToolpathError::Job(m)
            | ToolpathError::Selector(m)
            | ToolpathError::Geometry(m)
            | ToolpathError::Overlap(m) => m.clone(),
            ToolpathError::Operation { source, .. } => source.message(),
            ToolpathError::Cancelled => Message::new("toolpath-error-cancelled"),
        }
    }

    /// The same error with "`context`: " in front of its message.
    pub fn with_context(self, context: Message) -> Self {
        let wrap = |m: Message| {
            Message::new("toolpath-error-context")
                .arg("context", context.clone())
                .arg("error", m)
        };
        match self {
            ToolpathError::Job(m) => ToolpathError::Job(wrap(m)),
            ToolpathError::Selector(m) => ToolpathError::Selector(wrap(m)),
            ToolpathError::Geometry(m) => ToolpathError::Geometry(wrap(m)),
            ToolpathError::Overlap(m) => ToolpathError::Overlap(wrap(m)),
            ToolpathError::Operation { index, source } => ToolpathError::Operation {
                index,
                source: Box::new(source.with_context(context)),
            },
            ToolpathError::Cancelled => ToolpathError::Cancelled,
        }
    }

    /// The same error, marked as the operation at `index`'s (an error already marked keeps
    /// its operation; a cancellation is nobody's).
    pub fn at(self, index: usize) -> Self {
        match self {
            ToolpathError::Operation { .. } | ToolpathError::Cancelled => self,
            other => ToolpathError::Operation {
                index,
                source: Box::new(other),
            },
        }
    }

    /// The operation (index in `job.operations`) the error is about, if it is one's.
    pub fn operation(&self) -> Option<usize> {
        match self {
            ToolpathError::Operation { index, .. } => Some(*index),
            _ => None,
        }
    }

    /// The error itself, without the operation it was marked with.
    pub fn cause(&self) -> &ToolpathError {
        match self {
            ToolpathError::Operation { source, .. } => source.cause(),
            other => other,
        }
    }
}
