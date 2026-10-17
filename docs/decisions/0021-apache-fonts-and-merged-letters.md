# 0021: Ten Apache-2.0 fonts, and overlapping letters merged

Status: accepted (2026-10-17). Spec: docs/specs/more-fonts.md. Builds on ADR 0020.

## Context
The user asked for more fonts, "preferably some more artistic". Most artistic fonts are
under the SIL Open Font License, which is outside our licence list (ADR 0008). Connected
scripts overlap where one letter runs into the next, and `chipload-text` gave those
outlines as they are, crossing each other, which a pocket or V-carve can't use.

## Decision
- **Fonts:** Roboto Slab (Regular and Bold, fixed from its variable font), Satisfy,
  Yellowtail, Rochester, Luckiest Guy, Ultra, Chewy, Smokum, Cherry Cream Soda and
  Permanent Marker, from the `apache/` folder of the Google Fonts repository: all under
  Apache-2.0, already on our list. `packaging/fonts/make-fonts.sh` pins commit 47d54ca1 and
  each file's SHA-256, and cuts them down to FreeFont's Latin set with fontTools (a build
  tool, not shipped). Eleven faces, 489 KB. The licence text is shipped
  (`fonts/LICENSE-Apache-2.0.txt`) and each font's copyright is in
  `THIRD-PARTY-LICENSES.txt` (about.hbs).
- **A font's style** (`chipload_text::Style`: plain, script, display) groups the app's list
  and `text fonts`. The app adds the built-in fonts to egui only to draw their names in the
  list; the interface keeps the system's fonts.
- **Capital height:** a font's stated capital height is used unless its H differs by more
  than 10 % (Rochester states 326 units for an H of 1741); then the H's. FreeFont's are
  unchanged.
- **Merging:** `chipload_toolpath::merge::merge_overlaps` unites the fill of a text's
  outlines by the non-zero rule with the existing area booleans (cavalier_contours, arcs
  kept). `chipload-text` says which letter each outline is of, so a hole belongs to its
  own letter when letters overlap. Loops that overlap nothing are returned exactly; loops
  that only touch stay apart; hairline slivers are dropped and two-piece lenses get a third
  point. Every merged group is checked by sampling its fill against the input's, and for
  crossing loops; a failure is an error naming the text. The pipeline merges in
  `texts::lettering`, so `chipload-text` stays core only.

## Consequences
- No new licence, no new dependency, and no schema change of its own: a project names its
  font. (A project saved by 0.22 is schema 29 for the surface operation, so older versions
  don't open it anyway.)
- Existing texts don't change until they are changed; FreeFont texts without overlaps make
  the same shapes as before (tested).
- Found while testing and fixed the same day: an offset library failure on Smokum's sharp
  notches (pockets now retry with the arcs as chords, offset-retry.md §8), and V-carve
  centre lines split too coarsely where the depth meets its cap (more-fonts.md §6).
