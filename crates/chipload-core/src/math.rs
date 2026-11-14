// SPDX-License-Identifier: GPL-3.0-or-later
//! Elementary functions with the same result on every platform (ADR 0032).
//!
//! `f64::sin`, `cos`, `tan`, `exp`, `powf` and `hypot` call the platform's maths library, whose last bit
//! differs between Linux, macOS and Windows. Where a result decides something (how many
//! samples along a line, whether a point is kept), one bit can change the G-code, and the
//! same project would cut differently on another computer (seen 2026-11-14: a relief's
//! lines at 120° on macOS). These go through `libm`, a maths library written in Rust that
//! gives the same bits everywhere, the browser too. Plain arithmetic and `sqrt` need
//! nothing: IEEE 754 rounds them the same everywhere.

/// The sine and cosine of `x` (radians).
pub fn sin_cos(x: f64) -> (f64, f64) {
    (libm::sin(x), libm::cos(x))
}

/// The tangent of `x` (radians).
pub fn tan(x: f64) -> f64 {
    libm::tan(x)
}

/// e to the power `x`.
pub fn exp(x: f64) -> f64 {
    libm::exp(x)
}

/// `x` to the power `y`.
pub fn pow(x: f64, y: f64) -> f64 {
    libm::pow(x, y)
}

/// √(x² + y²), without overflow on the way.
pub fn hypot(x: f64, y: f64) -> f64 {
    libm::hypot(x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agree_with_the_platform_within_a_bit() {
        for k in 0..360 {
            let x = f64::from(k).to_radians();
            let (s, c) = sin_cos(x);
            assert!((s - x.sin()).abs() <= 2.0 * f64::EPSILON, "{k}");
            assert!((c - x.cos()).abs() <= 2.0 * f64::EPSILON, "{k}");
            if k % 90 != 0 || k % 180 == 0 {
                let t = tan(x);
                assert!(
                    (t - x.tan()).abs() <= 4.0 * f64::EPSILON * t.abs().max(1.0),
                    "{k}"
                );
            }
        }
        for x in [-3.0, -0.5, 0.0, 0.25, 2.0] {
            assert!((exp(x) - f64::exp(x)).abs() <= 2.0 * f64::EPSILON * f64::exp(x));
            assert!((pow(0.7, x) - 0.7f64.powf(x)).abs() <= 2.0 * f64::EPSILON * 2.0);
        }
        assert_eq!(pow(0.36, 1.0), 0.36);
        assert_eq!(hypot(3.0, 4.0), 5.0);
        assert_eq!(hypot(-6.0, 8.0), 10.0);
    }
}
