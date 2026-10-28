# Spec: Job settings, clearer

Status: **approved 2026-10-28** for §2–§4 (the user's answers: "1 yes, 2 no, keep drawing
origin, 3 yes, 4 no, if so we should have a checkbox in the operation that says 'all
through' or something, then it will follow stock thickness, 5 yes"). §2 and §3 **built
2026-10-28**. §5 is a **proposal**, waiting for the user.

## 1. Why

Looking at the panel (2026-10-28), one *Stock* section of nine rows did three jobs:
- the board: fit the drawing, size, the drawing's size, thickness, position;
- the zero, which the operator needs at the machine (work zero X Y, Z zero), last, after
  *Max depth below stock*;
- feeds: *Material*, between *Drawing* and *Thickness*, though choosing a material no longer
  touches the stock (docs/specs/materials.md §2).

And:
- *Fit the drawing* and *Stock position* were two controls for one choice: "centred on the
  drawing" is the fit's position, and another position turned the fit off unseen.
- The machine profile was chosen on the last row of *Machine*, below eight values that are
  set once.
- *Max depth below stock* is about the spoilboard, not the board on it.
- The two sections' label columns had different widths.

The small faults (travel past the panel's edge, report texts naming old menu items) were
fixed first (87eea19).

## 2. The layout

Four sections, in the order a job is set up. Nothing in the project file changes: this is
the window only.

```
Job settings
▾ Stock        300 × 220 × 18 mm
   Thickness    [18 mm]
   Size         (•) Outline + margin (100 × 60 mm)
                    Margin [10 mm]
                ( ) Fixed size, centred
                    [300] × [220]
▾ Zero         X0 Y0 drawing origin · Z0 stock top
   X Y zero     [drawing origin ▾]
   Z zero       [stock top ▾]
▾ Material     Plywood
   Material     [Plywood ▾]
                Each operation can choose its own, under its tool.
▸ Machine      Shapeoko · grbl · safe Z 5 mm · M3
   Library      [Use profile… ▾] [Save to library…]
                (how the machine stands to its profile)
   Safe Z       [5 mm]
   Max depth below stock [0.5 mm]
   Post, G-code units, spindle, max feed, tool changes, travel
```

- **Stock:**
  - *Thickness* first: every job needs it.
  - *Size* is one choice of two, both centred on the drawing's outline (the user,
    2026-10-28: "Stock position is mostly not needed now … allow fixed stock size
    (centered over project outline) or 'Stock margin' that is project outline + margin"):
    *Outline + margin* (the drawing's size in brackets, the margin under it), or *Fixed
    size, centred* (X and Y). *Fixed size* keeps the size the stock had.
  - **No position list** for a new choice. A project whose stock sits at the drawing's
    origin (corner or centre, `stock.origin` `bottom-left` or `center`) keeps that place,
    with the list shown under its size, until it is set to centred there; so its stock and
    a work zero on a stock corner never move unseen. The file and the command line keep
    the three positions. Dropping the other two from the format is a later decision with a
    migration (it moves the stock).
  - The separate *Drawing* row goes: its size is in the *Fit* line. With *Fixed size* and a
    drawing, it shows as a weak line under the fields, so a board can be checked against
    it.
- **Zero:** its own section. The summary line says where X0 Y0 and Z0 are, so it reads with
  the section closed. The default stays the drawing origin (decision 2: no).
- **Material:** its own section, only with a settings folder. Same list and behaviour as
  before (every operation and those added later take it); a weak line says each operation
  can choose its own.
- **Machine:**
  - **Closed by default**: set once per machine. Its summary gives the profile's name when
    linked, then post, safe Z and spindle.
  - The library rows come first: choosing a profile is the first thing done here. A linked
    machine, the same as its profile, shows its values as text with *Edit here*, as today.
  - *Safe Z* and *Max depth below stock* follow; then the machine's values.
- **Columns:** every section's grid has the same label column, as wide as the widest label in
  the four, so fields line up down the panel (within the panel's 40 % cap).

## 3. A default material

As a default machine and a default tool: `materials.json` gets an optional
`default_material` (a name; optional with a safe default, so no version bump).
- A new project (New, Import) starts with it: the project's material, and every operation
  an import made takes its values, as *Use for all operations*. The report says "Started with
  material Plywood (default)".
- **Window:** *Make default* on a material's form, "default" beside it in the folder.
- **CLI:** `chipload material default NAME`, `--none` to clear it. `material list` marks
  it.
- Renaming the material keeps it the default; removing it clears the default.

## 4. Tests

- **Layout:** the six languages fit the default and a narrow window (layout tests); the
  Fit and Fixed choices switch as the checkbox did (size kept); the Zero summary line.
- **Default material:** saved and read back; a new project and an import start with it;
  rename and removal; CLI `material default` with `--json`.

## 5. Proposal, not approved: cuts that follow the thickness

The user (decision 4): typing a new thickness should not move depths, "if so we should have
a checkbox in the operation that says 'all through' or something, then it will follow stock
thickness".

- **Profile, pocket, follow path and holes** get a **Through** checkbox beside *End depth*,
  in place of today's *Through* button. Ticked, the end depth is the stock's thickness plus
  how far below it the cut goes, and follows the thickness; the field shows it, read-only.
- **The project file:** each of those operations gets `through`: null, or the depth below the
  stock, mm (e.g. 0.3). Optional with a safe default (null: the end depth as typed), so no
  schema bump. The end depth is still written, kept equal to thickness + `through`, so
  older code and the checker read the depth that is cut.
- **Where it is kept equal:** in one function of `chipload-job`, run when a project is
  checked (load, every edit in the window, `set`, `run`, `plan`), so no path can leave a
  stale end depth.

**Decisions:**
1. **How far below the stock?** **Recommended:** its own amount per operation, starting at
   *Max depth below stock* (today's *Through* button), shown beside the box: "Through,
   0.3 mm below". Alternative: always *Max depth below stock*, one number for the project.
2. **Which operations?** **Recommended:** profile, pocket, follow path and holes. A V-carve
   and a surface never go through.
3. **Ticked by default for a new profile?** **Recommended: no**, as today's depths; tick it
   where you mean it. Alternative: a new outside profile starts ticked.
