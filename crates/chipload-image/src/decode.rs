// SPDX-License-Identifier: GPL-3.0-or-later
//! Reading the picture (spec §4, §8): PNG, JPEG or BMP, within the size limits, checked from
//! the header before decoding, as grey values with transparency blended over white.

use std::io::Cursor;
use std::panic::{AssertUnwindSafe, catch_unwind};

use image::{ImageFormat, ImageReader, Limits};

use crate::ImageError;

/// The largest picture file, in bytes.
pub const MAX_BYTES: usize = 20 * 1024 * 1024;
/// The most pixels. Lower than the spec's 40 megapixels: each working copy of the picture
/// holds 8 bytes a pixel, so 20 megapixels already use about half a gigabyte while tracing.
pub const MAX_PIXELS: u64 = 20_000_000;

/// A decoded picture: grey values 0 (black) to 255 (white), row by row from the top.
#[derive(Debug, Clone, PartialEq)]
pub struct Decoded {
    pub width: u32,
    pub height: u32,
    pub grey: Vec<f64>,
}

impl Decoded {
    /// The grey value of the pixel in `column`, `row` (row 0 at the top).
    pub(crate) fn at(&self, column: usize, row: usize) -> f64 {
        self.grey[row * self.width as usize + column]
    }
}

/// Reads a picture's bytes. Hostile input gives an error, never a panic.
pub fn decode(bytes: &[u8]) -> Result<Decoded, ImageError> {
    if bytes.len() > MAX_BYTES {
        return Err(ImageError::TooLarge(bytes.len()));
    }
    let reader = || -> Result<ImageReader<Cursor<&[u8]>>, ImageError> {
        let reader = ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .map_err(|e| ImageError::Decode(e.to_string()))?;
        match reader.format() {
            Some(ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Bmp) => Ok(reader),
            _ => Err(ImageError::UnsupportedFormat),
        }
    };
    let (width, height) = reader()?
        .into_dimensions()
        .map_err(|e| ImageError::Decode(e.to_string()))?;
    if width == 0 || height == 0 {
        return Err(ImageError::Decode("the picture has no pixels".into()));
    }
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(ImageError::TooManyPixels(width, height));
    }
    let mut reader = reader()?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(width);
    limits.max_image_height = Some(height);
    reader.limits(limits);
    let picture = catch_unwind(AssertUnwindSafe(|| reader.decode()))
        .map_err(|_| ImageError::Decode("the decoder failed".into()))?
        .map_err(|e| ImageError::Decode(e.to_string()))?;
    let rgba = picture.to_rgba16();
    let grey = rgba
        .pixels()
        .map(|p| {
            let [r, g, b, a] = p.0.map(|c| f64::from(c) / 65535.0);
            // Rec. 709 luma, blended over white by the alpha.
            let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
            255.0 * (a * luma + (1.0 - a))
        })
        .collect();
    Ok(Decoded {
        width,
        height,
        grey,
    })
}
