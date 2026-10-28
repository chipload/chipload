# 0024: SVG import with `usvg`, in its own crate

Status: accepted (2026-10-28). Spec: docs/specs/svg-import.md (approved 2026-10-28).

## Context
Logos and artwork reach makers as SVG from Inkscape, Illustrator and Affinity (drawing
roadmap phase 3). SVG is large and loose: CSS and presentation attributes, `use` and
`symbol`, nested transforms, units and a viewBox, basic shapes, arcs and Bézier curves.
Reading all of that correctly is a project of its own; the resvg project's `usvg` does it
and reduces a file to plain paths in one coordinate system.

## Decision
- **`usvg` 0.48.1** (Apache-2.0 OR MIT), **without default features**: no text shaping
  and no system fonts (text in an SVG is left out and reported, so a file gives the same
  G-code on every computer), no writer, no built-in `.svgz` support. Its dependencies are
  MIT, Apache-2.0, BSD-3-Clause or Zlib; `cargo deny` passes.
- **A new crate, `chipload-svg`,** beside `chipload-dxf`, `chipload-image` and
  `chipload-text`: bytes in, shapes of lines and arcs in mm out, on `chipload-core` (and
  `chipload-i18n` for its errors). No file I/O; it builds for wasm32.
- **The XML is parsed once with `roxmltree`** (usvg's own, through `usvg::roxmltree`) with
  a node limit, a depth check before usvg's recursive conversion, and DTD entity loops
  refused; usvg converts that same document. The XML scan also gives Inkscape's layer
  names, which usvg drops, and the counts of text, pictures and clips left out.
- **`.svgz` is unpacked here with `flate2`** (already in the tree), capped at 50 MB, as
  usvg's own unpacking has no limit.
- **Links are never followed:** no resources directory, and an image resolver that
  resolves nothing.
- **Curves are fitted** like text outlines: flattened within a quarter of the arc
  tolerance, then lines and arcs with `chipload_core::fit::fit_path` within the rest.

## Consequences
- The architecture test keeps `chipload-svg` on the core, and away from the checker.
- usvg holds coordinates as `f32`: about 0.0005 mm at a metre, well inside the arc
  tolerance.
- A path drawn through `use` comes back without its element's id; such paths are keyed by
  their order for updates.
