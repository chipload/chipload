// SPDX-License-Identifier: GPL-3.0-or-later
//! A total order on floats, so sort keys can be tuples of them (nearest first, then lowest,
//! then leftmost) with the standard `min_by_key`, `max_by_key` and `sort_by_key`.

use std::cmp::Ordering;

/// An `f64` ordered by [`f64::total_cmp`]: every value, NaN included, has one place.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ordered(pub f64);

impl Eq for Ordered {}

impl PartialOrd for Ordered {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Ordered {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tuples_of_ordered_floats_sort_lexicographically() {
        let mut keys = [
            (Ordered(1.0), Ordered(2.0)),
            (Ordered(0.5), Ordered(9.0)),
            (Ordered(1.0), Ordered(-1.0)),
        ];
        keys.sort();
        assert_eq!(keys[0].0, Ordered(0.5));
        assert_eq!(keys[1], (Ordered(1.0), Ordered(-1.0)));
        assert!(Ordered(f64::NAN) > Ordered(f64::INFINITY));
    }
}
