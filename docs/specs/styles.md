# Spec: defaults that know the job, and styles

Status: draft 2026-11-03, decisions in §7 to review. Replaces docs/specs/presets.md (paused
2026-10-13), whose layer rules (§5 there) come back as step 4 here.

## 1. Why

The user, 2026-11-03: "prioritize usability improvements and a streamlined, quick and
efficient workflow". The yardstick (direction review, 2026-10-28): for the usual router job,
how many edits from opening the DXF to a program you trust. It was about four per
operation; the aim is none beyond choosing a style.

What a new operation gets today (the window's `editor.rs`, `add_with` and `inherited`):
- a profile 1 mm deep, *Through* off, no tabs, in any stock;
- a selection of an outline with its holes gets one profile outside, marked *side not
  chosen*;
- feeds, step down and stepover copied from the operation above, whatever its tool; the
  tool's cutting data only for the first operation, or when the setup has a material;
- a pocket plunges unless its tool is marked *not* centre-cutting, so with *unknown* every
  new pocket warns "make sure it is a centre-cutting tool";
- on the command line no way to add a profile, pocket, V-carve, path or holes but `set`
  with a whole operation in JSON (rule 5: the window does what the command line can't).

Two parts, one code path:
- **Defaults** (§2): what any new operation gets, from what is selected, the tool and the
  project. Every router job gains, with or without styles.
- **Styles** (§3-§5): your own operations saved by name and applied to a selection in one
  click: "Through 18 mm ply, 6 mm 2F, tabs", "Pocket 6 mm", "Drill 5 mm". CamBam calls
  these machining styles.

## 2. Defaults for a new operation

One function makes new operations, in the pipeline, for the window, `op add` and styles
alike: `chipload_pipeline::new_ops::new_operations(job, kind, select, contours, library) ->
Vec<Operation>` (with the report lines of §2.6). The window's `add_with` keeps only where
the operations go (the active part) and the undo step. Architecture rule 6: no
machining choices in the window.

### 2.1 A profile's side and depth come from what is selected

| Selected (closed contours) | Gets |
|---|---|
| outlines only (contours with nothing around them) | one profile **outside**, **through**, **with tabs** |
| holes only (contours inside an outline) | one profile **inside**, **through**, no tabs |
| both | **two profiles**: the holes inside first, then the outlines outside with tabs, each through and selecting its shapes |
| open shapes only | one profile **on** the line, 1 mm deep, as today |

- **Through** is the project's `through` (job-settings-layout.md §5): the end depth follows
  the stock thickness, below it by the setup's max overcut, as the *Through* box does.
- **Tabs** are the form's tabs today: 4 per contour, 8 mm wide, 3 mm high, the flank from
  the ramp and tool (`Tabs::default_flank`). In stock thinner than 4 mm, the height is
  half the thickness.
- **Two profiles** are named "Profile 2 (holes)" and "Profile 3 (outline)", and cut in that
  order: a hole cut after its part is free would cut a loose part. The *side not chosen*
  mark is left for what can't be told (an outline selected together with an open shape).
- **A layer or a whole DXF** selected with both: the split selects by shapes, so a later
  `update` of the DXF doesn't add new shapes to them; the Report says so once ("selects
  shapes S1-S9: new shapes on layer CUT won't be added"). Selecting outlines only, or holes
  only, keeps the layer.

### 2.2 Depths of the other kinds

- **Pocket, V-carve, follow path:** the depth (or *Through*) of the last operation of the
  same kind in the project; with none, 1 mm as today (a pocket's depth is the drawing's,
  not the stock's; a style holds yours, §3).
- **Holes:** through, as most holes in sheet goods are.
- **Surface:** as today (docs/specs/surfacing.md).

### 2.3 Cutting values come from the tool

Feed, plunge feed, slot feed and slot step down, step down and stepover come from the
tool's cutting data for the operation's material (the setup's; cutting-data.md §6): its
row for the material, else *Any material*. Never from the operation above: that one may
cut with another tool. A tool not from the library has no rows: then the operation above
if it cuts with the same tool, else the values from the diameter as today
(`default_feed`).

### 2.4 Going down with a tool not known to be centre-cutting

A new pocket helixes down and a new follow path ramps when the tool is *not* centre-cutting
(today) **or unknown** (new). The pocket's warning on every run goes; a tool marked
centre-cutting still plunges. Profiles spiral anyway; holes and V-carves go down their own
way.

### 2.5 What isn't changed

- The tool chosen (`tool_for`: the last operation's tool that suits the kind; a drill that
  fits for holes).
- *Machine travel not set* stays a warning: it is set once in the machine profile, and the
  first-run setup (TODO) is where it belongs.
- Projects already saved: nothing in them changes, the G-code neither. No schema change.

### 2.6 On the command line

```
chipload op add part.chipload profile --layer CUT           # also --shapes 3,4,7, --dxf ID, --all
chipload op add part.chipload pocket --shapes 5 --depth 6   # --through, --tool ID
```

Kinds: `profile`, `pocket`, `vcarve`, `path`, `holes` (`op add-surface` stays). Prints the
operations added (one or two, with what §2.1 chose) and `--json` gives their numbers.
`--depth`/`--through` and `--tool` override the defaults; everything else is `set` after.

## 3. Styles: what one holds

A style is one operation saved by name:
- **name**, unique ("Through 18 mm ply");
- **the operation's kind and settings**, as the project stores them: side, direction, entry,
  ramp angle, lead, tabs (count, size, flank), finish, onion skin, corners, hole method,
  pecks, V-carve passes and *Ignore kinks*, rest passes and floor passes (each with its
  tool, §3.1). Not kept: the shapes (`select`), start points, tabs and corners placed by
  hand, the operation's name, part and on/off;
- **the tool** as its library tool's uid (library-in-browser.md §2);
- **the depth:** start depth, and end depth as a number of mm or **through** (with the
  operation's `through` value);
- **no cutting values** (decision 3): feeds, step downs and stepover come from the tool's
  cutting data when the style is used (§2.3), so a better feed in the tool reaches every
  style that uses it, and a style works in any material your tool has a row for.

### 3.1 Saving a style

- **Window:** an operation's right-click menu in the Browser and its ⋯ in Properties: *Save
  as style…* asks for a name; a name in use asks to replace it.
- **Command line:** `chipload style save part.chipload --op 2 --name "Through 18 mm ply"
  [--replace]`.
- **The tool must be from your library** (`library_uid` that the library knows), and so must
  a rest or floor pass's tool. Otherwise it is refused: "tool 2 isn't in your library: save
  it to the library first". The project tool's values may differ from the library's (the
  style takes the library's, as *Update from library* would).
- **Side:** a profile's side is saved as "by the selection" (§2.1) when it is inside or
  outside, "on the line" when it is on (decision 4).
- **Depth:** an operation with *Through* saves through; one without saves its numbers.

### 3.2 Using a style

- **Window**, with shapes selected:
  - the ribbon's *Operation* group gets a **Style** button whose menu lists your styles
    (with their kind's icon) and *Manage styles…* (the Browser's Styles folder);
  - the canvas' right-click menu on a selection: **Apply style ▸**;
  - a style's own right-click menu in the Browser: *Apply to the selection*.
- **Command line:** `chipload style use part.chipload "Through 18 mm ply" (--layer NAME |
  --shapes IDS | --dxf ID | --all)`.
- **What it does:** the operations of §2 for the selection (one, or two for a profile style
  on outlines with holes), then the style's settings over them, in the active part, as one
  undo step. The name is the style's ("Through 18 mm ply", "Through 18 mm ply (holes)"),
  numbered when used again.
- **The tool:** the project tool linked to the style's library tool if there is one, else
  the library tool copied in under the next free number (as `tool use`). A style whose
  library tool is gone is refused by name.
- **Through** follows the stock; a number is used as it is, also deeper than the stock (the
  checks say so as for any operation).
- **No link back** (decision 5): the operation is an ordinary operation after; nothing in
  the project remembers the style, and changing the style later doesn't change it.

### 3.3 Managing styles

- **Browser:** *This computer* gets a **Styles** folder beside Tools, Machines and
  Materials. A style's properties show its kind, tool, depth and a summary of its settings
  ("outside by selection, climb, through + 0.3 mm, 4 tabs 8 × 3 mm"), with *Rename*,
  *Delete* and the Delete key. A style isn't edited in a form of its own (decision 6): use
  it, change the operation, *Save as style…* with the same name.
- **Command line:** `chipload style list [--json]`, `style show NAME`, `style rename NAME
  NEW`, `style remove NAME`, `style export FILE [--name …]`, `style import FILE
  [--replace]`.
- **Importing** a style whose tool isn't in this library keeps it, marked *tool missing*,
  and refuses to use it until a tool is chosen (`style set NAME --tool "6 mm 2F"`, or
  *Tool* in its properties).

## 4. Where styles live: `styles.json`

A new persisted file in the settings folder, beside `tools.json`:

```json
{
  "schema_version": 1,
  "written_by": "chipload 0.33.0",
  "operation_schema": 35,
  "styles": [
    {
      "uid": "…",
      "name": "Through 18 mm ply",
      "tool": { "uid": "…", "name": "6 mm 2F upcut" },
      "side": "by_selection",
      "operation": { "type": "profile", … }
    }
  ]
}
```

- **The operation is stored as a project stores it** (cutting values and shapes left at
  neutral values and ignored), so every operation setting, now and later, works in styles
  without code of its own.
- **Migrations:** when the project schema changes an operation, the file is read through
  the job migrations, each style's operation wrapped in a minimal project; then
  `operation_schema` is raised and the file saved with a `.bak`. No second set of
  migrations. A style that won't migrate is refused by name and the others load.
- **A malformed or newer file** is a typed error: Chipload carries on without styles and
  says so, as for the tool library.
- **The tool's name** is kept beside its uid, only to name a missing tool after an import.
- A frozen v1 sample in `tests/corpus/styles/v1/`.

## 5. Later steps, not in this spec's build

- **Keys for styles** and *select similar* (TODO, fewer clicks).
- **Layer rules:** a style with a layer pattern applied on import or on demand
  (presets.md §5 and its decisions 6-8, reviewed when that step comes).
- **Styles of several operations** (a profile and its pocket as one style).

## 6. Tests

- **Defaults:** outlines only, holes only, both (two operations, holes first, by shapes,
  the Report line for a layer), open shapes; through follows the thickness; tabs in thin
  stock; depths from the last of a kind; holes through; cutting values from the tool's row
  for the material, not from the operation above with another tool; unknown centre-cutting
  helixes; G-code of every corpus project unchanged.
- **`op add`:** each kind, each selector, `--depth`, `--through`, `--tool`, `--json`.
- **Styles:** save refuses a tool not in the library (also a pass's); through and numbers;
  what isn't kept (shapes, starts, placed tabs); use copies a missing tool in, reuses a
  linked one, refuses a style whose tool is gone; a profile style on a mix gives two;
  cutting values from the tool's row; a style used and the operation it was saved from give
  the same G-code on the same shapes; one undo step.
- **File:** round trip, the v1 sample, an older `operation_schema` migrating, a newer file
  refused, one broken style refused while the others load, export and import, a missing
  tool marked.
- **Window:** the Style button and the canvas menu, *Save as style…*, the Styles folder,
  Delete; layout tests in every language.
- **Golden:** one project made only with `op add` and `style use` from a corpus DXF.

## 7. Decisions to review

1. **One function for new operations, in the pipeline** (§2), used by the window, `op add`
   and styles. *Recommended:* the defaults are machining choices (rule 6), and styles must
   start from the same operations the window makes.
2. **A profile's side, depth and tabs from the selection** (§2.1): outlines outside and
   through with tabs, holes inside and through, a mix as two profiles, holes first.
   *Recommended:* the usual cut-out job in one click, and the order that keeps a part from
   coming loose before its holes are cut. Alternatives: no tabs by default; holes with tabs too (a loose slug can be caught by the bit; with
   a 6 mm bit and small holes the slug is small); a mix as one profile per side choice left
   to you, as today.
3. **Styles hold no feeds or step downs** (§3); they come from the tool's cutting data.
   *Recommended:* one place for the numbers (ADR 0029, "it feels a bit split"), and a
   style works in every material. Alternative: a style keeps its operation's feeds too,
   as presets.md had it, which brings back two places for the same numbers.
4. **A profile style's side is "by the selection"** unless saved *on* the line.
   *Recommended:* one "Through 18 mm ply" style for outlines and holes alike. Alternative:
   the side saved as set, so an outside style applied to a hole cuts it outside.
5. **No link from an operation to its style.** *Recommended:* simple and predictable; a
   proven project cuts as it did (as tool copies, ADR 0014). Alternative: operations
   remember their style and offer *Update from style*, as tools do from the library.
6. **No style form; save again to change one** (presets.md decision 2). *Recommended:* the
   operation form already exists and is tested. Alternative: a full style editor.
7. **The name "style"**, CamBam's word for it, in every language (Stil, style, estilo,
   stile, stil, стиль). *Recommended:* short, and what a CamBam user looks for.
   Alternative: "preset".
8. **`styles.json` beside the tool library, read through the job migrations** (§4).
   *Recommended*, as presets.md decision 5. Alternative: inside `tools.json`, under each
   tool, which would tie a style to one tool.
9. **Holes through by default** (§2.2). *Recommended:* holes in sheet goods mostly go
   through; a blind hole is one edit, or a style. Alternative: 1 mm as today.
10. **`op add` on the command line** (§2.6), before any window change. *Recommended:* rule
    5, and the golden test in §6 needs it.

## 8. Steps

1. **Defaults:** `new_operations` in the pipeline with `op add`, the window's `add_with`
   on it; guide pages for the operations (*new operations start …*).
2. **Styles, file and command line:** `chipload_job::styles` (`styles.json` v1, the
   sample), `style save/use/list/show/rename/remove/export/import/set`.
3. **Styles in the window:** the ribbon's Style button, the canvas menu, *Save as style…*,
   the Styles folder; a new guide page *Styles* under the library.
4. **Later:** layer rules, keys (§5).
