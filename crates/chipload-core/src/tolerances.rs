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
/// How much farther than the tool radius a verified profile offset may run: the offset
/// library's own accuracy (cavalier_contours `offset_dist_eps`). Only the far side: the tool
/// leaves at most this much material and never cuts into the part (user decision
/// 2026-10-11, after a valid outside profile was refused 1.6–24 µm too far).
pub const OFFSET_FAR_SLACK_MM: f64 = 1e-4;
/// How much a retried offset moves after a numerical failure (docs/specs/offset-retry.md).
pub const OFFSET_RETRY_NUDGE_MM: f64 = 1e-6;
/// The pocket shape offset's slice tolerance on its last retry (cavalier_contours
/// `offset_dist_eps`, default 1e-4), after a result that came too close to a wall
/// (docs/specs/offset-retry.md §6). Not the default: it loses whole areas elsewhere.
pub const OFFSET_TIGHT_SLICE_EPS_MM: f64 = 1e-6;
/// The wider nudge, after three of [`OFFSET_RETRY_NUDGE_MM`] have failed: an offset can fail
/// at one exact distance and the next few nanometres (docs/specs/offset-retry.md §7).
pub const OFFSET_RETRY_WIDE_NUDGE_MM: f64 = 1e-5;
/// Image tracing (docs/specs/image-trace.md): a traced edge crosses between two pixel
/// centres at least this fraction of the pixel spacing away from either, so a pixel exactly
/// at the threshold never puts two loops through one point.
pub const TRACE_EDGE_MARGIN: f64 = 1e-6;
/// Uncut pocket material thinner than this is not reported.
pub const REST_TOL_MM: f64 = 0.05;
/// Boundary points closer than ARC_APPROX_TOL_MM but farther apart than this along the
/// boundary mean the outline folds back on itself (docs/specs/folded-contours.md).
pub const FOLD_MIN_PATH_MM: f64 = 1.0;
/// A straight up or down move shorter than this is float noise, not a move: a ramp's end
/// and the depth it ramps to can differ in the last bits, and differently on each
/// platform's maths library (one move more or less in a plan).
pub const Z_NOISE_MM: f64 = 1e-9;
/// The floor of a V-carve's wide strokes counts as cleared where no more than this stands on
/// it (docs/specs/vcarve-wide-strokes.md §4.1, decision 6): without a height, no round end
/// mill ever clears a sharp corner, so the largest that clears it all would be 0.
pub const FLOOR_LEFT_MM: f64 = 0.2;
/// And what stands there counts only when it is at least this share of the region's floor:
/// the specks a round end mill leaves in sharp corners of the floor are less, and a note for
/// them would come with nearly every carving (2026-10-30, "not obtrusive").
pub const FLOOR_LEFT_SHARE: f64 = 0.01;
/// Imported geometry within this of the stored one counts as unchanged after an update: a
/// drawing moved several times may differ in the last bits (docs/specs/drawing-placement.md).
pub const PLACEMENT_ROUNDING_MM: f64 = 1e-9;
/// Grid the V-carve outline is snapped to for the Voronoi diagram, which needs integers
/// (docs/specs/v-carve.md §4).
pub const VORONOI_GRID_MM: f64 = 0.0001;
/// How far a V-carve may stray from the outline: no gouge beyond it, and every part of the
/// shape reached within it (docs/specs/v-carve.md §1).
pub const VCARVE_TOL_MM: f64 = ARC_APPROX_TOL_MM;
