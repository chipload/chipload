// SPDX-License-Identifier: GPL-3.0-or-later
//! Stopping a planning run from another thread (docs/specs/background-generation.md §2),
//! and how far it has got (§7).
//!
//! Cooperative: the planner checks the flag between operations, pocket pieces and other
//! long steps and stops with [`ToolpathError::Cancelled`]. A run that isn't cancelled is
//! exactly the same as one without a flag.
//!
//! Progress is a share from 0 to 1 that only rises, so a window can show it as a bar: a
//! caller marks out a *window* of the whole (planning the front, then simulating it, then
//! the back), the work a *stage* of that window (an operation of the plan), and reports
//! the share of the stage done (a relief's lines). It changes nothing that is computed.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::ToolpathError;

/// A flag a planning run checks; set it from any thread to stop the run.
#[derive(Debug, Default)]
pub struct Cancel {
    flag: AtomicBool,
    /// The share done, an `f64`'s bits: never negative, so the bits rise with the value.
    done: AtomicU64,
    /// The window's start and width, and the stage's within it, `f64` bits (0 for the
    /// width means the whole: a flag made with `default` has no window yet).
    window: [AtomicU64; 2],
    stage: [AtomicU64; 2],
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

    /// The share of the whole done, 0 to 1.
    pub fn progress(&self) -> f64 {
        f64::from_bits(self.done.load(Ordering::Relaxed))
    }

    /// Mark out the window the work from now on fills, as shares of the whole; its stage
    /// starts as the whole window.
    pub fn set_window(&self, start: f64, width: f64) {
        store(&self.window[0], start);
        store(&self.window[1], width);
        self.set_stage(0.0, 1.0);
    }

    /// Mark out the stage the work from now on fills, as shares of the window, and report
    /// its start.
    pub fn set_stage(&self, start: f64, width: f64) {
        store(&self.stage[0], start);
        store(&self.stage[1], width);
        self.stage_done(0.0);
    }

    /// Report `share` of the stage done (0 to 1); a smaller share than before changes
    /// nothing.
    pub fn stage_done(&self, share: f64) {
        let (window, stage) = (self.span(&self.window), self.span(&self.stage));
        let within = stage.0 + stage.1 * share.clamp(0.0, 1.0);
        let whole = (window.0 + window.1 * within).clamp(0.0, 1.0);
        if whole.is_finite() {
            self.done.fetch_max(whole.to_bits(), Ordering::Relaxed);
        }
    }

    /// A start and width; no width set yet means all of it.
    fn span(&self, of: &[AtomicU64; 2]) -> (f64, f64) {
        let width = f64::from_bits(of[1].load(Ordering::Relaxed));
        if width > 0.0 {
            (f64::from_bits(of[0].load(Ordering::Relaxed)), width)
        } else {
            (0.0, 1.0)
        }
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

/// `value` kept in `cell`, never negative.
fn store(cell: &AtomicU64, value: f64) {
    let value = if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    };
    cell.store(value.to_bits(), Ordering::Relaxed);
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

    #[test]
    fn progress_fills_windows_and_stages_and_only_rises() {
        let cancel = Cancel::new();
        assert_eq!(cancel.progress(), 0.0);
        // No window: the stage's share is the whole's.
        cancel.stage_done(0.25);
        assert_eq!(cancel.progress(), 0.25);
        // The second half, its second quarter half done: 0.5 + 0.5 · (0.25 + 0.25 · 0.5).
        cancel.set_window(0.5, 0.5);
        cancel.set_stage(0.25, 0.25);
        cancel.stage_done(0.5);
        assert!((cancel.progress() - 0.6875).abs() < 1e-12);
        // Never back: an earlier stage or a smaller share leaves it.
        cancel.set_stage(0.0, 0.25);
        cancel.stage_done(0.1);
        assert!((cancel.progress() - 0.6875).abs() < 1e-12);
        cancel.set_window(0.0, 1.0);
        cancel.stage_done(1.0);
        assert_eq!(cancel.progress(), 1.0);
    }
}
