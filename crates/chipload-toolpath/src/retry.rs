// SPDX-License-Identifier: GPL-3.0-or-later
//! Retrying an offset that hit a numerical coincidence with a negligibly different one
//! (docs/specs/offset-retry.md).

use chipload_core::tolerances::OFFSET_RETRY_NUDGE_MM;

use crate::ToolpathError;

/// Attempts after the first failed one.
const OFFSET_RETRIES: u32 = 3;

/// How far an attempt moves its offsets, in mm: both zero on the first attempt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Nudge {
    /// Taken off the stepover between rings.
    pub step: f64,
    /// Added to the offset from the walls.
    pub wall: f64,
}

impl Nudge {
    pub const NONE: Nudge = Nudge {
        step: 0.0,
        wall: 0.0,
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

/// The first attempt that succeeds; after a numerical failure up to three more, each nudged
/// by one more [`OFFSET_RETRY_NUDGE_MM`]. If none succeeds, the first attempt's error.
pub(crate) fn nudged<T>(
    mut attempt: impl FnMut(Nudge) -> Result<T, Failure>,
) -> Result<T, ToolpathError> {
    let (error, mut move_wall) = match attempt(Nudge::NONE) {
        Ok(value) => return Ok(value),
        Err(Failure::Final(error)) => return Err(error),
        Err(Failure::Numerical { error, wall }) => (error, wall),
    };
    for k in 1..=OFFSET_RETRIES {
        let by = f64::from(k) * OFFSET_RETRY_NUDGE_MM;
        let nudge = Nudge {
            step: by,
            wall: if move_wall { by } else { 0.0 },
        };
        match attempt(nudge) {
            Ok(value) => return Ok(value),
            Err(Failure::Final(ToolpathError::Cancelled)) => return Err(ToolpathError::Cancelled),
            Err(Failure::Final(_)) => break,
            Err(Failure::Numerical { wall, .. }) => move_wall |= wall,
        }
    }
    Err(error)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn numerical(text: &str, wall: bool) -> Failure {
        Failure::Numerical {
            error: ToolpathError::Geometry(text.into()),
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
        assert_eq!(result, Err(ToolpathError::Geometry("attempt 1".into())));
        let e = OFFSET_RETRY_NUDGE_MM;
        assert_eq!(
            seen,
            [
                Nudge::NONE,
                Nudge { step: e, wall: 0.0 },
                Nudge {
                    step: 2.0 * e,
                    wall: 2.0 * e
                },
                Nudge {
                    step: 3.0 * e,
                    wall: 3.0 * e
                },
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
        assert_eq!(seen[1], Nudge { step: e, wall: e });
    }

    #[test]
    fn errors_a_nudge_cannot_change_end_the_retries_with_the_first_error() {
        let mut calls = 0;
        let result: Result<(), _> = nudged(|_| {
            calls += 1;
            Err(Failure::Final(ToolpathError::Geometry(
                "does not fit".into(),
            )))
        });
        assert_eq!(result, Err(ToolpathError::Geometry("does not fit".into())));
        assert_eq!(calls, 1);
        let mut calls = 0;
        let result: Result<(), _> = nudged(|_| {
            calls += 1;
            if calls == 1 {
                Err(numerical("verification", false))
            } else {
                Err(Failure::Final(ToolpathError::Geometry(
                    "does not fit".into(),
                )))
            }
        });
        assert_eq!(result, Err(ToolpathError::Geometry("verification".into())));
        assert_eq!(calls, 2);
    }
}
