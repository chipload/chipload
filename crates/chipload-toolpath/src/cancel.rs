// SPDX-License-Identifier: GPL-3.0-or-later
//! Stopping a planning run from another thread (docs/specs/background-generation.md §2).
//!
//! Cooperative: the planner checks the flag between operations, pocket pieces and other
//! long steps and stops with [`ToolpathError::Cancelled`]. A run that isn't cancelled is
//! exactly the same as one without a flag.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::ToolpathError;

/// A flag a planning run checks; set it from any thread to stop the run.
#[derive(Debug, Default)]
pub struct Cancel {
    flag: AtomicBool,
}

impl Cancel {
    /// A flag that isn't set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Ask the run to stop at its next check.
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::Relaxed)
    }

    /// `Err(Cancelled)` once the flag is set.
    pub fn check(&self) -> Result<(), ToolpathError> {
        if self.is_cancelled() {
            Err(ToolpathError::Cancelled)
        } else {
            Ok(())
        }
    }
}

/// [`Cancel::check`] for an optional flag.
pub(crate) fn check(cancel: Option<&Cancel>) -> Result<(), ToolpathError> {
    cancel.map_or(Ok(()), Cancel::check)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_pass_until_the_flag_is_set() {
        let cancel = Cancel::new();
        assert!(cancel.check().is_ok());
        assert!(check(None).is_ok());
        cancel.cancel();
        assert_eq!(cancel.check(), Err(ToolpathError::Cancelled));
        assert_eq!(check(Some(&cancel)), Err(ToolpathError::Cancelled));
    }
}
