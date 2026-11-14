// SPDX-License-Identifier: GPL-3.0-or-later
//! A picture read as a depth map (docs/specs/picture-relief.md §3): brightness as height,
//! on a grid of at most [`MAX_CELLS`] cells along the longer side, smoothed, its background
//! left out and its heights bent by a gamma.

use chipload_i18n::{Digits, Message, Value};

use crate::{Decoded, ImageError};

/// The most cells along a depth grid's longer side (spec §3).
pub const MAX_CELLS: usize = 800;

/// The smoothing radius's range, mm on the board.
pub const SMOOTHING_RANGE: (f64, f64) = (0.0, 5.0);
/// The background share's range.
pub const BACKGROUND_RANGE: (f64, f64) = (0.0, 0.95);
/// The gamma's range.
pub const GAMMA_RANGE: (f64, f64) = (0.2, 5.0);

/// Below this many cells of σ the smoothing is left out: it would change nothing seen.
const LEAST_SIGMA_CELLS: f64 = 0.25;
/// The Gaussian is cut off this many σ out.
const SIGMAS: f64 = 3.0;

/// How a picture is read as heights (spec §3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DepthSettings {
    /// White is high; false: black is.
    pub white_high: bool,
    /// The blur's radius on the board, mm.
    pub smoothing: f64,
    /// Shares up to this are background, not the model.
    pub background: f64,
    /// The heights raised to this power.
    pub gamma: f64,
}

impl DepthSettings {
    /// The settings' ranges: a refusal naming the first one out of it.
    pub fn check(&self) -> Result<(), ImageError> {
        let within = |v: f64, (lo, hi): (f64, f64)| v.is_finite() && (lo..=hi).contains(&v);
        let out = |field: &str, v: f64, (lo, hi): (f64, f64)| {
            ImageError::Setting(
                Message::new("image-error-depth-setting")
                    .arg("field", field.to_string())
                    .arg("low", Value::Number(lo, Digits::Shortest))
                    .arg("high", Value::Number(hi, Digits::Shortest))
                    .arg("got", Value::Number(v, Digits::Shortest)),
            )
        };
        if !within(self.smoothing, SMOOTHING_RANGE) {
            return Err(out("smoothing", self.smoothing, SMOOTHING_RANGE));
        }
        if !within(self.background, BACKGROUND_RANGE) {
            return Err(out("background", self.background, BACKGROUND_RANGE));
        }
        if !within(self.gamma, GAMMA_RANGE) {
            return Err(out("gamma", self.gamma, GAMMA_RANGE));
        }
        Ok(())
    }
}

/// A picture's heights on a grid: row 0 the lowest (the picture's bottom), each from the
/// left; None for background. Heights from 0 to 1, before any depth is given them.
#[derive(Debug, Clone, PartialEq)]
pub struct DepthGrid {
    pub columns: usize,
    pub rows: usize,
    /// A cell's side, mm, for a picture `width` mm wide.
    pub cell: f64,
    pub heights: Vec<Option<f64>>,
}

impl DepthGrid {
    /// The height of cell (`column`, `row`).
    pub fn at(&self, column: usize, row: usize) -> Option<f64> {
        self.heights[row * self.columns + column]
    }
}

/// `picture` as heights on the board, `width` mm wide (spec §3): averaged onto at most
/// [`MAX_CELLS`] cells along its longer side, turned round unless white is high, smoothed,
/// its background left out and the rest stretched to 0..1, then raised to the gamma.
/// Refused when every cell is background.
pub fn depth_grid(
    picture: &Decoded,
    settings: &DepthSettings,
    width: f64,
) -> Result<DepthGrid, ImageError> {
    settings.check()?;
    if !(width.is_finite() && width > 0.0) {
        return Err(ImageError::Width);
    }
    let (columns, rows, cell, greys) = averaged(picture, width);
    // Turned round as asked.
    let mut shares: Vec<f64> = greys
        .iter()
        .map(|g| {
            let v = g / 255.0;
            if settings.white_high { v } else { 1.0 - v }
        })
        .collect();
    shares = smoothed(shares, (columns, rows), cell, settings.smoothing);
    let b = settings.background;
    let heights: Vec<Option<f64>> = shares
        .iter()
        .map(|&v| {
            if b > 0.0 && v <= b {
                return None;
            }
            let stretched = ((v - b) / (1.0 - b)).clamp(0.0, 1.0);
            Some(chipload_core::math::pow(stretched, settings.gamma))
        })
        .collect();
    if heights.iter().all(Option::is_none) {
        return Err(ImageError::Setting(Message::new("image-error-depth-empty")));
    }
    Ok(DepthGrid {
        columns,
        rows,
        cell,
        heights,
    })
}

/// `picture` averaged onto the grid for a picture `width` mm wide: at most [`MAX_CELLS`]
/// cells along its longer side, whole pixels per cell so no cell is finer than a pixel.
/// Columns, rows, the cell's side in mm, and the grey values (0–255), row 0 the picture's
/// bottom.
pub(crate) fn averaged(picture: &Decoded, width: f64) -> (usize, usize, f64, Vec<f64>) {
    let (w, h) = (picture.width as usize, picture.height as usize);
    let factor = w.max(h).div_ceil(MAX_CELLS).max(1);
    let (columns, rows) = (w.div_ceil(factor), h.div_ceil(factor));
    let cell = width / columns as f64;
    let mut greys = vec![0.0; columns * rows];
    for row in 0..rows {
        let top = (rows - 1 - row) * factor;
        for column in 0..columns {
            let left = column * factor;
            let (mut sum, mut count) = (0.0, 0.0);
            for y in top..(top + factor).min(h) {
                for x in left..(left + factor).min(w) {
                    sum += picture.at(x, y);
                    count += 1.0;
                }
            }
            greys[row * columns + column] = sum / count;
        }
    }
    (columns, rows, cell, greys)
}

/// `values` on a grid of `cell` mm blurred by a Gaussian of `radius` mm (σ = radius / 2),
/// or as they are when that is less than a quarter of a cell.
pub(crate) fn smoothed(
    values: Vec<f64>,
    (columns, rows): (usize, usize),
    cell: f64,
    radius: f64,
) -> Vec<f64> {
    let sigma = radius / cell / 2.0;
    if sigma >= LEAST_SIGMA_CELLS {
        blurred(&values, columns, rows, sigma)
    } else {
        values
    }
}

/// `values` (`columns` × `rows`) blurred by a Gaussian of `sigma` cells, along the rows
/// then the columns, the edges repeated.
fn blurred(values: &[f64], columns: usize, rows: usize, sigma: f64) -> Vec<f64> {
    let reach = (SIGMAS * sigma).ceil() as isize;
    let weights: Vec<f64> = (-reach..=reach)
        .map(|k| chipload_core::math::exp(-((k * k) as f64) / (2.0 * sigma * sigma)))
        .collect();
    let total: f64 = weights.iter().sum();
    let pass =
        |source: &[f64], along: usize, across: usize, index: &dyn Fn(usize, usize) -> usize| {
            let mut out = vec![0.0; source.len()];
            for a in 0..across {
                for i in 0..along {
                    let mut sum = 0.0;
                    for (k, weight) in weights.iter().enumerate() {
                        let j =
                            (i as isize + k as isize - reach).clamp(0, along as isize - 1) as usize;
                        sum += weight * source[index(j, a)];
                    }
                    out[index(i, a)] = sum / total;
                }
            }
            out
        };
    let by_rows = pass(values, columns, rows, &|i, a| a * columns + i);
    pass(&by_rows, rows, columns, &|i, a| i * columns + a)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAIN: DepthSettings = DepthSettings {
        white_high: true,
        smoothing: 0.0,
        background: 0.0,
        gamma: 1.0,
    };

    /// A picture `w` × `h` with grey `f(column, row)` (row 0 at the top).
    fn picture(w: u32, h: u32, f: impl Fn(u32, u32) -> f64) -> Decoded {
        let grey = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .map(|(x, y)| f(x, y))
            .collect();
        Decoded {
            width: w,
            height: h,
            grey,
        }
    }

    #[test]
    fn a_grey_ramp_rises_with_its_brightness() {
        let ramp = picture(256, 4, |x, _| f64::from(x));
        let grid = depth_grid(&ramp, &PLAIN, 128.0).unwrap();
        assert_eq!((grid.columns, grid.rows), (256, 4));
        assert_eq!(grid.cell, 0.5);
        for x in [0, 64, 255] {
            let h = grid.at(x, 0).unwrap();
            assert!((h - x as f64 / 255.0).abs() < 1e-9, "{x}: {h}");
        }
        // Black high turns it round; a gamma of 2 bends it.
        let low = DepthSettings {
            white_high: false,
            ..PLAIN
        };
        assert!((depth_grid(&ramp, &low, 128.0).unwrap().at(0, 0).unwrap() - 1.0).abs() < 1e-9);
        let bent = DepthSettings {
            gamma: 2.0,
            ..PLAIN
        };
        let h = depth_grid(&ramp, &bent, 128.0).unwrap().at(128, 0).unwrap();
        assert!((h - (128.0f64 / 255.0).powi(2)).abs() < 1e-9, "{h}");
    }

    #[test]
    fn the_background_is_left_out_and_the_rest_stretched() {
        // A white square in the middle of black.
        let disc = picture(20, 20, |x, y| {
            if (5..15).contains(&x) && (5..15).contains(&y) {
                255.0
            } else {
                0.0
            }
        });
        let settings = DepthSettings {
            background: 0.1,
            ..PLAIN
        };
        let grid = depth_grid(&disc, &settings, 20.0).unwrap();
        assert_eq!(grid.at(0, 0), None);
        assert_eq!(grid.at(10, 10), Some(1.0));
        assert_eq!(grid.heights.iter().flatten().count(), 100);
        // All of it background: refused.
        let black = picture(4, 4, |_, _| 0.0);
        assert!(depth_grid(&black, &settings, 4.0).is_err());
    }

    #[test]
    fn smoothing_softens_a_step_over_about_its_radius() {
        let step = picture(100, 2, |x, _| if x < 50 { 0.0 } else { 255.0 });
        let sharp = depth_grid(&step, &PLAIN, 100.0).unwrap();
        assert_eq!((sharp.at(49, 0), sharp.at(50, 0)), (Some(0.0), Some(1.0)));
        let soft = DepthSettings {
            smoothing: 4.0,
            ..PLAIN
        };
        let grid = depth_grid(&step, &soft, 100.0).unwrap();
        let (a, b) = (grid.at(49, 0).unwrap(), grid.at(50, 0).unwrap());
        assert!(a > 0.2 && a < 0.5 && b > 0.5 && b < 0.8, "{a} {b}");
        // Far from the step, nothing changes.
        assert!(grid.at(10, 0).unwrap() < 1e-6 && grid.at(90, 0).unwrap() > 1.0 - 1e-6);
    }

    #[test]
    fn a_large_picture_is_averaged_onto_at_most_800_cells() {
        let big = picture(4000, 3000, |x, _| if x < 2000 { 0.0 } else { 255.0 });
        let grid = depth_grid(&big, &PLAIN, 400.0).unwrap();
        assert_eq!((grid.columns, grid.rows), (800, 600));
        assert!((grid.cell - 0.5).abs() < 1e-12);
    }

    #[test]
    fn settings_out_of_range_are_refused() {
        for bad in [
            DepthSettings {
                smoothing: -1.0,
                ..PLAIN
            },
            DepthSettings {
                background: 0.99,
                ..PLAIN
            },
            DepthSettings {
                gamma: 0.0,
                ..PLAIN
            },
            DepthSettings {
                gamma: f64::NAN,
                ..PLAIN
            },
        ] {
            assert!(bad.check().is_err(), "{bad:?}");
        }
    }
}
