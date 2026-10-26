// SPDX-License-Identifier: GPL-3.0-or-later
//! Tabs placed by hand (docs/specs/tabs-by-hand.md): a profile's tabs with their centres,
//! and tabs added, removed or spaced evenly again.

use chipload_core::Point;
use chipload_core::primitives::distance;
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{Job, Operation, ProfileOp};
use chipload_toolpath::plan::TabCentre;
use chipload_toolpath::plan_job;
use chipload_toolpath::tabs::arc_position;

use crate::{PipelineError, load_drawing, no_operation};

/// One tab of an operation: its ring, its centre on it, and whether it was placed by hand.
pub type TabRow = TabCentre;

/// A tool-centre ring.
pub type Ring = Vec<Point>;

/// Every operation's rings, each with its operation's index, in ring-start order.
type Rings = Vec<(usize, Ring)>;

/// Every tab of profile `index` (0-based), in planning order, as the job is planned now.
pub fn tabs_of(job: &Job, index: usize) -> Result<Vec<TabRow>, PipelineError> {
    profile(job, index)?;
    Ok(planned(job)?
        .0
        .into_iter()
        .filter(|t| t.operation == index)
        .collect())
}

/// The job planned: its tab centres and every ring of the operations, in ring-start order.
fn planned(job: &Job) -> Result<(Vec<TabCentre>, Rings), PipelineError> {
    let drawing = load_drawing(job)?;
    let plan = plan_job(job, &drawing.contours).map_err(PipelineError::Toolpath)?;
    let rings = plan
        .ring_starts
        .into_iter()
        .map(|r| (r.operation, r.ring))
        .collect();
    Ok((plan.tab_centres, rings))
}

/// Add a tab to profile `index` at `at` (moved onto its nearest ring). On a ring whose tabs
/// Chipload spaced, they are first kept where they are, so only the new one is added
/// (docs/specs/tabs-by-hand.md decision 3).
pub fn add_tab(job: &mut Job, index: usize, at: [f64; 2]) -> Result<(), PipelineError> {
    let (tabs, rings) = planned_for(job, index)?;
    let ring = nearest_ring(&rings, at, job, index)?;
    with_tab_added(tab_points(job, index)?, &tabs, ring, at);
    Ok(())
}

/// Remove profile `index`'s tab nearest to `at` (within a tool diameter). On a ring whose
/// tabs Chipload spaced, the others are kept where they are. Removing a ring's last tab
/// placed by hand spaces it evenly again.
pub fn remove_tab(job: &mut Job, index: usize, at: [f64; 2]) -> Result<(), PipelineError> {
    let (tabs, _) = planned_for(job, index)?;
    let reach = tool_diameter(job, index);
    let p = (at[0], at[1]);
    let tab = tabs
        .iter()
        .filter(|t| distance(t.at, p) <= reach)
        .min_by(|a, b| distance(a.at, p).total_cmp(&distance(b.at, p)))
        .copied()
        .ok_or_else(|| no_tab(index, at))?;
    with_tab_removed(tab_points(job, index)?, &tabs, &tab);
    Ok(())
}

/// Space the tabs of profile `index`'s ring nearest to `at` evenly again: its points placed
/// by hand go. Returns how many.
pub fn even_ring(job: &mut Job, index: usize, at: [f64; 2]) -> Result<usize, PipelineError> {
    let (_, rings) = planned_for(job, index)?;
    let ring = nearest_ring(&rings, at, job, index)?;
    Ok(with_ring_even(tab_points(job, index)?, &rings, ring))
}

// The changes themselves, on a profile's tab points (`tabs.at`), given its planned tabs
// (`tabs`, of this operation) and rings: the app uses these with the last generation's
// tabs.

/// A tab added at `at` on ring `ring`, its other tabs kept where they are if Chipload had
/// spaced them.
pub fn with_tab_added(points: &mut Vec<[f64; 2]>, tabs: &[TabCentre], ring: usize, at: [f64; 2]) {
    points.extend(pin_ring(tabs, ring));
    points.push(at);
}

/// Tab `tab` moved to `to`, its ring's other tabs kept where they are.
pub fn with_tab_moved(
    points: &mut Vec<[f64; 2]>,
    tabs: &[TabCentre],
    tab: &TabCentre,
    to: [f64; 2],
) {
    points.extend(pin_ring(tabs, tab.ring));
    if let Some(k) = stored_point(points, tab.at) {
        points[k] = to;
    }
}

/// Tab `tab` removed, its ring's other tabs kept where they are.
pub fn with_tab_removed(points: &mut Vec<[f64; 2]>, tabs: &[TabCentre], tab: &TabCentre) {
    points.extend(pin_ring(tabs, tab.ring));
    if let Some(k) = stored_point(points, tab.at) {
        points.remove(k);
    }
}

/// Ring `ring` (of `rings`, the operation's) spaced evenly again: the points nearest to it
/// go. Returns how many.
pub fn with_ring_even(points: &mut Vec<[f64; 2]>, rings: &[Ring], ring: usize) -> usize {
    let before = points.len();
    points.retain(|p| nearest(rings, *p).map(|(k, _)| k) != Some(ring));
    before - points.len()
}

/// The stored point nearest a tab's centre: the point that placed it.
fn stored_point(points: &[[f64; 2]], centre: Point) -> Option<usize> {
    (0..points.len()).min_by(|&a, &b| {
        let d = |k: usize| distance((points[k][0], points[k][1]), centre);
        d(a).total_cmp(&d(b))
    })
}

/// Space every tab of profile `index` evenly again; returns how many points were placed by
/// hand.
pub fn reset_tabs(job: &mut Job, index: usize) -> Result<usize, PipelineError> {
    let points = tab_points(job, index)?;
    let count = points.len();
    points.clear();
    Ok(count)
}

/// The tabs and rings of profile `index`.
fn planned_for(job: &Job, index: usize) -> Result<(Vec<TabCentre>, Vec<Ring>), PipelineError> {
    profile(job, index)?;
    let (tabs, rings) = planned(job)?;
    let tabs = tabs.into_iter().filter(|t| t.operation == index).collect();
    let rings = rings
        .into_iter()
        .filter(|(op, _)| *op == index)
        .map(|(_, ring)| ring)
        .collect();
    Ok((tabs, rings))
}

/// The centres of ring `ring`'s tabs that Chipload spaced, to keep them where they are; none
/// when it has tabs placed by hand already.
fn pin_ring(tabs: &[TabCentre], ring: usize) -> Vec<[f64; 2]> {
    let on_ring: Vec<&TabCentre> = tabs.iter().filter(|t| t.ring == ring).collect();
    if on_ring.iter().any(|t| t.by_hand) {
        return Vec::new();
    }
    on_ring.iter().map(|t| [t.at.0, t.at.1]).collect()
}

/// The ring nearest to `p`, and how far.
fn nearest(rings: &[Ring], p: [f64; 2]) -> Option<(usize, f64)> {
    rings
        .iter()
        .enumerate()
        .filter_map(|(k, r)| arc_position(r, (p[0], p[1])).map(|(_, d)| (k, d)))
        .min_by(|a, b| a.1.total_cmp(&b.1))
}

/// The ring nearest to `at`, within a tool diameter, as planning assigns tabs.
fn nearest_ring(
    rings: &[Ring],
    at: [f64; 2],
    job: &Job,
    index: usize,
) -> Result<usize, PipelineError> {
    match nearest(rings, at) {
        Some((k, d)) if d <= tool_diameter(job, index) => Ok(k),
        _ => Err(PipelineError::Job(
            Message::new("pipeline-tab-no-ring")
                .arg("number", index + 1)
                .arg("at", Value::Point(at[0], at[1], Digits::Fixed(3))),
        )),
    }
}

fn tool_diameter(job: &Job, index: usize) -> f64 {
    job.operations
        .get(index)
        .and_then(|op| job.tool(op.tool()))
        .map_or(0.0, |t| t.diameter)
}

fn no_tab(index: usize, at: [f64; 2]) -> PipelineError {
    PipelineError::Job(
        Message::new("pipeline-tab-none-there")
            .arg("number", index + 1)
            .arg("at", Value::Point(at[0], at[1], Digits::Fixed(3))),
    )
}

/// Profile `index`'s tabs placed by hand.
fn tab_points(job: &mut Job, index: usize) -> Result<&mut Vec<[f64; 2]>, PipelineError> {
    profile(job, index)?;
    job.operations
        .get_mut(index)
        .and_then(Operation::tab_points_mut)
        .ok_or_else(|| no_tabs(index))
}

/// Operation `index`, if it is a profile with tabs.
fn profile(job: &Job, index: usize) -> Result<&ProfileOp, PipelineError> {
    match job.operations.get(index) {
        None => Err(no_operation(index + 1, job.operations.len())),
        Some(Operation::Profile(p)) if p.tabs.is_some() => Ok(p),
        Some(_) => Err(no_tabs(index)),
    }
}

fn no_tabs(index: usize) -> PipelineError {
    PipelineError::Job(Message::new("pipeline-tab-no-tabs").arg("number", index + 1))
}
