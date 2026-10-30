# Spec: let the simulation show material left

Status: **approved 2026-10-13** ("Go ahead"), all decisions as recommended (§5).
Built 2026-10-13. As built:
- The missed spots are still found as squares inside the pocket planner. The ring-corner
  rounding uses them to round less where it would leave material; they're just no longer
  returned for display.
- The dogbone legs were returned only to take them out of the display; the planner no longer
  returns them.
- `left.rs` keeps what rest machining needs (`reach`, `left_by`, `rest_areas`, `cleared`
  and the boolean helpers), and its tests now run through `left_by`.
- Golden setup sheets changed as §4 says: only the "can't reach" lines went; the rest
  pocket's sheet now says "Nothing was reported."

Approved plan (2026-10-13, ADR 0017), item 5: "merge the material-left display into the
simulation, keep the check". Two features now answer the same question, "what does the tool
leave?":

- **Material left** (docs/specs/material-left.md, 2026-10-09). Computed geometrically for
  every pocket and profile, with offsets and booleans:
  - *unreachable* material (inside corners, narrow slots), shaded faintly;
  - *missed* material (rings too far apart), shaded red, with a warning.

  It also gives a line per operation ("the tool can't reach material in 4 places, largest
  about 2.75 × 2.75 mm") in the report, the CLI, the setup sheet and `plan --json`
  (`material_left`).
- **The simulation** (docs/specs/simulation.md, 2026-10-13) cuts the stock with the real
  moves and tool shapes. It shows everything left, whatever the cause: corners, slots,
  missed rings, wrong depths, a tool too short in a V-carve.

The geometric display is the fragile one. Its booleans failed on a random pocket in the
property tests until today's fix, and it has needed special cases: dogbones, crossing
pockets, clearance, and now rest machining.

## 1. What goes

- **GUI:** the material-left shading, its hover tooltip, and *View > Show material left*.
- **Toolpath:** the *unreachable* computation for display:
  - pockets, profiles inside and outside, rest pockets;
  - the dogbone and crossing-pocket special cases;
  - `MaterialLeft` / `LeftPiece` and `Plan.material_left`.

  About half of `left.rs` and its display tests go.
- **Text:** the line "the tool can't reach material in N places, largest about …" in the
  report, `plan`, `run` and the setup sheet's *Watch out*.
- **JSON:** `plan --json`'s `material_left` key. It is not released yet, so no script
  depends on it; `chipload_json` stays 1.

## 2. What stays

- **The missed-material check** (pocket.md §3): the quadtree check of exact distances, and
  its warning "uncut material in N places, largest about …". That's a toolpath that
  doesn't do what it should, which is worth a warning. Only its red shading goes.
- **The narrow-part warning:** "the tool does not fit through a narrow part …".
- **Rest machining's geometry:** `left_by`, `rest_areas` and `reach` compute what a larger
  tool leaves, to plan the rest pocket. That is toolpath input, not display, and keeps its
  tests.
- **The simulation**, unchanged, plus decision 2.

## 3. What changes for the user

- **Seeing it:** to see what the tools leave, turn on *View > Show simulation*, or leave it
  on (decision 2). Corners show as the tool's radius in the stock, in top view and in 3D,
  and the playback shows when they're cut.
- **Fewer lines:** the report, the CLI output and the setup sheet lose the per-operation
  "can't reach" line. Missed material still gives its warning.
- **The guide:** the pages on pockets, rest machining, problems, the simulation and the
  setup sheet are updated in the same commit.

## 4. Tests and goldens

- **Removed:** the display tests of `left.rs`, the property test
  `material_left_is_out_of_reach_and_complete`, the GUI test of the toggle, and the CLI's
  `material_left` JSON check.
- **Kept or added:** the missed-material warning still fires (pocket tests), rest machining's
  geometry and its simulation test stay, and `plan --json` has no `material_left`.
- **Golden G-code:** unchanged, since the display never touched toolpaths.
- **Golden setup sheets:** change, because the "can't reach" lines go from *Watch out*. The
  commit explains the diff.

## 5. Decisions

1. **Remove the geometric display and its computation, not only the GUI shading.**
   **Recommended**: the simulation shows the same and more, from the real moves, and the
   fragile code goes with it. Alternative: remove only the shading and the toggle, and keep
   the computation for the text lines. Less goes, the fragile code stays, and the lines
   duplicate what the simulation shows.
2. **The simulation on when the app starts.** The toggle isn't saved between sessions, and
   today it starts off. **Recommended**: with the material-left shading gone, it is how you
   see what's left. It runs in the background after generating, typically in well under a
   second. Alternative: off at start as now, with a hint in the report when it's off.
   *Replaced 2026-10-30 by docs/specs/edit-and-simulate.md: projects open in Edit, and the
   simulation shows in Simulate mode (F6).*
3. **Keep the missed-material warning, text only.** **Recommended**: it flags a toolpath
   fault, not a property of the drawing. Alternative: drop it too and rely on the
   simulation, which would lose a warning that needs no simulation.
