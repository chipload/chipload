# Spec: inches (G-code and display)

Status: approved 2026-10-17 with the recommended decisions. Step 1 (§4, G-code units)
step 2 (§5.1, §5.2, display units in the app) and step 3 (§5.3, the command line) built
the same day.

**As built, step 2:** every field, section summary, the canvas (cursor, grid, snapping,
measure, drawing and drag hints), the playback line, the chip load and the setup sheet
follow the display units. Report messages, planning notes and problem messages keep mm
with the unit written ("cuts 0.300 mm below the stock"): about 110 texts in the library
crates, left for a later step. In mm, a field also takes a length typed with `in` or `"`.

**As built, step 3:** length options take a unit (`0.25in`, `1/4in`, `6mm`) and feed
options `in/min`; X,Y pairs and the travel too. Since the command line's messages are those
same report texts, `--units inch` (§5.3) went only where the output is the sheet: `chipload
sheet --units inch` and `chipload run --sheet-units inch` (not `--units` on `run`, which
reads as the G-code's units). The MCP tools keep numbers in mm.

## 1. Why

Many hobby and small-shop CNC users work in inches: their tools are 1/4" and 1/8", their
stock is 3/4" ply, their feed charts are in in/min and their controllers are often set to
G20. Chipload reads inch DXFs, but everything after that is millimetres, and the checker
refuses G20.

## 2. What stays the same

- **Everything inside is millimetres.** Geometry, toolpaths, tolerances, the project file,
  the tool library and machine profiles all keep mm (CLAUDE.md: convert only at the edges).
  Inches are only ever a way to show, type or write a value.
- **`--json`, `get`/`set`, the JSON Schemas and the MCP tools stay in mm,** so a script
  never changes meaning with someone's preference.
- **Existing G-code doesn't change.** mm machines get byte-identical programs; no golden file
  changes.

## 3. Three separate settings

| Setting | Lives in | Values | What it changes |
|---|---|---|---|
| **Display units** | app settings (your preference) | mm, inch | what the app shows and accepts |
| **G-code units** | the machine (project, and machine profiles) | mm (G21), inch (G20) | what the program is written in |
| Drawing units | each DXF (exists today) | mm, inch | how the DXF is read |

They're independent: an inch-minded user can drive a G21 controller, and the other way round.
A G-code comment and the setup sheet always say which units the program is in.

## 4. G-code units

### 4.1 The machine setting

- `machine.gcode_units`: `"mm"` or `"inch"`, in the project and in machine profiles
  (`machines.json`).
- Project schema **27 → 28**: the field is required and written always; the migration sets
  `"mm"` explicitly, which is exactly what every older program used (G21). Machine profiles:
  the same as a format bump with its migration. (See decision 1.)
- Job settings > Machine gets a *G-code units* row; `chipload machine add/set
  --gcode-units inch`; `chipload set project machine.gcode_units=inch`.

### 4.2 The post

- Post config gets `units = ["mm", "inch"]`: what the template can write. Missing: `["mm"]`.
  The three built-in posts declare both.
- **A machine set to inch with a post that only declares mm is refused** with a clear
  message, before anything is written. This matters for user posts: their templates write
  `G21` themselves, and inch numbers under G21 would cut the part 25.4 times too small.
- Templates get two new values: `units_word` (`G20` or `G21`) and `unit` (`mm` or `inch`).
  The built-in templates write `{{ units_word }}` where they now write `G21`, and `unit` in
  their comments ("Tool 1: 0.2500 inch").
- In inch, the post converts every X, Y, Z, I, J and safe height to inches, and feeds to
  in/min. Spindle speed and dwell are unchanged.
- Decimals: the config's `decimals` stays for mm; a new `inch_decimals` (missing: 4) for
  inch. (Decision 2.)

### 4.3 The checker

The checker stays independent: it learns the units from the program itself and the expected
units from the job, never from the post.

- `G20` is no longer an error. Motion before G20 or G21 is still an error, and so is a
  **change of units within a program** (a classic crash).
- After G20 the checker multiplies every coordinate and feed by 25.4 as it reads them, so
  all its checks (safe height, stock, travel, rapids through material, arcs, max feed) run in
  mm as now.
- `CheckLimits` gets `units`: the machine's G-code units. **A program whose units word
  differs from the machine's is an error.** This catches a user post that writes G21 for an
  inch machine even if its config says otherwise.
- Tolerance: 4 inch decimals round a value by up to 0.00005" = 0.00127 mm, more than the
  checker's 0.001 mm position tolerance. In an inch program the position tolerance becomes
  half an output step plus the usual tolerance: 0.00127 + 0.001 mm. (Decision 2.)

### 4.4 Tests

- The built-in posts in inch: new golden files for three corpus projects (a profile with
  tabs, a pocket with arcs, holes with a peck cycle), in `tests/corpus/expected/inch/`.
- **The same motion:** for every corpus project, the inch program read back and multiplied by
  25.4 equals the mm program within half an inch step.
- Checker: G20 accepted; motion before units, a units change mid-program and a units word
  that differs from the machine's are errors; an inch program at exactly safe height passes.
- A user post without `units` and a machine in inch: refused, nothing written.
- The v27 corpus samples migrate to 28 with `gcode_units = "mm"` and give their golden G-code.

## 5. Display units

### 5.1 What changes in the app

With *Display units: inch* (Settings, next to the theme):

- **Every length field** shows inches with 4 decimals and accepts inches: depths, stepover in
  mm, stock size and thickness, safe height, positions, shape sizes, tool diameters, tip
  diameters, tab sizes, lead radii, text height, picture width, grid of copies gap, and so on.
- **Feeds** in in/min, **chip load** in in/tooth, **plunge** in in/min.
- **Fractions are accepted** when typing: `1/4`, `3/8"`, `1 1/2`. They're shown as decimals.
  (Decision 3.)
- The **report**, **setup sheet**, **cycle-time and chip-load lines**, **simulation numbers**,
  the **measure** tool, the **status bar** ("in") and the **canvas grid** (steps of 1/16",
  1/8", 1/4", 1", … instead of 1, 5, 10 mm) follow it.
- Snapping to the grid snaps to the inch grid.
- Angles, percentages, rpm, flutes and seconds are unchanged.

Every field goes through one helper (today `mm_field`), which becomes a length field that
knows the display units; labels and texts go through one `length_text`/`feed_text` pair in
the pipeline, so the setup sheet, the report and the CLI say the same thing.

### 5.2 New projects

With inch display, a new project starts from tidy inch values instead of converted mm ones:
stock 24" × 12" × 0.75", safe height 0.2", and the like. The full list is in the
implementation, each value stated in both systems in the guide. (Decision 4.)

### 5.3 Command line

- Lengths typed as options may carry a unit: `--depth 0.25in`, `--diameter 1/4in`,
  `--at 1in,2in`, `--height 20mm`. Without a unit they're mm, as now, so no script changes.
- Human-readable output stays in mm unless `--units inch` is given. (Decision 5.)

### 5.4 Tests

- Every form drawn with inch display: each length field shows value/25.4 and, edited, stores
  the typed value × 25.4. A field that misses the conversion fails this test.
- Fractions parse; nonsense is refused without changing the value.
- The setup sheet in inch: a golden sheet (`expected/sheets/inch/`).
- A round trip: typing 0.2500 in a field stores 6.35 mm exactly.

## 6. Order of work

1. **G-code units** (§4): schema 28, post, checker, goldens, guide. Small and safety-critical,
   so first and on its own.
2. **Display units in the app** (§5.1, §5.2) and the setup sheet.
3. **Command line** units (§5.3).

Each is its own commit with its tests and guide page, and the app is usable after each.

## 7. Decisions

1. **G-code units: a required field with a schema bump (28)**, written in every project.
   The alternative, an optional field defaulting to mm with no bump, saves a version but
   leaves units implicit in the file, which CLAUDE.md asks us to avoid. *Recommended: bump.*
2. **Inch output: 4 decimals,** with the checker's tolerance widened to half a step in inch
   programs. 4 decimals is what inch controllers and senders expect; 5 decimals would keep
   the tolerance unchanged but look odd and some senders show it badly. *Recommended: 4.*
3. **Fractions:** accepted when typing, always shown as decimals. Showing fractions
   (0.375 → 3/8) would need a rule for values that aren't nice fractions. *Recommended:
   accept, don't show.*
4. **New project defaults in inch** are tidy inch values, not converted mm values.
   *Recommended.*
5. **Command line output** stays mm by default, with `--units inch`, rather than following
   the app's preference: a command's output then doesn't depend on the computer it runs on.
   *Recommended.*

## 8. Out of scope

- Fractional display, feet, and imperial units beyond inches.
- Changing the units inside a program (G20/G21 mid-program) or per operation.
- Converting a project's values to tidy inch numbers (a 6 mm tool stays 0.2362").
