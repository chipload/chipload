// SPDX-License-Identifier: GPL-3.0-or-later
//! The search (docs/specs/nesting-search.md §4): a local search over the order the items
//! are placed in and the lie of each, every candidate placed by the placer (`decode`).
//!
//! Searching the sequence and decoding it by a bottom-left rule is from Jakobs, *On genetic
//! algorithms for the packing of polygons*, EJOR 88 (1996), and Burke, Hellier, Kendall &
//! Whitwell, *A new bottom-left-fill heuristic algorithm for the two-dimensional irregular
//! packing problem*, Operations Research 54 (2006). Swapping two items is the 2-exchange
//! of Gomes & Oliveira, *A 2-exchange heuristic for nesting problems*, EJOR 141 (2002).
//! The acceptance rule is late acceptance hill climbing: Burke & Bykov, *The late
//! acceptance hill-climbing heuristic*, EJOR 258 (2017). The random numbers are SplitMix64:
//! Steele, Lea & Flood, *Fast splittable pseudorandom number generators*, OOPSLA 2014.

use super::fill::{Candidate, Decoded, Goal, Item, Key, Sheet, decode, starts};
use crate::{Cancel, ToolpathError};

/// The layouts the search places for up to [`BUDGET_ITEMS`] items.
const BUDGET: usize = 600;

/// Beyond this many items, the search places fewer layouts, each costing more: by the
/// square of the share, as a layout's cost grows about so (more items, each on a fuller
/// board).
const BUDGET_ITEMS: usize = 20;

/// The fewest layouts the search places, however many items.
const BUDGET_LEAST: usize = 100;

/// How many layouts the search places after its starts, for `items` items: measured so
/// that 20 instances of mixed parts take about a second and 50 on a full sheet stay well
/// under 10 s in a release build (§4, as built). A count, never a time: the same input
/// gives the same layout on any computer.
pub(super) fn budget(items: usize) -> usize {
    if items <= BUDGET_ITEMS {
        BUDGET
    } else {
        (BUDGET * BUDGET_ITEMS * BUDGET_ITEMS / (items * items)).max(BUDGET_LEAST)
    }
}

/// The late acceptance list's length: a candidate is taken when it is no worse than the
/// current one, or than the current one was this many steps ago.
const LATE: usize = 30;

/// The seed of the search's random numbers: fixed, so the search is the same every time.
const SEED: u64 = 0x6368_6970_6c6f_6164;

/// How often a move that would change nothing is drawn again before the step is skipped.
const REDRAWS: usize = 8;

/// SplitMix64 (Steele, Lea & Flood 2014): small, fast, and the same on every platform.
struct Random(u64);

impl Random {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A whole number below `n` (`n` > 0); the bias of the remainder is far below what
    /// matters here.
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

/// The best layout for `items` on `sheet`: the starts placed (§4.1), `extra` among them
/// (the board as it is, §5), then `budget` more candidates, each one move from the current
/// one (§4.2), taken by late acceptance (§4.3), the random moves from [`SEED`] mixed with
/// `seed`. None when there are no starts (no items). Progress is reported on `cancel`.
pub(super) fn search(
    sheet: &Sheet,
    items: &[Item],
    goal: Goal,
    budget: usize,
    extra: Option<Candidate>,
    seed: u64,
    cancel: &Cancel,
) -> Result<Option<Decoded>, ToolpathError> {
    let mut starts = starts(items);
    starts.extend(extra);
    let total = (starts.len() + budget).max(1) as f64;
    let mut best: Option<(Candidate, Decoded)> = None;
    for (k, candidate) in starts.into_iter().enumerate() {
        let decoded = decode(sheet, items, &candidate, goal, cancel)?;
        cancel.stage_done(k as f64 / total);
        if best.as_ref().is_none_or(|(_, b)| decoded.score < b.score) {
            best = Some((candidate, decoded));
        }
    }
    let Some((candidate, decoded)) = best else {
        return Ok(None);
    };
    let kinds = kinds(items);
    let mut current = with_turns(candidate, &decoded);
    let mut current_score = decoded.score;
    let mut best = decoded;
    let mut late: Vec<Key> = vec![current_score; LATE];
    let mut random = Random(SEED ^ seed);
    let done = total - budget as f64;
    for step in 0..budget {
        cancel.check()?;
        cancel.stage_done((done + step as f64) / total);
        let Some(next) = neighbour(&current, items, &kinds, &mut random) else {
            continue;
        };
        let decoded = decode(sheet, items, &next, goal, cancel)?;
        let v = step % LATE;
        let late_score = late.get(v).copied().unwrap_or(current_score);
        if decoded.score <= current_score || decoded.score <= late_score {
            current = with_turns(next, &decoded);
            current_score = decoded.score;
        }
        if let Some(slot) = late.get_mut(v) {
            *slot = current_score;
        }
        if decoded.score < best.score {
            best = decoded;
        }
    }
    cancel.stage_done(1.0);
    Ok(Some(best))
}

/// `candidate` with the lies its layout took: what the placer chose becomes the turn the
/// search moves from.
fn with_turns(mut candidate: Candidate, decoded: &Decoded) -> Candidate {
    candidate.turns.clone_from(&decoded.turns);
    candidate
}

/// Each item's kind: the first item with the same lies, so that swapping two of one kind
/// in the same lie is known to change nothing.
fn kinds(items: &[Item]) -> Vec<usize> {
    let same = |a: &Item, b: &Item| {
        a.lies.len() == b.lies.len()
            && a.lies
                .iter()
                .zip(&b.lies)
                .all(|(x, y)| x.angle == y.angle && x.footprint == y.footprint)
    };
    (0..items.len())
        .map(|k| (0..k).find(|&j| same(&items[j], &items[k])).unwrap_or(k))
        .collect()
}

/// One move from `current` (§4.2): two items swapped in the order, one moved to another
/// place in it, or one given another of its lies; drawn again when it would change
/// nothing, None after [`REDRAWS`] tries.
fn neighbour(
    current: &Candidate,
    items: &[Item],
    kinds: &[usize],
    random: &mut Random,
) -> Option<Candidate> {
    let n = current.order.len();
    let turnable: Vec<usize> = (0..items.len())
        .filter(|&k| items.get(k).is_some_and(|i| i.lies.len() > 1))
        .collect();
    for _ in 0..REDRAWS {
        let mut next = current.clone();
        let changed = match random.below(3) {
            0 if n >= 2 => {
                let (i, j) = (random.below(n), random.below(n));
                let (a, b) = (next.order[i], next.order[j]);
                let same = kinds.get(a) == kinds.get(b) && next.turns.get(a) == next.turns.get(b);
                if i != j && !same {
                    next.order.swap(i, j);
                    true
                } else {
                    false
                }
            }
            1 if n >= 2 => {
                let (i, j) = (random.below(n), random.below(n));
                if i != j {
                    let k = next.order.remove(i);
                    next.order.insert(j, k);
                    true
                } else {
                    false
                }
            }
            2 if !turnable.is_empty() => {
                let k = turnable[random.below(turnable.len())];
                let lies = items.get(k).map_or(1, |i| i.lies.len());
                let now = next.turns.get(k).copied().flatten().unwrap_or(0);
                // Another lie: one of the others, chosen evenly.
                let other = (now + 1 + random.below(lies - 1)) % lies;
                if let Some(t) = next.turns.get_mut(k) {
                    *t = Some(other);
                }
                true
            }
            _ => false,
        };
        if changed && next != *current {
            return Some(next);
        }
    }
    None
}
