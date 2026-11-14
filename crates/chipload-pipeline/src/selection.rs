// SPDX-License-Identifier: GPL-3.0-or-later
//! What a selection of contours is good for, and selecting by kind
//! (docs/specs/fewer-clicks.md §2, §3): which new operations it can take, for the window's
//! Q menu; the contours similar to one, or inside one.

use std::collections::BTreeSet;

use chipload_core::chain::Contour;
use chipload_core::nesting::{Ring, contains};
use chipload_core::tolerances::CHAIN_TOL_MM;
use chipload_toolpath::holes::circle_of;

use crate::new_ops::OpKind;

/// Why a selection can't take an operation of some kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unfit {
    /// Nothing selected.
    Nothing,
    /// Holes drill circles, and none is selected.
    NoCircles,
    /// Pockets and V-carves fill closed shapes, and none is selected.
    NoClosed,
}

/// Whether the contours `ids` can take a new operation of `kind`: None when they can, else
/// why not. Construction geometry never counts, as it is never cut. Kinds that don't work
/// on a selection (surface, relief) are not judged here.
pub fn unfit_for(kind: OpKind, ids: &BTreeSet<usize>, contours: &[Contour]) -> Option<Unfit> {
    let chosen: Vec<&Contour> = contours
        .iter()
        .filter(|c| ids.contains(&c.id) && !c.is_construction())
        .collect();
    if chosen.is_empty() {
        return Some(Unfit::Nothing);
    }
    let closed = || {
        chosen
            .iter()
            .any(|c| c.closed && c.invalid_reason.is_none())
    };
    match kind {
        OpKind::Holes if !chosen.iter().any(|c| circle_of(c).is_some()) => Some(Unfit::NoCircles),
        OpKind::Pocket | OpKind::Vcarve if !closed() => Some(Unfit::NoClosed),
        _ => None,
    }
}

/// The box round a contour: (width, height).
fn size_of(contour: &Contour) -> (f64, f64) {
    let mut low = (f64::INFINITY, f64::INFINITY);
    let mut high = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for part in &contour.parts {
        let (a, b) = part.segment.prim.bounds();
        low = (low.0.min(a.0), low.1.min(a.1));
        high = (high.0.max(b.0), high.1.max(b.1));
    }
    (high.0 - low.0, high.1 - low.1)
}

fn same(a: f64, b: f64) -> bool {
    (a - b).abs() <= CHAIN_TOL_MM
}

/// *Select similar* (docs/specs/fewer-clicks.md §3): the contours like contour `of`, itself
/// included. A circle gives every circle of the same diameter on any layer; anything else
/// the contours of the same kind (closed or open, not circles) on the same layers whose
/// boxes are the same size either way round, all within the chaining tolerance.
/// Construction geometry never counts. Empty when there is no contour `of`.
pub fn similar(contours: &[Contour], of: usize) -> BTreeSet<usize> {
    let Some(this) = contours.iter().find(|c| c.id == of) else {
        return BTreeSet::new();
    };
    let candidates = contours.iter().filter(|c| !c.is_construction());
    if let Some(circle) = circle_of(this) {
        return candidates
            .filter(|c| circle_of(c).is_some_and(|other| same(other.radius, circle.radius)))
            .map(|c| c.id)
            .chain([of])
            .collect();
    }
    let layers = this.layers();
    let (w, h) = size_of(this);
    candidates
        .filter(|c| c.closed == this.closed && circle_of(c).is_none() && c.layers() == layers)
        .filter(|c| {
            let (cw, ch) = size_of(c);
            (same(cw, w) && same(ch, h)) || (same(cw, h) && same(ch, w))
        })
        .map(|c| c.id)
        .chain([of])
        .collect()
}

/// *Select inside* (docs/specs/fewer-clicks.md §3): every valid closed contour inside the
/// closed contour `of`, nested ones too, without `of` itself. Open contours and
/// construction geometry are left out. None when `of` is not a valid closed contour.
pub fn inside(contours: &[Contour], of: usize) -> Option<BTreeSet<usize>> {
    let ring = |c: &Contour| Ring::from_points(c.points());
    let this = contours
        .iter()
        .find(|c| c.id == of && c.closed && c.invalid_reason.is_none())?;
    let outer = ring(this);
    Some(
        contours
            .iter()
            .filter(|c| {
                c.id != of && c.closed && c.invalid_reason.is_none() && !c.is_construction()
            })
            .filter(|c| contains(&outer, &ring(c)))
            .map(|c| c.id)
            .collect(),
    )
}

/// The project's shape ids that contours `ids` are made of (`select.shapes`).
pub fn shapes_of(contours: &[Contour], ids: &BTreeSet<usize>) -> BTreeSet<u64> {
    contours
        .iter()
        .filter(|c| ids.contains(&c.id))
        .flat_map(|c| c.parts.iter().filter_map(|p| p.segment.shape))
        .collect()
}

/// The contour shape `shape` is part of, if any.
pub fn contour_of_shape(contours: &[Contour], shape: u64) -> Option<usize> {
    contours
        .iter()
        .find(|c| c.parts.iter().any(|p| p.segment.shape == Some(shape)))
        .map(|c| c.id)
}
