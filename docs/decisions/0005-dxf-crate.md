# 0005: Read DXF with the `dxf` crate

Status: accepted (2026-10-09)

The Rust port reads DXF files with `dxf` 0.6.2 (ixmilia/dxf-rs, MIT; maintained since 2016,
last release 2026-10-03). All of its dependencies are MIT, Apache-2.0, BSD or Unicode-3.0
licensed (`cargo deny check` passes).

## What the crate gives us
A faithful reader generated from the DXF spec: every entity type we import (LINE, ARC,
CIRCLE, LWPOLYLINE with bulges, POLYLINE + VERTEX, SPLINE, ELLIPSE, INSERT incl. row/column
arrays), extrusion directions, layers, handles, blocks with base points, and `$INSUNITS`.

## What we build ourselves (in `chipload-dxf`)
The importer reads the drawing as drawn and changes as little as possible (decision
2026-10-09: it doesn't try to reproduce ezdxf). Everything goes through one 2D affine
transform per entity:
- **Extrusion:** only ±Z is accepted; −Z (the mirrored OCS, common for mirrored arcs) flips x.
  Entities tilted out of the XY plane are skipped and reported, never projected.
- **Block expansion:** INSERT base point, scale, rotation, extrusion and row/column arrays,
  nested blocks with a depth limit, layer "0" inheritance. Arcs stay arcs under a similarity
  (a mirror reverses the sweep); arcs under non-uniform scaling become lines within
  `ARC_APPROX_TOL_MM`.
- **Spline and ellipse flattening** within `ARC_APPROX_TOL_MM` (rational B-splines from
  control points). A spline given **only by fit points** is skipped with a warning: its shape
  depends on end tangents every CAD program picks differently, so we don't guess
  (ezdxf interpolates it; this is the one known difference from Python, see
  `tests/conformance/test_dxf_import_parity.py`, removed with Python in ADR 0009).
- **Nesting** uses cavalier_contours, with an exact fallback when boundaries touch, so a shape
  sharing edges with its container from the inside counts as nested (Shapely `contains`
  semantics, which the pocket island rule relies on).

## Crash in dxf 0.6.2 on local-time dates
The crate converts `$TDCREATE`/`$TDUPDATE` and OBJECTS timestamps from the epoch
1900-01-01 00:00 *local time* and unwraps it. In time zones where that instant is ambiguous
(Europe/Stockholm switched from local mean time to CET then) every file with such a date
panics, including most AutoCAD and ezdxf output. Workaround in `sanitize.rs`: before
parsing an ASCII DXF we drop those two header variables and the OBJECTS section, neither of
which affects geometry. The parse also runs under `catch_unwind`, so any other panic in the
parser becomes an import error instead of a crash. Worth reporting upstream; remove the
workaround once a fixed release is out.

## Trade-offs
- It pulls in `image` (BMP only, default features off) for DXF thumbnails, plus `chrono` and
  `uuid`. Acceptable for a desktop app; revisit if binary size matters.
- Flattened splines won't match ezdxf point-for-point, so import conformance compares
  structure exactly (contours, closed/open, layers, nesting, issue codes) and geometry within
  tolerance.
