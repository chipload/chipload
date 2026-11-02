# Spec: an ordinary step down for V-carves

Status: **agreed and built 2026-11-02**, all decisions as recommended (the user: "Go
ahead").

As built:
- `carve_levels(depth, passes, step_down)` gives both; the count for a step down is
  ⌈(depth − 0.001 mm) / step⌉, the same in the validation, so a speck over a whole number
  of steps adds no pass.
- In the form, choosing *Step down* starts it at the first pass's depth under equal load,
  rounded to 0.1 mm, so the first pass stays about as deep. Switching back keeps `passes`.
- Golden `v35_vcarve_step_down`: v34_vcarve_step_down with a 1 mm step down, 3.5 mm in four
  passes of 0.875 mm. Every other golden program is unchanged.

The user (2026-11-02): "Now we have progressive step down on v-carve there should be an
option to use ordinary step down as well."

## 1. What there is now

Since 0.29 (schema 35, docs/specs/v-carve.md decision 12) a V-carve has a number of
**passes of equal load**. A V groove's cross-section grows as its depth squared, so pass k
of n goes to D × √(k / n) below the carved surface. 5 mm in 6 passes: 2.04, 2.89, 3.54,
4.08, 4.56, 5 mm. The first pass is the deepest step; each one after it is shallower.

Every other operation (profile, pocket, follow path, surface) has an ordinary **step down**
in mm: equal steps no deeper than it, the last at the end depth (`pass_depths`). 5 mm with
a 2 mm step down: 1.67, 3.33, 5 mm.

## 2. Why both

- **Equal load** keeps the work on the bit the same from pass to pass. It suits deep
  carves with a large V-bit.
- **Equal steps** go down evenly, the way the other operations do. A user who thinks in
  "how deep per pass" for their machine (2 to 2.5 mm in wood on a Shapeoko) sets that step
  and it holds for every pass. The cost is that each pass removes more than the pass
  before: with 3 passes, 1 : 3 : 5.

## 3. Behaviour

- **Depth passes**, a choice in the V-carve form, Cut section:
  - **Equal load** (as now): *Passes*, a count; their depths shown beneath.
  - **Step down**: *Step down*, a length; the passes are equal, none deeper than it, the
    last exactly at the end depth (the same rule as pockets). The depths are shown beneath
    the field too.
- The steps are counted from the start depth (the carved surface) down to the end depth,
  as now. A stroke shallower than a pass finds nothing left to cut in the deeper passes,
  as now.
- Everything else stays the same: area by area, links through the groove, walls and the
  floor pass, smoothing, *Ignore kinks*. Only the list of pass depths differs.
- **Command line:** `chipload set p.chipload operations.0.step_down=2` turns step down on;
  `chipload set p.chipload operations.0.step_down=null` goes back to equal load.

## 4. In the project file

- A new optional field on the V-carve, **`step_down`** (mm). Missing or `null` means
  equal load with `passes`, so every existing project carves as before. That is an optional
  field with a safe default: **no schema bump** (CLAUDE.md).
- `passes` stays and is always written. It is kept while step down is on, so switching
  back finds the count the user had.
- Validation: `step_down` greater than 0, and no more than [`MAX_VCARVE_PASSES`] (100)
  passes: at least (end − start) / 100. A new message, `job-vcarve-step-down`, in the six
  languages.

## 5. Tests

- The levels: 5 mm with a 2 mm step down gives 1.67, 3.33, 5; a step down of the whole
  range or more gives one pass; the last is exactly the end depth.
- A project without `step_down` gives the same G-code as before (the golden V-carves are
  unchanged).
- A golden V-carve with `step_down` (a new corpus project); it passes the checker.
- Validation at the boundaries: 0, negative, exactly the range, the 100-pass limit.
- The form: switching the choice keeps `passes`; the GUI test for the V-carve form.
- The guide: the V-carve page's *Passes* section and the file table; the reference pages
  regenerated.

## 6. Decisions

1. **Step down as a length in mm, not a count of equal steps.** *Recommended:* it is what
   "ordinary step down" means in every other operation, and how the user thinks of their
   machine. Alternative: keep *Passes* and choose equal load or equal steps for them.
2. **Equal load stays the default for new V-carves.** *Recommended:* it is the gentler cut
   and what 0.29 and 0.30 do. Alternative: step down as the default, like the other
   operations.
3. **An optional `step_down` field, no schema bump; `passes` kept beside it.**
   *Recommended:* old projects carve exactly as before, with the least change to the file,
   as for `ignore_kinks_deg`. (An older Chipload refuses a file with the field, an unknown
   field, either way, rather than carve it wrongly.) Alternative: a `pass_depths` choice
   (`"equal_load"` / `"step_down"`) with a schema bump: more explicit in the file, and a
   migration for nothing.
4. **The step down's pass depths follow the pocket's rule** (equal steps, none deeper than
   the step down). *Recommended:* the same numbers as every other operation. Alternative:
   full steps with a shorter last one (5 mm with a 2 mm step: 2, 4, 5).
