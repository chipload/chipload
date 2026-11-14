// SPDX-License-Identifier: GPL-3.0-or-later
//! What a selection of contours is good for (docs/specs/fewer-clicks.md §2): which new
//! operations it can take, for the window's Q menu.

use std::collections::BTreeSet;

use chipload_core::chain::Contour;
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
