# 0020: Text with built-in GNU FreeFont, in its own crate

Status: accepted (2026-10-16). Spec: docs/specs/text.md.

## Context
The user wants to write text in the app, placed and turned like a picture. Text needs a font:
the letters must be the same on every computer and in every version, or a project would cut
differently elsewhere. ADR 0008 kept OFL fonts out of the app (the GUI uses the system's
fonts for its interface).

## Decision
- **A new crate, `chipload-text`,** beside `chipload-image`: text, font and size in, closed
  outlines of lines and arcs in mm out. It depends only on `chipload-core` and `ttf-parser`
  0.25.1 (MIT OR Apache-2.0), which egui already pulled in. It has no file I/O and builds for
  the browser.
- **Six built-in fonts:** GNU FreeFont 20120503, Sans, Serif and Mono, regular and bold,
  under GPL-3.0-or-later with the font exception. They are cut down to Latin, digits and
  common symbols (324 KB in all) by `packaging/fonts/make-fonts.sh`. The script pins the
  release by SHA-256; its GPG signature was checked against the GNU keyring. The fonts and
  `fonts/LICENSE.txt` are committed, and `THIRD-PARTY-LICENSES.txt` carries the notice.
- **A text is a drawing source, like a picture:** `drawing.texts`, with its letters stored
  as `path` shapes that name it. The font is needed only to change a text, never to cut one.
- **Curves:** each glyph's Béziers are flattened to a quarter of the arc tolerance, then
  fitted with lines and arcs (`chipload_core::fit`) within the rest, so the letters are
  within 0.01 mm of the font and come out as G2/G3 arcs.
- **Kerning:** from the OpenType `kern` feature (GPOS pair adjustments), else the old `kern`
  table.

## Consequences
- No new licence in the dependency list. The fonts' licence is the project's own, plus an
  exception that only widens it.
- `drawing.texts` is optional and left out when empty, so no schema bump. Older versions
  refuse only projects that have a text.
- Installed fonts, single-line engraving fonts and text on a curve are not done (spec §7).
