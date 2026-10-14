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
pub mod tabs;
pub mod vcarve;

pub use cancel::Cancel;
pub use plan::{
    OpSpan, Plan, RingStart, ToolGroup, job_problems, op_tool, plan_job, plan_job_cancellable,
    tool_groups, validate_job,
};

/// Why no toolpath could be made. Messages are the user-facing text.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ToolpathError {
    /// The job asks for something this version can't or won't do.
    #[error("{0}")]
    Job(String),
    /// A selector matches nothing, only part of a contour, or an invalid contour.
    #[error("{0}")]
    Selector(String),
    /// The geometry can't be machined as asked (tool too big, failed verification, ...).
    #[error("{0}")]
    Geometry(String),
    /// Copies or parts would cut into each other (docs/specs/copies-grid.md §4,
    /// docs/specs/parts.md §5).
    #[error("{0}")]
    Overlap(String),
    /// An error of the operation at `index` in `job.operations`: its message already names
    /// the operation, so this only says which one, for anything that leads the user to it.
    #[error("{source}")]
    Operation {
        index: usize,
        source: Box<ToolpathError>,
    },
    /// The run was stopped by its [`cancel::Cancel`] flag.
    #[error("Generation cancelled")]
    Cancelled,
}

impl ToolpathError {
    /// The same error with "`context`: " in front of its message.
    pub fn with_context(self, context: &str) -> Self {
        match self {
            ToolpathError::Job(m) => ToolpathError::Job(format!("{context}: {m}")),
            ToolpathError::Selector(m) => ToolpathError::Selector(format!("{context}: {m}")),
            ToolpathError::Geometry(m) => ToolpathError::Geometry(format!("{context}: {m}")),
            ToolpathError::Overlap(m) => ToolpathError::Overlap(format!("{context}: {m}")),
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
