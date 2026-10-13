# Spec: operation presets and layer rules

Status: **draft 2026-10-13**, for review (decisions in §10).

Approved plan (2026-10-13, ADR 0017): "layer rules: operation presets keyed to DXF layers",
the first leverage feature. It multiplies what exists (operations, the tool library) instead
of adding a new kind of thing.

## 1. Two drawing habits, one feature (decision 1)

- **A layer per operation** ("OUTSIDE", "POCKET", "DRILL_5"): common in exports from CAD
  set up for CNC. Here the layer says what to do, so the operations can be made on import
  without a click.
- **A layer per part** (how your own drawings are made: `upper`, `lower`, layer `0` for a
  shared outline). The layer says nothing about pocket or through cut, and you set the
  operations and their order yourself. Automatic rules don't help here; a saved operation
  applied to a selection does.

So the feature is a **preset**: a saved operation (type, library tool, depths, every
setting)
- applied by hand to the canvas selection ("+ From preset"); and,
- if it has a **layer pattern**, applied automatically to matching layers on import, or on
  demand with "Apply layer rules".

One list, one file, both habits.

## 2. What a preset holds

- A **name** ("Through 12 mm ply", "Pocket 6 mm").
- One **operation** as the project file has it (profile, pocket, V-carve, follow path or
  holes), with every setting: side, direction, step down, feeds, stepover, entry, lead,
  tabs, finish, skin, corners, hole method. Not kept: the geometry (`select`) and the start
  points (`starts`, drawing coordinates that mean nothing in another drawing).
- The **tool by its library name**, not a project id.
- The **end depth** as either a number of mm, or **through**, meaning "the stock thickness
  plus N mm" (decision 3). The start depth is always a number.
- An optional **layer pattern** (§5).
- **Side "auto"** for profiles (decision 4).

## 3. Making a preset: save it from an operation (decision 2)

You never fill in a preset form. You set up an operation as you want it in a project, then:
- **GUI:** the operation's ⋯ menu > *Save as preset…*, which asks for a name and, optionally,
  a layer pattern. The pattern defaults to the operation's layer when it selects one.
- **CLI:** `chipload preset save part.chipload --op 2 --name "Through 12 mm ply"
  [--layer "CUT*"] [--through | --depth] [--replace]`.

When saving:
- **The tool must be in the library with the same values.** If the project tool has no
  library name, or the library's tool of that name has other values, the save is refused
  with "save tool 2 to your library first (`tool save`)". Presets never change the library
  behind your back.
- **End depth:** at or below the stock bottom it becomes **through** by the difference
  ("through + 0.3 mm"), otherwise a number. `--through`/`--depth` (GUI: a choice in the
  dialog) override this.
- **Side:** saved as the operation has it. *Auto* is chosen in the dialog or with `--side
  auto`.
- **A name already in use** is refused unless `--replace` is given (the GUI asks).

## 4. Applying a preset to the selection

- **GUI:** next to the "+ Profile … + Holes" buttons, a **"+ From preset ▾"** menu lists the
  presets. Picking one adds its operation for the canvas selection, exactly where "+
  Profile" would add it (at the end; nothing is reordered).
- **CLI:** `chipload preset use "Through 12 mm ply" part.chipload (--layer NAME | --shapes
  3,4,7)`.

Applying a preset:
- **The tool:**
  - if the project has a tool with the preset's library name and the same values, that one
    is used;
  - otherwise the library tool is copied in under the next free id, as `tool use` does;
  - if the library has no tool by that name, the preset is refused and named.
- **The depth:** through means `stock thickness + N`, worked out now. The operation stores
  a plain number, as today, so the project format doesn't change. A later change to the
  stock thickness doesn't move it.
- **Auto side** (decision 4):
  - holes only gives one inside operation;
  - outers only gives one outside operation;
  - a mix gives **two operations**: inside on the holes first, then outside on the outers.
    The holes are cut before the outline frees the part. They select by shapes.
- **Name:**
  - the preset's name;
  - with the layer, "Through 12 mm ply: CUT", when applied by a layer rule;
  - with "(holes)" or "(outside)" for the two halves of an auto side.

## 5. Layer rules

- **Pattern:** a layer name with `*` (any run of characters) and `?` (one character),
  case-insensitive, because DXF layer names are and Chipload's layer selection already is.
  `CUT*` matches `cut_outside` and `Cut`. No regular expressions.
- **Rule order:** the order of the preset list.
  - Every preset whose pattern matches a layer applies to it, so a layer can get a rough
    pass and a finish pass from two presets.
  - The operations are added in list order, and by layer name within one preset.
  - The list order is therefore the cut order of what the rules add. Reorder the list to
    change it.
- **Which layers:** the layers of the drawing's shapes, imported and drawn.
  - Closed-contour presets (profile, pocket, V-carve) skip a layer with no closed contours,
    with a note.
  - Holes presets skip a layer with no circles.
  - Follow path takes open and closed contours.
- **Selector:** the operation selects **the layer** (`select.layer`), so updating the
  drawing keeps it in step with the layer, as a layer selection does today. Auto side with
  a mix is the exception and selects shapes (decision 4).
- **Already handled:** a layer that any operation already cuts, by layer or by one of its
  shapes, is skipped with a note ("layer CUT already has operations 1, 3"). So applying
  twice adds nothing.
- **When:**
  - **On import** (`chipload import`, *File > Import DXF*), when the user has any rule.
    `--no-rules` on the CLI skips them. In the GUI, the report says what was added, and one
    Undo removes it.
  - **On demand:** *Operations > Apply layer rules*, or `chipload preset apply
    part.chipload [--dry-run]`. This is for a project made before the rules, or after
    `update` brought new layers.
  - **Not on update** (decision 6). `update` adds a line naming new layers that match a
    rule: "layer DRILL_5 is new and matches preset 'Drill 5': apply layer rules to add
    it".
- **The report:** one line per operation added, one per layer skipped and why, and the
  layers no rule matched ("no rule for layers 0, DIMENSIONS"). These are the same lines on
  the CLI and in the GUI.

## 6. Where presets live: `presets.json` (decision 5)

A new persisted file in the settings folder, next to `tools.json`:
- `schema_version` 1;
- `written_by`;
- `operation_schema`: the project schema its operations were written in;
- `presets: [{ name, layer, tool, end: {depth: 6.0} | {through: 0.3}, side_auto, operation
  }]`.

Rules:
- **Operations follow the project format.** A preset's operation is stored exactly as in a
  project, so every operation setting there is, or will be, works in presets without its
  own code.
- **Migrations:** when the project schema changes an operation, the presets file is read
  through the same job migrations, wrapped in a minimal job. Then `operation_schema` is
  raised and the file saved. No second set of migrations is kept. A preset that won't
  migrate is refused by name, and the others still load.
- **Problems:** a malformed or newer file is a typed error, and Chipload carries on without
  presets, as for the tool library.
- **Export and import:** `preset export FILE [--name …]` and `preset import FILE
  [--replace]`, the same as for tools. This shares a shop's presets. A preset's tool must
  be in the importing library too, and import names any preset whose tool isn't.

## 7. Managing presets

- **CLI:**
  - `chipload preset list [--json]`: the list order, the layer pattern, and a one-line
    summary ("profile, outside, climb, tool '6 mm 2F', through + 0.3, tabs 4");
  - `preset remove NAME`;
  - `preset move NAME --to N`;
  - `preset set NAME [--layer PATTERN | --no-layer] [--rename NEW]`;
  - `preset save … --replace` changes the operation.
- **GUI:** *Settings > Presets…* opens a window with the list. It can:
  - reorder (up and down);
  - rename;
  - set or clear the layer pattern;
  - delete;
  - import and export.

  The operation itself isn't edited there. To change one, use the preset on a shape, edit
  the operation, and save it again with *Replace*. That is one form instead of two.

## 8. Out of scope

- **Depth in the layer name** (`POCKET_6` gives 6 mm; decision 7).
- **Presets of several operations as one unit.** Several presets matching one layer do
  this already.
- **Material-dependent values.** The feeds-and-speeds-per-material spec comes next, and
  decides whether a preset can take its feeds from the tool and material instead of fixed
  numbers.

## 9. Tests

- **Patterns:** `*`, `?`, case, an empty pattern refused, and characters with no special
  meaning (`.`, `[`) taken literally.
- **Save:**
  - a tool missing from the library, or with other values, is refused;
  - through is found below the stock bottom;
  - `select` and `starts` are not stored;
  - a name already in use is refused without `--replace`.
- **Use:**
  - the tool is reused when its name and values are the same, and copied otherwise;
  - through gives the thickness plus N;
  - auto side gives one operation or two (holes first);
  - a preset applied to shapes cuts the same as the operation it was saved from, compared
    by G-code.
- **Layer rules:**
  - several presets on one layer, in list order;
  - a handled layer is skipped, so applying twice adds nothing;
  - layers with nothing to cut are skipped with a note, and unmatched layers are listed;
  - `--dry-run` writes nothing;
  - `import --no-rules`;
  - `update` reports new matching layers and adds nothing.
- **File:**
  - round trip;
  - a preset written at an older `operation_schema` migrates through the job migrations;
  - a newer file is refused;
  - a bad preset is refused by name while the others load;
  - export and import.
- **Golden:** a new corpus DXF with layers per operation, plus a presets file, run through
  `import` to G-code. That gives one new golden. Existing goldens don't change.
- **GUI:**
  - *+ From preset* adds the operation, and Undo removes it;
  - *Save as preset…* round trip;
  - import with rules shows the report, and one Undo removes what was added.

## 10. Decisions

1. **One feature for both habits:** presets you apply by hand, with an optional layer
   pattern that makes them automatic. **Recommended**: your layer-per-part drawings gain
   from presets by hand; layer-per-operation drawings gain the automatic part. Alternative:
   layer rules only, which wouldn't help your own drawings.
2. **Presets are made only by saving an operation**, never in a form of their own.
   **Recommended**: the operation editor already exists and is tested; a second editor
   would double the form code. Alternative: a full editor in the Presets window.
3. **End depth "through + N mm"**, worked out against the stock when the preset is applied,
   and stored in the operation as a plain number. **Recommended**: one preset fits 12 and
   18 mm board, and the project format doesn't change. Alternative: a "through" depth in
   the project too, which would follow later stock changes but needs a schema bump and
   touches every planner.
4. **Side "auto"**: holes inside, outers outside, two operations (holes first, by shapes)
   when a selection or layer has both. **Recommended**: the common "one layer, part with
   holes" case in one step. Alternative: a fixed side only; or a new selector filter
   (`role: holes | outers`) that keeps both operations selecting the layer, which is a
   format change and touches selection in the planner.
5. **`presets.json`**, separate from the tool library, read through the job migrations.
   **Recommended**. Alternative: inside `tools.json`, which would mix two kinds of thing in
   one file and make the export/import story murkier.
6. **Rules apply on import and on demand, never by themselves on update.** **Recommended**:
   an update that silently adds operations would change what a saved project cuts.
   Alternative: also on update, for new layers only.
7. **No depth in layer names for now.** **Recommended** per ADR 0017: none of your drawings
   use it, and it is easy to add if a user asks. Alternative: `{depth}` in the pattern
   (`POCKET_{depth}`), in mm.
8. **Rule order is list order, and every matching preset applies.** **Recommended**: you
   set the order, as you do for operations. Alternative: first match only (simpler, but
   rough and finish on one layer would need a second pattern).
