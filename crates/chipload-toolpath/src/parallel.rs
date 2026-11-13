// SPDX-License-Identifier: GPL-3.0-or-later
//! Work spread over the processor's cores (docs/specs/stl-relief.md §15): items worked
//! independently, their results in the items' order, so the outcome is the same as one
//! after the other, whatever the number of threads. Only for pure work: `f` must give the
//! same result for an item whichever thread runs it.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

/// Items per piece of work handed to a thread: small enough that threads finish together
/// when some items cost far more than others, large enough that handing out costs nothing.
const PIECES_PER_THREAD: usize = 8;

/// `f` over `items`, in order, on as many threads as the processor has. One after the other
/// where threads can't be had (one core, the browser) or one fails to start.
pub fn map_ordered<T, R, F>(items: &[T], f: F) -> Vec<R>
where
    T: Sync,
    R: Send,
    F: Fn(&T) -> R + Sync,
{
    let threads = thread::available_parallelism().map_or(1, |n| n.get());
    if threads < 2 || items.len() < 2 {
        return items.iter().map(&f).collect();
    }
    let piece = items.len().div_ceil(threads * PIECES_PER_THREAD).max(1);
    let pieces: Vec<&[T]> = items.chunks(piece).collect();
    let next = AtomicUsize::new(0);
    let work = || {
        let mut done: Vec<(usize, Vec<R>)> = Vec::new();
        loop {
            let k = next.fetch_add(1, Ordering::Relaxed);
            let Some(items) = pieces.get(k) else {
                return done;
            };
            done.push((k, items.iter().map(&f).collect()));
        }
    };
    let mut done: Vec<(usize, Vec<R>)> = thread::scope(|s| {
        let helpers: Vec<_> = (1..threads)
            .filter_map(|_| thread::Builder::new().spawn_scoped(s, work).ok())
            .collect();
        // This thread works too; the helpers' pieces follow.
        let mut done = work();
        for helper in helpers {
            if let Ok(theirs) = helper.join() {
                done.extend(theirs);
            }
        }
        done
    });
    // A piece whose thread failed is worked here, so none is ever missing.
    done.sort_by_key(|(k, _)| *k);
    let mut out = Vec::with_capacity(items.len());
    let mut have = done.into_iter().peekable();
    for (k, items) in pieces.iter().enumerate() {
        match have.next_if(|(j, _)| *j == k) {
            Some((_, results)) => out.extend(results),
            None => out.extend(items.iter().map(&f)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_order_of_the_items() {
        let items: Vec<u64> = (0..10_000).collect();
        let squares = map_ordered(&items, |x| x * x);
        let expected: Vec<u64> = items.iter().map(|x| x * x).collect();
        assert_eq!(squares, expected);
    }

    #[test]
    fn handles_none_and_one() {
        assert!(map_ordered(&[] as &[u8], |x| *x).is_empty());
        assert_eq!(map_ordered(&[7u8], |x| *x + 1), vec![8]);
    }

    #[test]
    fn uneven_work_gives_the_same_result() {
        let items: Vec<u64> = (0..300).collect();
        let slow = |x: &u64| (0..(*x % 7) * 10_000).fold(*x, |a, b| a.wrapping_mul(31) ^ b);
        let expected: Vec<u64> = items.iter().map(slow).collect();
        assert_eq!(map_ordered(&items, slow), expected);
    }
}
