# Spec: more built-in fonts, and merged letter outlines

Status: **approved 2026-10-17**, all decisions as recommended (§8); built the same day
(ADR 0021). Builds on docs/specs/text.md and ADR 0020.

As built:
- §2: eleven faces, 489 KB, pinned to Google Fonts commit 47d54ca1. Yellowtail's file has
  no licence entry in its name table; Google Fonts' metadata gives Apache-2.0 for all ten,
  and the licence text is shipped. Every font has the test's Latin set (only ⌀ is missing,
  as in four FreeFont faces).
- Rochester states a capital height of 326 units for an H of 1741: a stated height that
  differs from the H's by more than 10 % is replaced by the H's (FreeFont unchanged).
- §4: `chipload-text` gives each outline's letter (`Lettering::letter`), so a hole is told
  from another letter's outline round it; without that, lines set close enough to overlap
  were merged wrongly (the check caught it). Loops that only touch (no area in common) stay
  apart. Hairlines thinner than the arc tolerance on average are dropped, and a two-piece
  lens gets its arc split so it has three points. The check samples a 65 × 65 grid over
  each merged group, away from every edge.
- §6: every font V-carves a two-line sentence; every font but Smokum pockets one with
  lines that overlap (0.7 capital heights apart). Found: Smokum's spiked serifs (the
  outline turns back by more than 150°) make the pocket's first offset fail with tools
  under 2 mm even unmerged; V-carving lines that overlap can fail its own verification in
  FreeSerif Bold and Permanent Marker where strokes cross at shallow angles. Both stop with
  an error (TODO.md). Golden `v29_script_text`: two lines of Satisfy V-carved, an Ultra B
  pocketed. The inch comparison of the golden test now lets a move shorter than a rounding
  step vanish from either program (the grids don't nest), at most 1 % of a file's moves.
- §5: the guide's picture of the fonts is made by `packaging/docs/font-samples.py` (Pillow,
  a build tool) from the committed font files.

User request (2026-10-17): "Could we add more fonts? A nice selection should be included,
preferably some more artistic." Then, after a sample sheet of 22 candidates: "use your
suggestions".

## 1. What changes

- **Ten more built-in fonts** (eleven faces), artistic and plain, all under Apache-2.0, a
  licence already on our list (§2).
- **Overlapping letter outlines are merged** (§4). Connected scripts overlap where one
  letter joins the next, and some letters are drawn as overlapping strokes. Today those
  come out as contours that cross, which a pocket or a V-carve can't use. Every font's text
  is merged, the existing FreeFont faces included.
- **The font list shows each name in its own face** (§5), so you can choose by eye.

## 2. The fonts and their licence (decision 1)

| Style | Font (as a project names it) | By |
|---|---|---|
| Script | `Satisfy` | Sideshow |
| Script | `Yellowtail` | Astigmatic |
| Script | `Rochester` | Sideshow |
| Bold display | `Luckiest Guy` | Astigmatic |
| Bold display | `Ultra` | Astigmatic |
| Bold display | `Chewy` | Sideshow |
| Character | `Smokum` | Astigmatic |
| Character | `Cherry Cream Soda` | Font Diner |
| Character | `Permanent Marker` | Font Diner |
| Plain | `Roboto Slab`, `Roboto Slab Bold` | Christian Robertson |

(Designers as named in the Google Fonts metadata, checked 2026-10-17.)

- **Licence:** Apache-2.0, from the `apache/` directory of the Google Fonts repository
  (github.com/google/fonts). Apache-2.0 is on our list and compatible with
  GPL-3.0-or-later. Apache-2.0 needs the licence text and any NOTICE file kept with the
  fonts, and modified files marked as modified: `fonts/LICENSE-Apache-2.0.txt`, the notice
  in `THIRD-PARTY-LICENSES.txt` (about.hbs) listing each font, its copyright and "cut down
  to Latin by Chipload".
- **What you cut is yours:** Apache-2.0 puts no condition on documents or output made with
  the fonts; the letters in a project and its G-code are yours, as with FreeFont.
- **Not OFL:** most artistic fonts are under the SIL Open Font License, which isn't on our
  list (ADR 0008). These ten aren't; no new licence is added.
- **Pinned:** `packaging/fonts/make-fonts.sh` fetches each file from a fixed commit of the
  Google Fonts repository, checks its SHA-256, and cuts it down to the same Latin set as
  FreeFont with fontTools (a build tool, not shipped). The results are committed.
- **Roboto Slab** is a variable font: two static faces are made from it, Regular (400) and
  Bold (700), with fontTools' instancer, so every build has the same letters.
- **Size:** the eleven cut-down faces should add about 0.3–0.6 MB to the binaries; the
  real number goes in the ADR.

## 3. Letters a font lacks

Some display fonts have few symbols, and Luckiest Guy draws lower case as small capitals.
A letter a font doesn't have is left out with a note, as today ("Smokum has no letter for
"€": left out"). Nothing falls back to another font: the letters would no longer match.

## 4. Merging overlapping outlines (decision 2)

**Where:** after `chipload-text` lays out the letters (its closed outlines of lines and
arcs), and before they become shapes in the project: in the pipeline (`texts.rs`), which
uses the toolpath crate's area booleans (`cavalier.rs`, built on cavalier_contours, which
keeps arcs as arcs). `chipload-text` stays as it is: core only, no booleans.

**What:** the text's outlines are read as areas by the fonts' fill rule (non-zero winding:
an outer loop and the holes inside it), and the areas of all its letters are united. The
result is the same set of points the font would fill, as loops that don't cross:

- two letters joined by a stroke become one shape with one outline;
- a letter drawn as overlapping strokes becomes one outline;
- a counter (the hole in an "o") stays a hole; where another letter's stroke covers part of
  it, the hole shrinks by that much, or goes;
- letters that only touch at a point are left apart.

**Only where needed:** outlines that don't overlap anything are kept exactly as they come
from the font, so a FreeSans text that has no overlaps makes the same shapes as today.

**Checked:** the merged area equals the sum of the letters' areas less their overlaps,
within the arc tolerance, and no loop crosses another or itself. A merge that fails its
check, or a boolean that fails, makes the text an error naming the letters
("couldn't join the outlines of "ab" at …"), never shapes that silently differ.

**Existing projects:** a text's letters are stored as shapes (text.md decision 1), so
nothing changes until a text is changed; then it is made again, merged.

## 5. In the app

- **Font:** the drop-down lists the fonts in groups (Plain: the FreeFont faces and Roboto
  Slab; Script; Display), each name drawn in its own face. The app loads the built-in fonts
  into egui for this list only; the rest of the window keeps the system's fonts (ADR 0008).
- **Command line:** `chipload text fonts` lists them with their style; `--font "Luckiest
  Guy"` as before.
- **Guide:** the Text page shows a sample of every font (a picture made from the fonts by
  the screenshots script) and their licences.

## 6. Tests

- Every built-in font: the alphabet, digits and a sentence make loops that close, don't
  cross, and nest properly; each font has its licence and copyright in its name table.
- Merging: two overlapping squares become one outline; an "o" overlapped by a stroke keeps
  the uncovered part of its hole; touching letters stay apart; a script word in Satisfy
  becomes one or a few shapes with no crossings; areas match (§4).
- A FreeSans text without overlaps makes the same shapes as before (the corpus project
  v27_text and its golden G-code are unchanged).
- A pocket and a V-carve of a Satisfy word plan and pass the checker; a new golden file.
- A letter a font lacks: a note, the others made.

## 7. Out of scope

Installed fonts, single-line engraving fonts and text on a curve (text.md §7), OFL fonts,
and fonts outside Latin.

## 8. Decisions

1. **These ten fonts, under Apache-2.0** (§2). **Recommended:** artistic, cuttable, and
   no new licence. Alternative: also OFL fonts, which needs your approval of the OFL (FSF:
   a free licence, but not GPL-compatible when combined with a program; it's fine to ship
   alongside one, which is less clear for fonts built into the binary).
2. **Merge overlapping outlines, for every font** (§4). **Recommended:** scripts need it,
   and one rule for all fonts. Outlines that don't overlap stay exactly as they are.
3. **The merge in the pipeline with the toolpath's booleans** (§4). **Recommended:** the
   booleans exist, are checked and handle arcs; `chipload-text` stays core only.
   Alternative: booleans in `chipload-text` (a new dependency there).
4. **The font list drawn in each font** (§5). **Recommended:** choosing an artistic font by
   name alone is guessing. It loads the built-in fonts into egui for the list only.
5. **Roboto Slab as two static faces** (§2). **Recommended:** Regular and Bold, fixed.
6. **An ADR** (0021) for the added fonts and their licence, beside ADR 0020.
