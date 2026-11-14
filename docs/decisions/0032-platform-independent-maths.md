# 0032: Elementary functions with the same bits on every platform

Status: **accepted** (2026-11-14, while releasing 0.36.0, after the user said to do the
release).

## Context

The same project must give byte-identical G-code (CLAUDE.md, *Determinism*), and the golden
G-code test runs on Linux, Windows and macOS. Releasing 0.36.0, the macOS run failed for one
relief (`v36_relief_box_smooth`): its smoothing lines run at 120°, and `f64::sin` and `cos`
call the platform's maths library, whose last bit differs between glibc and Apple's.
Nudging either value by one bit on Linux changed that golden and the Relief wizard's: a
relief samples its lines, refines and thins the points against thresholds, so one bit can
add or drop a point. IEEE 754 rounds `+ − × ÷` and `sqrt` the same everywhere; it doesn't
fix `sin`, `cos`, `tan`, `exp`, `pow` or `hypot`.

## Decision

`chipload_core::math` has `sin_cos`, `tan`, `exp`, `pow` and `hypot` through the `libm`
crate (MIT OR Apache-2.0, a Rust port of musl's maths library, already built for
`boostvoronoi`), which computes them in plain Rust and gives the same bits on every platform,
the browser too. The relief path uses them: the lines' direction, the model's turn, the rim's
ramp, the cutters' angles, the drop cutter's and the relief's lengths, the path check, and
the depth map's gamma and blur.

## Consequences

- No new crate in the build: `libm` was already there.
- The goldens didn't change on Linux (glibc and `libm` agree there); macOS now matches them.
- The rest of the toolpath (profiles, pockets, V-carving, offsets in `cavalier_contours`)
  still uses the platform's functions. Its goldens have matched on every platform so far,
  as their output is rounded far from any threshold. A case found later moves that code to
  `chipload_core::math` too.
