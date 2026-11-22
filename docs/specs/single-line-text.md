# Spec: single-line text

Status: **approved 2026-11-22** (the user: "Go ahead"; §6 as recommended; see *As built*,
§9). From the review of capability gaps (the user:
"Do 1, 2 and 4"). Builds on text.md (texts, their fonts), more-fonts.md (bundled fonts,
ADR 0020/0021), text-on-arc.md and follow-path.md.

## 1. What is missing today

Every font Chipload has is an **outline** font: a letter is the closed outline round its
strokes. That is right for V-carving and for pockets, but most engraving wants one pass of
a small bit down the middle of each stroke:
- labels, serial and part numbers on jigs and fixtures,
- small lettering (under about 8 mm), where a V-carve's grooves run together,
- quick nameplates, rulers and scales, tool-wall labels.

text.md §8 already lists single-line engraving as a later step.

## 2. What it does

- **Single-line fonts:** a new kind of built-in font whose letters are **open strokes**,
  not outlines. A text in one of them is written as today (words, height, position,
  alignment, line spacing, on an arc), and its letters are open drawn paths of lines and
  arcs.
- **Cutting them:** a [Follow path](follow-path.md) on the text's letters, with a V-bit or
  a small end mill, cuts each stroke once at the depth set. Profile, pocket and V-carve
  refuse open shapes, as they do today, and say why.
- **In the window:** the font list gets a *Single-line* group. A text in a single-line font
  draws its strokes thin on the canvas (they have no inside to shade). *New operation* on
  such a text offers *Follow path* first.
- **In the sign wizard:** *Engraved* with a single-line font makes a Follow path with the
  V-bit at the **Engrave depth** (a new field, 1 mm by default) instead of a V-carve.
  *Raised* and *Inlay* need outlines, and are refused with the reason.

## 3. The font (decision 1)

**Relief SingleLine** (isdat graphic design department, **SIL Open Font License 1.1**, an
approved font licence, ADR 0008): a single-line font designed for CNC milling, pen plotting
and laser engraving. The project publishes it as an SVG font with open paths. Its
character map is checked when it is added: the Latin characters of the seven languages, and
which of them are missing (Cyrillic for Russian is unlikely; see §6).

- **Bundled** as the other fonts are (compiled in), its licence and copyright in
  THIRD-PARTY-LICENSES.txt and the About box, and the font's name kept: OFL fonts may keep
  a Reserved Font Name, and the file is used as published, not changed.
- **More single-line fonts later,** one by one with their licences checked: the Hershey-text
  "EMS" families (derivatives of OFL fonts) are candidates. The classic Hershey data is
  left out: its terms are a use notice of its own, not a licence on the approved list.

## 4. Reading an SVG font (our own reader, decision 2)

- An SVG font is an XML file: a `<font>` with `horiz-adv-x`, a `<font-face>` with
  `units-per-em`, `cap-height` and `ascent`, and one `<glyph unicode="A" d="…"
  horiz-adv-x="…">` per character. Its `d` is SVG path data in font units, Y up.
- **Path data** is read with `svgtypes` (MIT OR Apache-2.0), already in the tree through
  `usvg`; `chipload-text` takes it as a direct dependency. Lines stay lines; quadratic and
  cubic Béziers are flattened densely and fitted with lines and arcs within
  `ARC_APPROX_TOL_MM` at the size written (core's `fit_path`, as SVG import and splines
  do), so the letters keep G2/G3.
- **Hostile input** doesn't apply in the same way (the font is bundled), but the reader
  still refuses what it can't read with an error, never a panic, and has tests on the
  bundled file.
- **Kerning:** SVG fonts carry `<hkern>` pairs; they are used as the outline fonts' kerning
  is.
- **Missing characters** show as a gap with a note, as an outline font's missing glyphs do.

## 5. Format and model

- **No schema change:** `font` names a font as today; the new name is one more built-in
  font. An older Chipload refuses the project because it doesn't know the font, and says
  which (as for a missing font today).
- **Letters are open paths:** `Geometry::Path { closed: false, origin: text }`. The overlap
  merging of outline letters (more-fonts.md) is skipped for single-line letters: strokes
  that cross stay as they are, and the follow path cuts each.
- **Text on an arc** places single-line letters the same way: moved and turned whole.

## 6. Decisions (recommendations first)

1. **Bundle Relief SingleLine** now, more single-line fonts later, each checked.
   *Alternative:* the centre line of the outline fonts (the medial axis the V-carve
   already builds, pruned), which works with all 17 fonts and the computer's. It leaves
   spurs at stroke ends and messy joints, and quality would vary from font to font. It
   could come later for Cyrillic, which Relief likely lacks.
2. **Our own SVG font reader** with `svgtypes` for the path data. *Alternative:* the OTF
   version through the outline font code, which closes every stroke back on itself
   (doubling every line).
3. **The sign wizard engraves single-line words** with a Follow path at an *Engrave depth*
   when a single-line font is chosen. *Alternative:* leave the wizard to outline fonts.
4. **No default font change:** FreeSans stays the default; single-line fonts are chosen.

## 7. Tests

- **Reader:** the bundled file reads; every glyph's strokes are open paths within the
  arc tolerance of the flattened curves; advance and kerning; a broken file is refused.
- **Text:** "HELLO" in Relief SingleLine gives open paths, at the height asked (the cap
  height), aligned and on an arc as other texts; no merging.
- **Operations:** a Follow path on the text runs through the checker and simulation; a
  V-carve or pocket on it is refused with the reason.
- **Sign wizard:** an engraved single-line sign has a Follow path at the engrave depth;
  raised and inlay refused.
- **Golden:** a single-line nameplate, Follow path with a V-bit.
- **CLI:** `text add --font "Relief SingleLine"`, `text fonts` lists it as single-line.

## 8. Order of work

1. The reader, the font bundled, texts in it, the CLI, the golden, the guide, licences.
2. The window: the font list group, thin strokes, Follow path offered; the sign wizard's
   engrave depth.

## 9. As built

- **Step 1:** `chipload_text::single_line` reads the SVG font (`roxmltree`, `svgtypes`;
  ADR 0033); `BuiltinFont::single_line`, `Style::SingleLine`, `Lettering::closed`. The font
  as published (`fonts/ReliefSingleLine.svg`, 423 characters, kerning pairs, no Cyrillic)
  with its licence beside it. Glyphs named by several characters (ligatures) are left
  out. Single-line letters are not merged (`chipload_pipeline::texts::lettering`). The
  font list's *Single-line* group came in this step too, its name drawn in Inter (egui
  can't draw an SVG font). Golden `v43_nameplate`.
- **Step 2:** `SignRequest::engrave_depth` (`sign add --engrave-depth`, the wizard's
  *Engrave depth* with a single-line font); raised and inlaid single-line signs refused.
  *New operation* keeps its order: rather than putting Follow path first, a profile,
  pocket or V-carve on open strokes is refused with a reason that now names Follow path
  (`toolpath-error-selects-open`), the same for any open shapes. Single-line letters
  already draw as thin lines on the canvas (open paths have no inside to shade).
