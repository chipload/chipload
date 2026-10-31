# Spec: texts in the computer's fonts

Status: **draft 2026-10-31, to review** (decisions in §9). Builds on docs/specs/text.md and
more-fonts.md.

User request (2026-10-31): "I think we should allow to use fonts from the computer when
creating texts, not only the integrated ones. Since the text is converted to shapes it will
be stored in the project file even when opening the file on another computer. We must only
lock the editing of the text if the font is not available and show a helpful note, like if
you want to edit the text you must choose another font."

## 1. What changes

- A text can be written in any outline font installed on the computer (TrueType or
  OpenType, `.ttf`, `.otf`, `.ttc`), besides the 17 built-in ones.
- Nothing about the font goes into the project but its name. The letters are shapes in the
  project already (text.md decision 1), so the project cuts the same on any computer.
- **On a computer without the font**, the text keeps its shapes and cuts. Changing what
  needs new letters (words, size, spacing, line spacing, alignment) is locked, with a note:
  *"Font "Bahnschrift" is not on this computer. The text keeps its letters and cuts as
  they are; to change its words or size, choose another font."* Choosing another font
  unlocks it. Moving and turning it still work (§5).

## 2. Which fonts

- **The computer's fonts:** the system's font folders (Windows: `C:\Windows\Fonts` and the
  user's fonts; macOS: `/Library/Fonts`, `~/Library/Fonts`, `/System/Library/Fonts`;
  Linux: what fontconfig lists), read by the `fontdb` crate (§7).
- **And a `fonts` folder in the settings folder** (beside `posts/`), for fonts not
  installed system-wide, and for the tests (decision 4).
- Only fonts with outlines that `ttf-parser` reads and that have letters for "H"
  (`capital_height`, as for the built-in ones) are listed; bitmap and symbol-only fonts
  are left out. A variable font gives its default instance, and each face of a collection
  is a font of its own.
- **Name:** the face's full name as the system shows it ("Arial Bold", "Segoe UI
  Semibold"). A built-in font's name wins when both have it: projects that name
  "FreeSans" keep meaning the built-in one.
- The browser build has the built-in fonts only.

## 3. The project

- **No format change** (decision 1): `text.font` already names a font, and an older
  Chipload that meets a name it doesn't know already keeps the shapes and asks for another
  font when the text is changed (text.md §2). That is exactly the behaviour wanted here,
  so a project written with a computer font opens in older versions too.
- Nothing of the font file is stored (decision 2): fonts are licensed to the computer they
  are installed on, and embedding them would mean shipping them with every project. The
  letters, as geometry, are the user's drawing.
- Validation stays as it is: a font name is any non-empty text.

## 4. Where the font is read

- `chipload-text` takes the font's bytes: `letters(layout, font_data)` with the built-in
  fonts found by name as now (`builtin(name)`). No I/O in it.
- `chipload-pipeline::fonts` (not in the browser build) finds a font by name: built-in
  first, then the computer's and the settings folder's. The list is made once per run, the
  first time a text needs a font that isn't built in, so starting the app and planning
  projects without such a text don't scan the fonts at all (decision 6).
- `lettering`, `add_text`, `remake` and `remake_changed` take the font finder. A text whose
  font isn't found gives the typed error `FontMissing(name)`, whose message is the note
  in §1.

## 5. A text whose font is missing

- **It cuts:** planning never needs the font; it uses the stored shapes.
- **Locked:** words, size, spacing, line spacing, alignment. In the window those fields are
  greyed, with the note above them; on the command line `text set` with any of those
  refuses with the note.
- **Not locked:**
  - **Font:** choosing another (built-in or the computer's) makes the letters again in it.
  - **Moving and turning** (`x`, `y`, `anchor`, `rotation`, drags, Position, Turn): the
    stored shapes are moved and turned as they are, without the font (decision 3). The box
    and anchor come from the shapes.
  - Layer, removing it, exploding it into drawn shapes.
- The Browser row of such a text shows a small warning mark, with the note as its hover.

## 6. The window

- The text's **Font** dropdown has two groups (decision 5):
  - **Built in:** the 17, each in its own font, as now.
  - **On this computer:** the computer's fonts by name, a search field at the top (there
    may be hundreds), plain text; the one under the mouse shows a sample beside the list,
    loaded only then.
- The chosen font's name is shown in it as now; a computer font's sample is loaded when the
  text is opened.
- While the list is made (the first time it opens, in the background): *Looking for fonts…*.

## 7. The command line

- `chipload text fonts` lists the built-in fonts, then the computer's (`--json`: both, with
  `"builtin": true/false`).
- `text add` / `text set --font "Arial Bold"` take either.
- `chipload text list` marks a text whose font is missing.

## 8. Tests

- `chipload-text`: the same letters from a built-in font's bytes as by its name.
- Pipeline and CLI, with a settings folder whose `fonts/` holds a copy of a built-in
  font's file under another family name (made in the test; no new font in the
  repository): a text added in it; the project opened with that folder gone keeps its
  shapes and plans the same G-code; words and size refused with the note; another font
  accepted; moving and turning move the stored shapes (same shapes as before, moved).
- Window: the locked fields and the note; choosing a built-in font unlocks.
- A built-in name is never taken by a computer font of the same name.
- Determinism: golden files keep using built-in fonts only.

## 9. Decisions

1. **No format change:** the font's name in `text.font`, as now; older versions already
   lock such a text. *Recommended.* Alternative: a `font_source` field (built-in or
   computer), which needs a schema bump and adds nothing the name doesn't say.
2. **The font file is never stored in the project.** *Recommended* (licences; the letters
   are already there). Alternative: embed it, as the old note in text.md §2 said: lets any
   computer edit the text, but copies the user's licensed fonts into files they share.
3. **Moving and turning a text whose font is missing still work,** on its stored shapes.
   *Recommended:* only the letters themselves need the font. Alternative: lock the whole
   text but its font (simpler, but a text can't even be moved on the other computer).
4. **A `fonts` folder in the settings folder** besides the installed fonts. *Recommended:*
   fonts without installing them, and the tests get a known font without depending on the
   computer's. Alternative: installed fonts only.
5. **The dropdown: built-in first with samples, then the computer's as plain names with a
   search field and a sample of the one under the mouse.** *Recommended:* drawing hundreds
   of fonts in the list would load them all. Alternative: every name in its own font.
6. **The fonts are looked for only when needed** (a text in a computer font, or the list
   opened). *Recommended:* scanning can take a second with many fonts installed.
7. **Dependency: `fontdb` 0.24** (MIT, by the author of `ttf-parser`; its `fontconfig`
   feature reads the Linux font configuration without the C library). Without its default
   `memmap` feature, so font files are read into memory rather than mapped (mapping is
   `unsafe` and can crash if a file changes underneath). On the allowed list; not in the
   browser build. Alternative: `font-kit` (MIT/Apache-2.0), which links the platforms'
   font libraries.
