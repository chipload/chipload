// SPDX-License-Identifier: GPL-3.0-or-later
//! Black and white (spec §2): the grey values blurred, and the level where they turn black.

use crate::{Decoded, Settings, Threshold};

/// The blurred picture as "darkness" around a white border one pixel wide, and the level
/// below which it is black. Inverting flips the darkness, so black is always below the
/// level and the border always above it.
pub(crate) struct Level {
    /// Samples per row and rows, border included.
    pub columns: usize,
    pub rows: usize,
    /// Row by row from the **bottom**: sample (i, j) is the pixel in column i − 1, row
    /// `height − j` from the top; the outermost samples are the border.
    values: Vec<f64>,
    /// Black below this.
    pub level: f64,
    /// The threshold used.
    pub threshold: u8,
    width: usize,
    height: usize,
}

/// White, for the border: above every level (levels lie between 0.5 and 254.5).
const BORDER: f64 = 255.0;

impl Level {
    pub(crate) fn new(picture: &Decoded, settings: &Settings) -> Level {
        let grey = blurred(picture, settings.blur);
        let threshold = match settings.threshold {
            Threshold::Auto => otsu(&grey),
            Threshold::Level(t) => t,
        };
        // Black: grey below the threshold. Between integer grey values the edge lies
        // half-way, so an unblurred picture traces through the middle between pixels.
        let below = f64::from(threshold) - 0.5;
        let (darkness, level): (Box<dyn Fn(f64) -> f64>, f64) = if settings.invert {
            (Box::new(|v| 255.0 - v), 255.0 - below)
        } else {
            (Box::new(|v| v), below)
        };
        let (width, height) = (picture.width as usize, picture.height as usize);
        let (columns, rows) = (width + 2, height + 2);
        let mut values = vec![BORDER; columns * rows];
        for j in 1..=height {
            let row = height - j;
            for i in 1..=width {
                values[j * columns + i] = darkness(grey[row * width + i - 1]);
            }
        }
        Level {
            columns,
            rows,
            values,
            level,
            threshold,
            width,
            height,
        }
    }

    /// The sample in column `i`, row `j` from the bottom, border included.
    pub(crate) fn at(&self, i: usize, j: usize) -> f64 {
        self.values[j * self.columns + i]
    }

    pub(crate) fn is_black(&self, i: usize, j: usize) -> bool {
        self.at(i, j) < self.level
    }

    /// Every pixel, row by row from the top: true is black.
    pub(crate) fn black(&self) -> Vec<bool> {
        let mut out = Vec::with_capacity(self.width * self.height);
        for row in 0..self.height {
            let j = self.height - row;
            for i in 1..=self.width {
                out.push(self.is_black(i, j));
            }
        }
        out
    }

    /// A black pixel in the outermost row or column.
    pub(crate) fn black_at_border(&self) -> bool {
        let (w, h) = (self.width, self.height);
        (1..=w).any(|i| self.is_black(i, 1) || self.is_black(i, h))
            || (1..=h).any(|j| self.is_black(1, j) || self.is_black(w, j))
    }
}

/// The grey values blurred with a Gaussian of standard deviation `sigma` pixels (0: as
/// they are), one direction at a time; at the picture's edge the edge pixel repeats.
fn blurred(picture: &Decoded, sigma: f64) -> Vec<f64> {
    let kernel = gaussian(sigma);
    if kernel.len() == 1 {
        return picture.grey.clone();
    }
    let (w, h) = (picture.width as usize, picture.height as usize);
    let r = kernel.len() / 2;
    let clamp = |k: usize, d: usize, n: usize| (k + d).saturating_sub(r).min(n - 1);
    let mut across = vec![0.0; w * h];
    for row in 0..h {
        for column in 0..w {
            across[row * w + column] = kernel
                .iter()
                .enumerate()
                .map(|(d, k)| k * picture.at(clamp(column, d, w), row))
                .sum();
        }
    }
    let mut down = vec![0.0; w * h];
    for row in 0..h {
        for column in 0..w {
            down[row * w + column] = kernel
                .iter()
                .enumerate()
                .map(|(d, k)| k * across[clamp(row, d, h) * w + column])
                .sum();
        }
    }
    down
}

/// Weights of a Gaussian of standard deviation `sigma`, out to three of them, summing to 1.
pub(crate) fn gaussian(sigma: f64) -> Vec<f64> {
    if sigma <= 0.0 {
        return vec![1.0];
    }
    let r = (3.0 * sigma).ceil() as i64;
    let weights: Vec<f64> = (-r..=r)
        .map(|x| (-((x * x) as f64) / (2.0 * sigma * sigma)).exp())
        .collect();
    let sum: f64 = weights.iter().sum();
    weights.into_iter().map(|w| w / sum).collect()
}

/// Otsu's threshold for grey values 0–255: the one that best separates them into two
/// groups (the largest variance between the groups). Black is below it; 1 to 254.
pub fn otsu(grey: &[f64]) -> u8 {
    let mut histogram = [0u64; 256];
    for &v in grey {
        histogram[v.round().clamp(0.0, 255.0) as usize] += 1;
    }
    let total = grey.len() as f64;
    let sum: f64 = (0..256).map(|k| k as f64 * histogram[k] as f64).sum();
    let (mut weight_dark, mut sum_dark) = (0.0, 0.0);
    let (mut best, mut best_variance) = (127usize, -1.0);
    for (k, &count) in histogram.iter().enumerate().take(255) {
        weight_dark += count as f64;
        sum_dark += k as f64 * count as f64;
        let weight_light = total - weight_dark;
        if weight_dark == 0.0 || weight_light == 0.0 {
            continue;
        }
        let mean_dark = sum_dark / weight_dark;
        let mean_light = (sum - sum_dark) / weight_light;
        let variance = weight_dark * weight_light * (mean_dark - mean_light).powi(2);
        if variance > best_variance {
            best_variance = variance;
            best = k;
        }
    }
    // Values up to `best` are dark: black is below best + 1.
    (best + 1).clamp(1, 254) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_blur_weights_sum_to_one_and_are_symmetric() {
        for sigma in [0.3, 1.0, 2.5, 10.0] {
            let k = gaussian(sigma);
            assert!((k.iter().sum::<f64>() - 1.0).abs() < 1e-12);
            assert_eq!(k.len() % 2, 1);
            for i in 0..k.len() / 2 {
                assert_eq!(k[i], k[k.len() - 1 - i]);
            }
        }
        assert_eq!(gaussian(0.0), vec![1.0]);
    }

    #[test]
    fn otsu_separates_two_levels() {
        let mut grey = vec![40.0; 300];
        grey.extend(vec![200.0; 700]);
        let t = otsu(&grey);
        assert!(t > 40 && t <= 200, "{t}");
    }

    #[test]
    fn otsu_of_one_level_stays_in_range() {
        assert!((1..=254).contains(&otsu(&[0.0; 10])));
        assert!((1..=254).contains(&otsu(&[255.0; 10])));
    }
}
