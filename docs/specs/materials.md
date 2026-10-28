# Spec: materials, with your own feeds

Status: **built 2026-10-28 with the recommended decisions**, for the user to try ("It
sounds reasonable, I must really try it out to know"). See §10 for where it differs from
the draft.

## Why

Today every operation's feeds and step down are typed by hand, or come from a library
tool's single set of values (`Cutting`), whatever the material. The direction review
(docs/reviews/2026-10-28-direction.md §7, step 3) puts materials next, before styles
(docs/specs/presets.md): first how fast, then what to do.

The user, 2026-10-28:
- On calculating feeds from tooling manufacturers' charts: "DIY CNC routers are on another
  level. My simple Shapeoko, I often use 2–2.5 mm step down and perhaps 1500 mm/min in
  wood."
- On the design: "yes it should be simple".

So a material holds **your own proven numbers** for the tools you use, typed in once or
saved from an operation that cut well. There are no formulas, no chip-load calculation and
no machine factor.

## 1. What a material is

A library entry with:
- a **name** ("Plywood", "Oak", "MDF"), unique in the library (case doesn't matter);
- a **uid** (16 hex digits, never changed), as tools and machines have
  (docs/specs/library-in-browser.md §2);
- a **thickness**, mm, optional: the board you usually buy;
- **values per tool**: for each library tool (by its uid), the same four values a tool
  carries today (`Cutting`):
  - feed, mm/min;
  - plunge feed, mm/min;
  - step down, mm;
  - stepover, a fraction of the tool diameter.

A material doesn't need values for every tool, only for the tools you use with it.

There is no link to a machine (decision 2). With a second machine you'd make "Plywood (big
router)" beside "Plywood".

## 2. Choosing a material for a project

**Job settings > Stock > Material:** a list of *None* and the library's materials, above
*Thickness*. CLI: `chipload material use part.chipload Plywood` (`--none` to clear it).

Choosing one, as one undo step:
- **Thickness:** if the material has one, the stock takes it.
- **Existing operations:** each operation whose tool has values in the material takes them
  (decision 3). The values an operation has are:
  - feed and plunge feed, for every operation;
  - step down, where the operation has one;
  - stepover, for pockets only. A surface keeps its own wide stepover (surfacing.md §4).
- **Tools without values:** an operation whose tool has none in the material, or whose tool
  isn't linked to the library, keeps its values. The report names it: "Operation 3 (Pocket):
  tool 2 has no values for Plywood; its feeds are unchanged".
- **The report** lists what changed, one line per operation, as other bulk edits do.

The project remembers the material (§6), so:
- **New operations:** an operation added later takes the material's values for its tool.
  It falls back to the tool's own values (`Cutting`), then to today's defaults.
- **Changing an operation's tool** takes the new tool's values the same way.

## 3. Saving values from an operation

An operation that cut well is where good numbers come from, so they can be saved straight
back:
- **GUI:** under the operation's feeds, one line says where they come from:
  - *From Plywood* when they match the material's values for its tool;
  - otherwise *Changed from Plywood* (or *Not in Plywood* when the tool has none there),
    with a **Save to Plywood** button.
- **CLI:** `chipload material save part.chipload --op 2` saves to the project's material;
  `--material NAME` saves to another one.

Saving:
- writes that tool's four values in the material, replacing any it had;
- needs the tool to be in the library: a tool that isn't is refused with "save tool 2 to
  your library first (`tool save`)", as for presets;
- needs a material: with none chosen, the GUI's button offers *New material…* (a name, and
  the stock thickness as its thickness).

## 4. Managing materials

- **GUI:** Library > **Materials** in the Browser, as for tools and machines. A material's
  form has its name, thickness, and a table: one row per library tool with values, plus
  *Add tool* to type values for another library tool. A row can be removed.
- **CLI:**
  - `chipload material list [--json]`;
  - `chipload material show NAME`;
  - `chipload material add NAME [--thickness 18]`;
  - `chipload material set NAME --tool "6 mm 2-flute" --feed 1500 --plunge 500
    --step-down 2.5 --stepover 0.4`;
  - `chipload material remove NAME [--tool NAME]` (the material, or one tool's values);
  - `chipload material export FILE` and `material import FILE [--replace]`, as for tools
    (docs/specs/library-import-export.md).
- **Removing a library tool** removes its values from every material. The report names the
  materials.

## 5. No built-in materials (decision 4)

The library starts with **no materials**. Any number shipped would come from charts the
user doesn't trust for a hobby router, or be a guess. The first material is made by:
- *Save to …* from an operation that cut well (§3); or
- typing the values in Library > Materials.

The guide's materials page gives the user's Shapeoko numbers (wood, 6 mm 2-flute: about
1500 mm/min, 2–2.5 mm step down) as an example of what to start from on a light machine,
not as a default.

## 6. Files

- **`materials.json`** in the settings folder, beside `tools.json` and `machines.json`
  (decision 5):

  ```json
  {
    "schema_version": 1,
    "written_by": "chipload 0.28.0",
    "materials": [{
      "name": "Plywood",
      "uid": "3f9c0a1b2d4e5f60",
      "thickness": 18.0,
      "tools": [{
        "tool_uid": "a1b2c3d4e5f60718",
        "feed": 1500.0, "plunge_feed": 500.0, "step_down": 2.5, "stepover": 0.4
      }]
    }]
  }
  ```

  - Loaded through one function (`load_materials`): a missing file is an empty library; a
    malformed or newer file is a typed error, and Chipload carries on without materials, as
    for the tool library.
  - Values are checked as an operation's are: feeds above 0, step down above 0, stepover
    more than 0 and at most 1. A material with a bad value is refused by name; the others
    load.
- **The project:** `stock.material`, optional: `{ "name": "Plywood", "library_uid": "…" }`,
  or null.
  - It is an optional field with a safe default (none), so there is no schema bump (CLAUDE.md,
    persisted formats).
  - Only the name and uid are kept, not the values: the values that matter are already in
    the operations. A project opened on a computer without that material shows its name
    with "not in your library", and new operations fall back to the tool's values.
  - The setup sheet gives the material's name beside the stock.

## 7. What doesn't change

- The G-code of every existing project and every golden. Nothing is applied without the user
  choosing a material or saving values.
- A library tool's own values (`Cutting`) stay: they are what an operation gets with no
  material (decision 6).
- The checker, the chip-load note and the machine's max feed work on the operation's values
  as before.

## 8. Tests

- **Library:** `materials.json` round trip; a missing file is an empty library; newer and
  malformed files are refused; a bad material is refused by name while the others load.
- **Choosing:** the thickness is taken; operations with values change; operations without
  keep theirs, with the report line; a surface's stepover is kept; one undo step.
- **New operation and tool change:** they take the material's values, falling back to the
  tool's own values.
- **Saving:** values written for the right tool uid; refused for a tool that isn't in the
  library; *From* / *Changed from* / *Not in* states.
- **CLI:** each `material` command, with `--json`.
- **Removing a library tool** removes its values from every material.
- **Project:** `stock.material` round trip; an old project loads without it; goldens
  unchanged.
- **Window:** the six languages fit (layout tests), as for every new form.

## 9. Decisions

1. **A material is your own numbers per tool: feed, plunge, step down and stepover.** No
   formulas or chip-load calculation. **Recommended**, as you said: manufacturers' charts
   don't fit a hobby router. Alternative: compute from a chip load, which you rejected.
2. **No link to a machine.** **Recommended**: you have one router; a second machine gets
   its own named materials. Alternative: values per tool *and* machine, which doubles the
   table for everyone, for a case you don't have yet.
3. **Choosing a material updates the existing operations' feeds**, in one undo step, with a
   line per change. **Recommended**: choosing Plywood should mean the job cuts at
   plywood's numbers. Alternative: only new operations take them, and existing ones stay
   as they are, which is safer but leaves you to edit each one.
4. **No built-in materials**, an example in the guide only. **Recommended**: any shipped
   number would be a guess for your machine. Alternative: one or two cautious examples
   ("Softwood", "Plywood") with your Shapeoko numbers.
5. **`materials.json`, a separate file**, like `tools.json` and `machines.json`.
   **Recommended**: each library file holds one kind of thing, and has its own import and
   export. Alternative: materials inside `tools.json`.
6. **A tool's own values stay**, as the fallback when there is no material. **Recommended**:
   nothing you set up today is lost. Alternative: move them into a material called
   "Default" and remove them from the tool (one place for feeds), which is a library
   migration.
7. **Spindle speed stays on the tool.** Your router's speed is set on its dial. **Recommended**.
   Alternative: an optional rpm per material and tool, for materials that need a different
   speed (acrylic, aluminium).
8. **Saving from an operation is the main way to make values** (*Save to Plywood*), with the
   Library form for typing them. **Recommended**: the numbers you trust are the ones that
   just cut well. Alternative: the Library form only.

## 10. As built (2026-10-28)

- **Files:** `chipload_job::materials` (`MaterialLibrary`, `Material`, `MaterialTool`;
  export and import through the tool library's `Collection`), `chipload_pipeline::materials`
  (`use_material`, `material_values`, `material_state`, `values_to_save`),
  `chipload material …` on the command line, `app/materials.rs` in the window. The JSON
  Schema is `chipload schema materials`, the reference page `materials-file.md`.
- **Differences from the draft above:**
  - **Through cuts stay through.** When the material changes the stock thickness, an
    operation whose end depth was at or below the old stock bottom gets the same depth
    below the new one, with a report line. Found trying it: the badge's outline, through a
    12 mm board, would have stopped 6 mm short in 18 mm plywood. A decision for the user
    to confirm.
  - **Changing an operation's tool keeps its feeds** (as docs/specs/tool-per-operation.md
    decision 6 already says), and the Material row then offers *Use Plywood's*. The draft
    said the new tool's values would be taken.
  - **A bad value refuses the whole file**, as for the tool library, not only that
    material.
  - **Import and export are on the command line only** for now; the window's Materials
    folder has *New material*, the form, *Use in project* and *Delete*.
  - The tool's own *Use the tool's values* button shows only when the material has no
    values for the tool.

