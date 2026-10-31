# Spec: text in the drawing

Status: **approved 2026-10-16** ("Go ahead!"), all decisions as recommended (§9).
Built the same day (ADR 0020). As built:
- §2: FreeSans has no Latin kerning in the font itself (its kern pairs are Cyrillic); FreeSerif's
  is read from the OpenType `kern` feature, else the old `kern` table. FreeSans Bold and FreeMono Bold have no ⌀.
- §4: the box is the one round the fitted lines and arcs themselves (an arc can bulge a
  micrometre past the points it was fitted to), so the text's corner is exactly at `x`, `y`.
- §5: Draw > Text is also in the DRAW menu and the Drawing folder's right-click menu; placing
  starts in top view, as drawing does, and Esc stops it.
- §7: `drawing.texts` is left out of the file when empty, so projects without text still
  open in 0.19.
- The setup sheet lists each text with the DXFs and pictures when there are several.
- §3 `anchor` (user, 2026-10-16: "choose Center placement for the text instead of only left
  corner"): optional, `corner` when missing. Changing it in Properties, or with `text set
  --anchor` without `--at`, moves the position so the letters stay where they are.

Guide: docs/book/src/drawings/text.md.

User request (2026-10-16): "add a text object to chipload so we can write text directly in
the app. We should be able to position and rotate similar to images. It should fit in with
the workflow."

## 1. What it is

A **text** is a drawing source, like a picture (docs/specs/image-trace.md):
- what it says;
- the font;
- the size;
- where it goes and how it's turned;
- the layer.

Its letters become **shapes**: closed outlines with arcs and lines, the holes of letters
such as O and A included. These shapes are stored in the project, as a picture's traced
shapes are.
- **They cut like any drawn shape:** a V-carve, a pocket or a profile selects them, and
  they are planned and checked the same way.
- **The project cuts the same on any computer:** the font is needed only to change the
  text, not to cut it.
- **Changing the text** (its words, font, size, place or turn) makes its shapes again, as
  re-tracing a picture does. An operation that selects the whole text follows.

## 2. Fonts (decision 2)

- **Built in:** the three GNU FreeFont faces, FreeSans, FreeSerif and FreeMono, in
  regular and bold.
  - **Licence:** GPL-3.0-or-later with the font exception, which is inside our licence
    list.
  - **Size:** the full files are about 8 MB together, so each is cut down to the Latin
    letters (accents included), the digits and the common symbols. That leaves about
    0.5 MB in all, compiled into the text crate. The cut-down files are made once by a
    script in `packaging/`, from the release named in it, and committed.
  - **Every build has the same fonts,** so the tests, the command line, the app and the
    browser build make the same letters.
- **A project names its font** (`"FreeSans"`, `"FreeSerif Bold"`, …). A project from a
  newer Chipload with a font this one doesn't have keeps its shapes, and cuts. Changing
  its text asks for another font.
- **The computer's installed fonts** came 2026-10-31 (docs/specs/system-fonts.md): only
  their name goes into the project, not the file.

## 3. The text object (file format)

`drawing.texts` is a new list, empty by default. Each text has:

| field | meaning |
|---|---|
| `id` | its id, which its shapes name (`"text": id`) |
| `text` | what it says; `\n` starts a new line |
| `font` | a built-in font's name |
| `height` | the height of a capital letter, mm (what a ruler measures on the H) |
| `x`, `y` | the text's anchor on the stock, mm |
| `anchor` | `corner` (default): `x`, `y` are the lower-left corner of the text's box, before the turn; `center`: its centre, so a centred text stays centred when its words, font or size change (added 2026-10-16 at the user's request) |
| `rotation` | degrees counter-clockwise about the box's centre, 0 to <360, any angle |
| `align` | `left`, `center` or `right`: lines against each other |
| `spacing` | extra space between letters, mm (0: the font's own, kerning included) |
| `line_spacing` | distance between lines, in capital heights (default 1.6) |
| `layer` | the layer its shapes go on (default `"text"`) |

- **Its shapes** are `path` shapes on its layer, with `"text": id`, as a picture's are
  with `"image": id`.
- **Selector:** `"text": id` picks the whole text, and `exclude` leaves letters out
  (docs/specs/leave-out-shapes.md). A left-out letter is found again by its place when the
  text is made again. If it can't be found, there is a problem, as with pictures.
- **Format version:** an optional list with a safe default needs no schema bump (CLAUDE.md
  rules). Chipload 0.19 and older refuse a project that has a text, with their usual
  "unknown field" message. Other projects are unchanged. This is the same as `exclude` in
  0.17.3.

## 4. Making the letters (the `chipload-text` crate)

- **A new library crate, `chipload-text`:**
  - it depends on `chipload-core` and `ttf-parser` (0.25.1, MIT or Apache-2.0, already in
    the dependency tree through egui);
  - it has no file I/O, and builds for the browser;
  - it holds the fonts as bytes.
- **Glyphs:** the outlines, laid out along the line. The pen moves by each letter's advance
  width plus the font's kerning (`kern` table) and the extra `spacing`. Lines are
  `line_spacing` capital heights apart and aligned by `align`. The text is scaled so a
  capital H is `height` tall.
- **Curves:** the glyphs' quadratic and cubic Béziers are flattened finely, then fitted with
  lines and arcs within the arc tolerance (0.01 mm), with `chipload_core::fit`, as traced
  pictures are. Each outline is a closed path. Tiny or broken contours are dropped and
  reported, never silently.
- **Placed:** the box (every letter's outline, at the line's base) has its lower-left
  corner at `x`, `y`, and is turned by `rotation` about its centre. This is exactly a
  picture's rule (docs/specs/direct-placement.md).
- **Deterministic:** the same text, font and numbers always give the same shapes,
  byte-identical G-code.

## 5. In the app (decision 5)

- **Draw > Text** in the ribbon, next to Image. Click on the canvas where the text should
  start. A text "Text" appears with its lower-left corner there, and Properties opens with
  the text field ready to type into.
- **Properties of a text:**
  - **Text:** several lines;
  - **Font:** a list of the built-in fonts;
  - **Height;**
  - **Position, Rotation:** with the quarter-turn buttons and resets, as for pictures;
  - **Align, Letter spacing, Line spacing;**
  - **Layer;**
  - **Remove.**

  A change applies when the edit is finished and makes the letters again, as one undo step.
- **Browser:** under Drawing, each text is a row (an "A" icon), with an eye to hide it.
  Clicking the row selects the whole text, and clicking a letter on the canvas does too, as
  with a picture. Ctrl+click on a letter leaves it out.
- **Operations:** with a text selected, V-carve, Pocket or Profile select the whole text
  (`"text": id`). A new V-carve takes a V-bit as it does for a picture.
- **Moving and turning on the canvas** is through Properties only, as for pictures today.

## 6. Command line (decision 5; rule 5: the CLI first)

    chipload text add part.chipload "MAKER\nSPACE" --height 20 --at 40,30 [--rotate 15]
        [--font "FreeSerif Bold"] [--align center] [--spacing 1] [--line-spacing 1.4]
        [--layer text]                           # prints its id
    chipload text set part.chipload 1 --text "MAKERSPACE" --height 25 …
    chipload text list part.chipload
    chipload text remove part.chipload 1         # with its shapes
    chipload text fonts                          # the built-in fonts
    chipload place part.chipload --text 1 --at X,Y / --rotate 30

`get`/`set` reach every field (`drawing.texts.0.text=…`), and `chipload mcp` gets the
new commands as tools by itself.

## 7. Not now (decision 4)

- **Single-line engraving** (a follow path down the middle of each stroke). The outlines
  can be V-carved, which engraves too. Centre lines of the letters, as for traced pictures,
  or a single-line font, can come later.
- **Text on a curve,** right-to-left scripts, ligatures and complex shaping. Letters are
  laid out one after another; accented Latin letters work, as the fonts have them.
- **Installed fonts** (§2).

## 8. Tests

- **Letters:** "HO" in FreeSans at 10 mm:
  - the H is one closed outline, 10 mm tall (within the tolerance);
  - the O is an outline with a hole;
  - every arc is within 0.01 mm of the glyph's curve;
  - no contour crosses itself.
- **Layout:** advance and kerning ("AV" is narrower than "A" plus "V"); spacing; two lines
  aligned left, centre and right; the box's lower-left corner at `x`, `y`; rotation about
  the centre (as the picture test).
- **Deterministic:** the same text twice gives the same shapes. A golden project with a
  V-carved text gives frozen G-code.
- **Selector:** `"text": id` picks every letter; `exclude` leaves one out and finds it
  again after the size changes.
- **CLI:** `text add/set/list/remove/fonts`, and `place --text`.
- **GUI:** Draw > Text places a text and opens it; editing it makes the letters again as one
  undo step; clicking a letter selects the text; a V-carve of it generates.
- **The checker passes** on everything cut from text.

## 9. Decisions

1. **A text is a drawing source like a picture.** Its letters are shapes stored in the
   project, and the font is needed only to change it. **Recommended**.

   Alternative: shapes made from the font each time the project is planned, which needs
   the exact font on every computer and in every version.
2. **Fonts: the three GNU FreeFont faces built in** (Sans, Serif, Mono, regular and bold;
   GPL-3.0-or-later with the font exception; cut down to Latin, about 0.5 MB).
   **Recommended**: the same letters everywhere, and a licence already in our list.

   Alternatives:
   - also the computer's installed fonts, now rather than later (the font file goes into
     the project);
   - a font under the OFL (Liberation, Noto), which is outside our licence list and so
     needs your approval.
3. **Size is the capital height in mm; position and rotation as for pictures**
   (lower-left corner, any angle about the centre). Several lines, with alignment and
   spacing. **Recommended**.
4. **Outlines only for now:** V-carve, pocket, profile. **Recommended**.

   Single-line engraving follows as its own step.
5. **The app as §5 and the command line as §6.** **Recommended**.

   Alternative: dragging and turning the text on the canvas with handles. That would also
   be wanted for pictures, so it would be its own step for both.
