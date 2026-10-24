// SPDX-License-Identifier: GPL-3.0-or-later
//! Retrying an offset that hit a numerical coincidence with a negligibly different one
//! (docs/specs/offset-retry.md).

use chipload_core::tolerances::{
    ARC_APPROX_TOL_MM, OFFSET_RETRY_NUDGE_MM, OFFSET_RETRY_WIDE_NUDGE_MM,
};

use crate::ToolpathError;

/// How far each attempt after the first failed one moves its offsets, mm: three of
/// [`OFFSET_RETRY_NUDGE_MM`], then three of [`OFFSET_RETRY_WIDE_NUDGE_MM`] (spec §7).
pub(crate) const NUDGES: [f64; 6] = [
    OFFSET_RETRY_NUDGE_MM,
    2.0 * OFFSET_RETRY_NUDGE_MM,
    3.0 * OFFSET_RETRY_NUDGE_MM,
    OFFSET_RETRY_WIDE_NUDGE_MM,
    2.0 * OFFSET_RETRY_WIDE_NUDGE_MM,
    3.0 * OFFSET_RETRY_WIDE_NUDGE_MM,
];

/// How far an attempt moves its offsets, in mm: both zero on the first attempt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Nudge {
    /// Taken off the stepover between rings.
    pub step: f64,
    /// Added to the offset from the walls.
    pub wall: f64,
    /// Pocket shape offsets with the tight slice tolerance (`nudged_or_tight`).
    pub tight: bool,
    /// The pocket's region with its arcs as chords, and the walls moved out by the arc
    /// tolerance so the chords never let the tool nearer than the arcs would (the last
    /// attempt of `nudged_or_tight`, spec §8).
    pub flat: bool,
}

impl Nudge {
    pub const NONE: Nudge = Nudge {
        step: 0.0,
        wall: 0.0,
        tight: false,
        flat: false,
    };
}

/// Why an attempt failed.
#[derive(Debug)]
pub(crate) enum Failure {
    /// A nudge can't change it (the tool doesn't fit, too many rings, ...).
    Final(ToolpathError),
    /// The geometry library or the verification failed, possibly by a numerical coincidence.
    /// `wall`: at the offset from the walls itself, so later attempts move that too.
    Numerical { error: ToolpathError, wall: bool },
}

impl From<ToolpathError> for Failure {
    fn from(error: ToolpathError) -> Self {
        Failure::Final(error)
    }
}

/// The first attempt that succeeds; after a numerical failure up to six more, nudged by each
/// of [`NUDGES`] in turn. If none succeeds, the first attempt's error.
pub(crate) fn nudged<T>(
    attempt: impl FnMut(Nudge) -> Result<T, Failure>,
) -> Result<T, ToolpathError> {
    retried(attempt, false)
}

/// [`nudged`], and when every attempt failed by a numerical coincidence, two more: un-nudged,
/// with the tight slice tolerance in the shape offsets (docs/specs/offset-retry.md §6), then
/// with the arcs as chords (§8). If those fail too, the first attempt's error.
pub(crate) fn nudged_or_tight<T>(
    attempt: impl FnMut(Nudge) -> Result<T, Failure>,
) -> Result<T, ToolpathError> {
    retried(attempt, true)
}

fn retried<T>(
    mut attempt: impl FnMut(Nudge) -> Result<T, Failure>,
    tight: bool,
) -> Result<T, ToolpathError> {
    let (error, mut move_wall) = match attempt(Nudge::NONE) {
        Ok(value) => return Ok(value),
        Err(Failure::Final(error)) => return Err(error),
        Err(Failure::Numerical { error, wall }) => (error, wall),
    };
    for by in NUDGES {
        let nudge = Nudge {
            step: by,
            wall: if move_wall { by } else { 0.0 },
            tight: false,
            flat: false,
        };
        match attempt(nudge) {
            Ok(value) => return Ok(value),
            Err(Failure::Final(ToolpathError::Cancelled)) => return Err(ToolpathError::Cancelled),
            Err(Failure::Final(_)) => break,
            Err(Failure::Numerical { wall, .. }) => move_wall |= wall,
        }
    }
    if tight {
        let nudge = Nudge {
            tight: true,
            ..Nudge::NONE
        };
        match attempt(nudge) {
            Ok(value) => return Ok(value),
            Err(Failure::Final(ToolpathError::Cancelled)) => return Err(ToolpathError::Cancelled),
            Err(_) => {}
        }
        // Last, the arcs as chords: cavalier_contours can lose part of a shape whose arc
        // runs into a sharp inside notch, at many distances at once (spec §8).
        let nudge = Nudge {
            wall: ARC_APPROX_TOL_MM,
            flat: true,
            ..Nudge::NONE
        };
        match attempt(nudge) {
            Ok(value) => return Ok(value),
            Err(Failure::Final(ToolpathError::Cancelled)) => return Err(ToolpathError::Cancelled),
            Err(_) => {}
        }
    }
    Err(error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chipload_i18n::Message;

    /// An error told apart from others by `text`.
    fn error(text: &str) -> ToolpathError {
        ToolpathError::Geometry(Message::new("toolpath-error-joined-cross").arg("test", text))
    }

    fn numerical(text: &str, wall: bool) -> Failure {
        Failure::Numerical {
            error: error(text),
            wall,
        }
    }

    #[test]
    fn a_success_is_never_retried() {
        let mut seen = Vec::new();
        let result = nudged(|n| {
            seen.push(n);
            Ok::<_, Failure>(7)
        });
        assert_eq!(result, Ok(7));
        assert_eq!(seen, [Nudge::NONE]);
    }

    #[test]
    fn retries_nudge_the_stepover_and_move_the_wall_only_after_it_failed() {
        let mut seen = Vec::new();
        let result: Result<(), _> = nudged(|n| {
            seen.push(n);
            Err(numerical(
                &format!("attempt {}", seen.len()),
                seen.len() == 2,
            ))
        });
        assert_eq!(result, Err(error("attempt 1")));
        let (e, w) = (OFFSET_RETRY_NUDGE_MM, OFFSET_RETRY_WIDE_NUDGE_MM);
        let nudge = |step: f64, wall: f64| Nudge {
            step,
            wall,
            ..Nudge::NONE
        };
        assert_eq!(
            seen,
            [
                Nudge::NONE,
                nudge(e, 0.0),
                nudge(2.0 * e, 2.0 * e),
                nudge(3.0 * e, 3.0 * e),
                nudge(w, w),
                nudge(2.0 * w, 2.0 * w),
                nudge(3.0 * w, 3.0 * w),
            ]
        );
    }

    #[test]
    fn a_wall_failure_first_moves_the_wall_from_the_first_retry() {
        let mut seen = Vec::new();
        let result = nudged(|n| {
            seen.push(n);
            if seen.len() < 3 {
                Err(numerical("wall", true))
            } else {
                Ok(seen.len())
            }
        });
        assert_eq!(result, Ok(3));
        let e = OFFSET_RETRY_NUDGE_MM;
        assert_eq!(
            seen[1],
            Nudge {
                step: e,
                wall: e,
                ..Nudge::NONE
            }
        );
    }

    #[test]
    fn errors_a_nudge_cannot_change_end_the_retries_with_the_first_error() {
        let mut calls = 0;
        let result: Result<(), _> = nudged(|_| {
            calls += 1;
            Err(Failure::Final(error("does not fit")))
        });
        assert_eq!(result, Err(error("does not fit")));
        assert_eq!(calls, 1);
        let mut calls = 0;
        let result: Result<(), _> = nudged(|_| {
            calls += 1;
            if calls == 1 {
                Err(numerical("verification", false))
            } else {
                Err(Failure::Final(error("does not fit")))
            }
        });
        assert_eq!(result, Err(error("verification")));
        assert_eq!(calls, 2);
    }

    #[test]
    fn the_tight_attempt_comes_last_and_only_when_asked() {
        let mut seen = Vec::new();
        let result = nudged_or_tight(|n| {
            seen.push(n);
            if n.tight {
                Ok(seen.len())
            } else {
                Err(numerical("wall", true))
            }
        });
        assert_eq!(result, Ok(8));
        assert_eq!(
            seen[7],
            Nudge {
                tight: true,
                ..Nudge::NONE
            }
        );
        assert!(seen[..7].iter().all(|n| !n.tight));
        let mut calls = 0;
        let result: Result<(), _> = nudged(|_| {
            calls += 1;
            Err(numerical("verification", false))
        });
        assert_eq!(result, Err(error("verification")));
        assert_eq!(calls, 7);
        let result: Result<(), _> =
            nudged_or_tight(|n| Err(numerical(if n.tight { "tight" } else { "first" }, false)));
        assert_eq!(result, Err(error("first")));
    }

    #[test]
    fn the_flat_attempt_follows_the_tight_one() {
        // docs/specs/offset-retry.md §8: arcs as chords, the walls moved out by the arc
        // tolerance, after every other attempt failed.
        let mut seen = Vec::new();
        let result = nudged_or_tight(|n| {
            seen.push(n);
            if n.flat {
                Ok(seen.len())
            } else {
                Err(numerical("wall", true))
            }
        });
        assert_eq!(result, Ok(9));
        assert_eq!(
            seen[8],
            Nudge {
                wall: chipload_core::tolerances::ARC_APPROX_TOL_MM,
                flat: true,
                ..Nudge::NONE
            }
        );
        assert!(seen[..8].iter().all(|n| !n.flat));
    }
}
