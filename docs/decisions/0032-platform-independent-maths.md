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
- Since 0.37.0 (2026-11-14, the user: "do that change in the next release cycle") every
  crate uses them, not only the relief path: profiles, pockets, V-carving, the drawing,
  DXF and SVG import, the checker, the posts and the window. `chipload_core::math` gained
  `sin`, `cos`, `atan`, `atan2`, `asin` and `acos`; `chipload-post` depends on
  `chipload-core` for them. `clippy.toml` refuses the platform's functions
  (`disallowed-methods`) in the whole workspace, tests too, so the rule holds by
  construction; only `math`'s own tests call them, as the reference. `f64::powi` stays
  allowed: it is repeated multiplication (compiler-builtins), the same everywhere. The
  window's `f32` maths (egui's screen geometry) is not covered and reaches no output.
- The sweep changed no golden. Two tests showed where a last bit had mattered: an arc read
  from a DXF went from its start angle to a point and back (`atan2` of a `cos`, `sin`),
  which glibc happened to give back exactly; under a move or a uniform scale the angle is
  now kept as it is. And the line-and-arc fit's limit of half a turn now allows
  `ANGLE_TOL_RAD` more, so an exact half circle is one arc on every platform.
- Dependencies keep the platform's functions inside them (`cavalier_contours`'s offsets,
  `boostvoronoi`, `usvg`'s curves); their goldens have matched on every platform so far.
