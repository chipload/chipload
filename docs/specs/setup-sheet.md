# Spec: setup sheet for the operator

Status: approved (2026-10-12; decisions 1–7 as recommended). Item 8 of the general improvement pass (TODO.md,
approved 2026-10-12).

## 1. Why

At the machine, the operator (often the user, later) needs to know what stock to load, where
X0 Y0 and Z0 are, which tool goes in for which file and in what order, and what to watch
for. Today that means opening the project in Chipload or reading the G-code. Mistakes here
(wrong zero, wrong tool, files run out of order) crash tools and ruin stock. A setup sheet
is one printable page written next to the G-code that answers these questions.

## 2. What is on it

Everything comes from the checked run that produced the G-code (`RunResult` and the job), so
the sheet can't disagree with the files.

1. **Header:** the project file name, the machine (profile name if any) and post, `written
   by chipload x.y.z`. All lengths in mm.
2. **Stock:** X × Y × thickness.
3. **Zero:**
   - **X0 Y0** in words, measured on the stock: "the stock's bottom-left corner", "the
     stock's centre", or for the drawing origin and a chosen point, "25 mm right of and
     10 mm up from the stock's bottom-left corner" (with "left of", "below" and "outside
     the stock" when it applies).
   - **Z0:** "top of the stock" or "bottom of the stock (the spoilboard)".
   - The picture marks it.
4. **Picture:** a top view of the stock with the parts' contours, the toolpath's cutting
   moves coloured by file (or by tool in a one-file job), the tabs, and the X0 Y0 mark with
   arrows for +X and +Y. Rapids aren't drawn (they'd clutter it). Copies show as they're cut.
5. **Files, in the order to run them.** Each with:
   - the file name and its estimated time;
   - before it: "Load tool T3 (6 mm end mill, 2 flutes), set Z zero on the top of the stock"
     (the Z zero line from the second file on, since a tool change means touching off again);
   - the tool: number, name, type, diameter, flutes, V-bit angle and tip, spindle speed and
     direction;
   - its operations in cut order: name and kind (profile outside/inside/on the line,
     pocket and strategy, V-carve), depths (from → to, step down), stepover, feed and
     plunge feed, entry (plunge, ramp, helix), tabs (count × width × height), copies, and
     each one's estimated time;
   - the deepest cut: "Cuts down to Z −12.3 (0.3 mm into the spoilboard)", so the operator
     can check the tool sticks out far enough.
   - In a one-file job with M6, one file section with a "Tool change to T3" heading before
     each tool group.
6. **Watch out:** the run's notes and checker warnings (a cut into the spoilboard, a tool
   not marked centre-cutting, tabs left to cut free, material left), and the post's own
   warning when it says it's untested on a real controller.
7. **Total:** the estimated time line as in the report, with tool changes and the note that
   real time is longer.

Not on it (for later): the stock material and free notes (they'd need new project fields),
a date (§5.6), inch units (the inch display item).

## 3. Format

One self-contained **HTML** file: inline CSS and an inline SVG picture, no scripts, no
external files, no fonts to fetch. It opens in any browser and prints to one or two A4 or
Letter pages (`@media print` styles: black on white, the picture fits the page width, file
sections don't break across pages where avoidable). Text is escaped (layer and operation
names come from the user's drawing).

Built with plain Rust string formatting in a new module `chipload_pipeline::sheet`; no new
dependency. Numbers are formatted explicitly (as in the G-code: fixed decimals, no `-0`).

## 4. When it's written

- **CLI:** `chipload run part.chipload -o out.nc` also writes `out-sheet.html` next to it;
  `--no-sheet` leaves it out. A separate `chipload sheet part.chipload -o sheet.html` writes
  only the sheet (it runs the job, so it is checked the same way, but writes no G-code).
- **GUI:** Export G-code also writes the sheet next to the exported files, and the export's
  status line names it. A checkbox in the Settings menu ("Write a setup sheet with the
  G-code", on by default) turns it off; it's kept in the app settings as a new optional
  field with a default, so no settings schema bump.
- Never written when the run fails: no G-code, no sheet.

## 5. Decisions to review

1. **HTML with an inline SVG**, not PDF. *Recommended:* no dependency, prints well from any
   browser, readable on a phone or tablet at the machine. PDF can come later if wanted.
2. **Written by default with the G-code** (`--no-sheet` and a setting to turn it off), not
   only on request. *Recommended:* the point is that it's there when you reach the machine;
   one small extra file per export. Alternative: only with `--sheet` and a separate
   "Export setup sheet" command.
3. **The content in §2**, including X0 Y0 described in words relative to the stock and the
   deepest cut per file. *Recommended.* Tool stick-out isn't computed (it depends on the
   holder and clamps); the deepest cut lets the operator judge it.
4. **The picture shows cutting moves coloured per file, no rapids.** *Recommended:* readable
   when printed in black and white too, since the files are also told apart by line style
   (solid, dashed, dotted, repeating).
5. **Lives in the pipeline** (`chipload_pipeline::sheet::setup_sheet(job, &RunResult,
   file_names) -> String`), shown by CLI and GUI alike. *Recommended* (rule 6).
6. **No date or time on the sheet**, so it's deterministic and can have golden files like
   the G-code; the file names and the `written by` version tie it to its programs.
   *Recommended.* Alternative: a date line, kept out of the golden tests by a parameter.
7. **Sheet file name `<stem>-sheet.html`**, next to `<stem>.nc` or `<stem>-1-T1.nc` and so
   on. *Recommended.*

## 6. Tests

- Pipeline: golden HTML sheets for two corpus projects (`tests/corpus/expected/sheets/`):
  the two-tool project written as two files, and the one-file-with-M6 project. The same
  input gives byte-identical sheets.
- The zero described in words for each work zero choice and both stock positions (bottom
  left and centre), including a point outside the stock.
- Names with `<`, `&` and quotes are escaped; the SVG's viewBox covers the stock and all
  cutting moves; the deepest cut and spoilboard depth match the program.
- CLI: `run` writes `out-sheet.html` and `--no-sheet` doesn't; `sheet` writes only the
  sheet; a run refused by the checker writes neither.
- GUI: export writes the sheet next to the files when the setting is on, not when off.

## 7. As built (2026-10-12)

- `chipload_pipeline::sheet`: `setup_sheet(job, &RunResult, output, &SheetInfo)`,
  `write_setup_sheet`, `sheet_path`, `xy_zero_text`. `RunResult::groups` (the tool groups)
  was added so a one-file job with M6 can be shown group by group. `SheetInfo` carries the
  project's name and `written_by`, so the golden sheets use a fixed version.
- The post's warning comes from a new optional `note` in the post config (no post schema
  bump: an optional field with no effect on the G-code); the LinuxCNC and Mach3/4 posts
  carry their "not yet tested" note.
- The picture: the stock, every contour at every copy (grey), each group's cutting moves
  with its own colour and dash, tabs in orange, the X0 Y0 mark with X+ and Y+ arrows; lines
  keep their width at any size (`vector-effect`). Coordinates to 0.01 mm.
- Non-ASCII text is written as character references, as the file goes through the same
  ASCII-only atomic write as the G-code.
- CLI: `run` writes `<stem>-sheet.html` unless `--no-sheet`; `chipload sheet PROJECT -o
  FILE [--gcode PATH] [--tool-change ...]` writes the sheet alone (G-code names from
  `--gcode`, else the project's name with the post's extension). The G-code golden test
  runs with `--no-sheet`; the sheets have their own goldens in
  `tests/corpus/expected/sheets/` (`cargo test -p chipload-pipeline --test sheet`).
- GUI: export writes the sheet unless Settings → *Write a setup sheet with the G-code* is
  off (`setup_sheet` in the app settings, optional, on when missing); the status line and,
  for several files, the report name it.
