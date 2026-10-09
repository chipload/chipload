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
ezdxf did these for the Python version; the crate doesn't:
- **OCS → WCS** (the DXF "arbitrary axis algorithm"), so mirrored arcs (extrusion −Z) come out right.
- **Block expansion:** INSERT transforms (base point, scale, rotation, extrusion, arrays),
  nested blocks with a depth limit, layer "0" inheritance. Arcs under non-uniform scaling are
  flattened in block space and transformed as lines.
- **Spline and ellipse flattening** within `ARC_APPROX_TOL_MM` (rational B-spline evaluation).

## Trade-offs
- It pulls in `image` (BMP only, default features off) for DXF thumbnails, plus `chrono` and
  `uuid`. Acceptable for a desktop app; revisit if binary size matters.
- Flattened splines won't match ezdxf point-for-point, so import conformance compares
  structure exactly (contours, closed/open, layers, nesting, issue codes) and geometry within
  tolerance.
