// SPDX-License-Identifier: GPL-3.0-or-later
//! All tolerances live here. Do not write bare epsilons in algorithm code.
//!

/// Max gap when joining entities into a contour.
pub const CHAIN_TOL_MM: f64 = 0.001;
/// Max deviation when approximating arcs, splines and ellipses.
pub const ARC_APPROX_TOL_MM: f64 = 0.01;
pub const ANGLE_TOL_RAD: f64 = 1e-9;
/// Depth rounding slack when splitting into passes.
pub const DEPTH_TOL_MM: f64 = 0.001;
/// |Z| above this is reported as nonzero Z on import.
pub const Z_NONZERO_TOL_MM: f64 = 0.001;
/// Float slack when verifying offset distances.
pub const OFFSET_CHECK_SLACK_MM: f64 = 1e-6;
/// How much a retried offset moves after a numerical failure (docs/specs/offset-retry.md).
pub const OFFSET_RETRY_NUDGE_MM: f64 = 1e-6;
/// Uncut pocket material thinner than this is not reported.
pub const REST_TOL_MM: f64 = 0.05;
/// Boundary points closer than ARC_APPROX_TOL_MM but farther apart than this along the
/// boundary mean the outline folds back on itself (docs/specs/folded-contours.md).
pub const FOLD_MIN_PATH_MM: f64 = 1.0;
