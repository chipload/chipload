// SPDX-License-Identifier: GPL-3.0-or-later
//! A grid of cells, each occupied or free, a bit each, rows of 64-bit words: what
//! footprints and boards are rasterised onto (docs/specs/nesting.md §4).

/// `cols` × `rows` cells, column 0 and row 0 at the low X and Y corner. The bits past the
/// last column of each row are always clear.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grid {
    cols: usize,
    rows: usize,
    /// Words per row.
    words: usize,
    bits: Vec<u64>,
}

impl Grid {
    /// All cells free.
    pub fn new(cols: usize, rows: usize) -> Grid {
        let words = cols.div_ceil(64);
        Grid {
            cols,
            rows,
            words,
            bits: vec![0; words * rows],
        }
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Whether cell (`col`, `row`) is occupied; outside the grid it isn't.
    pub fn get(&self, col: usize, row: usize) -> bool {
        if col >= self.cols || row >= self.rows {
            return false;
        }
        let word = self.bits.get(row * self.words + col / 64).copied();
        word.is_some_and(|w| w >> (col % 64) & 1 == 1)
    }

    /// Cell (`col`, `row`) occupied; outside the grid nothing happens.
    pub fn set(&mut self, col: usize, row: usize) {
        if col >= self.cols || row >= self.rows {
            return;
        }
        if let Some(w) = self.bits.get_mut(row * self.words + col / 64) {
            *w |= 1 << (col % 64);
        }
    }

    /// How many cells are occupied.
    pub fn count(&self) -> usize {
        self.bits.iter().map(|w| w.count_ones() as usize).sum()
    }

    /// Whether `other`, its cell (0, 0) on this grid's cell (`col`, `row`), has an occupied
    /// cell on an occupied cell of this grid. Cells of `other` off this grid meet nothing.
    pub fn overlaps_at(&self, other: &Grid, col: i64, row: i64) -> bool {
        let mut hit = false;
        self.each_shifted(other, col, row, |grid, index, word| {
            hit = hit || grid.bits.get(index).is_some_and(|w| w & word != 0);
        });
        hit
    }

    /// `other`'s occupied cells, its cell (0, 0) on this grid's cell (`col`, `row`), made
    /// occupied here. Cells off this grid are dropped.
    pub fn or_at(&mut self, other: &Grid, col: i64, row: i64) {
        let mut writes = Vec::new();
        self.each_shifted(other, col, row, |_, index, word| writes.push((index, word)));
        for (index, word) in writes {
            if let Some(w) = self.bits.get_mut(index) {
                *w |= word;
            }
        }
    }

    /// For each word of `other` shifted to (`col`, `row`): the index of this grid's word it
    /// falls on and its bits there, masked to this grid's columns.
    fn each_shifted(
        &self,
        other: &Grid,
        col: i64,
        row: i64,
        mut each: impl FnMut(&Grid, usize, u64),
    ) {
        let (base, shift) = (col.div_euclid(64), col.rem_euclid(64) as u32);
        let words = self.words as i64;
        for r in 0..other.rows {
            let target = row + r as i64;
            if target < 0 || target >= self.rows as i64 {
                continue;
            }
            let row_start = target as usize * self.words;
            for w in 0..other.words {
                let Some(&word) = other.bits.get(r * other.words + w) else {
                    continue;
                };
                if word == 0 {
                    continue;
                }
                let at = base + w as i64;
                let parts = [
                    (at, word << shift),
                    (at + 1, if shift == 0 { 0 } else { word >> (64 - shift) }),
                ];
                for (index, bits) in parts {
                    if bits == 0 || index < 0 || index >= words {
                        continue;
                    }
                    let bits = bits & self.column_mask(index as usize);
                    if bits != 0 {
                        each(self, row_start + index as usize, bits);
                    }
                }
            }
        }
    }

    /// The bits of word `index` of a row that are columns of the grid.
    fn column_mask(&self, index: usize) -> u64 {
        let used = self.cols.saturating_sub(index * 64);
        if used >= 64 {
            u64::MAX
        } else {
            (1u64 << used) - 1
        }
    }
}
