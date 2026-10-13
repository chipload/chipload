// SPDX-License-Identifier: GPL-3.0-or-later
//! Toolpath generation: selection, offsets, profiles and job planning.
//!
//! Headless, no I/O. Offsets use cavalier_contours with true arcs and are always verified
//! afterwards (ADR 0006).

pub mod adaptive;
pub mod arcs;
pub mod cancel;
pub mod fronts;
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
pub mod spiral;
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
            ToolpathError::Cancelled => ToolpathError::Cancelled,
        }
    }
}
