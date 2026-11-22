# 0033: A single-line font, read from its SVG font

Status: **accepted** (2026-11-22, the user approved docs/specs/single-line-text.md: "Go
ahead").

## Context

Engraving labels, numbers and small lettering wants one pass down the middle of each
stroke, not an outline round it. Every built-in font is an outline font (ADR 0020, 0021).
A single-line font is drawn as open strokes; the ones made for CNC are published as SVG
fonts (and as OpenType fonts whose strokes are closed back on themselves, doubling every
line). The classic Hershey data comes with a use notice of its own, not a licence on the
approved list.

## Decision

- Bundle **Relief SingleLine** (isdat graphic design department), its SVG font as published,
  under the **SIL Open Font License 1.1** (approved for fonts, ADR 0008). Its licence ships
  as `crates/chipload-text/fonts/LICENSE-OFL-ReliefSingleLine.txt` and in
  THIRD-PARTY-LICENSES.txt.
- Read it with **our own small SVG font reader** in `chipload-text`: the XML through
  `roxmltree` and the path data through `svgtypes`, both MIT OR Apache-2.0 and already in
  the tree through `usvg` at the same versions (no new crates). Curves are flattened and
  fitted with lines and arcs within the arc tolerance, as the outline fonts' are.
- The font is a `BuiltinFont` with `Style::SingleLine`; its letters are open paths
  (`Lettering::closed`), not merged where strokes cross.

## Consequences

- No schema change: `font` names it as any built-in font.
- It has no Cyrillic; Russian text needs an outline font. A centre line of the outline
  fonts could come later for that.
- egui can't draw an SVG font, so the font list shows its name in Inter.
