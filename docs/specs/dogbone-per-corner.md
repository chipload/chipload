# Spec: dogbones chosen per corner

Status: **approved** 2026-10-26, decisions as recommended (the user: "it sounds good"),
with §2's "start round, pick the corners" made explicit.

## 1. Why

A user's pocket (contour #30 of `tests/corpus/dxf/shared_edges.dxf`, a 9 mm slot with a
3.167 mm step at each end, cut with a 6 mm tool) can't be generated with dogbones or
T-bones: "the dogbone at corner (11.214, 301.000) mm would cut into contour #30". The
corner at the foot of the step is so close to the step's top that any bone there reaches
it: a dogbone rounds the step's top corner by 0.6 mm, a T-bone along the longer wall (the
rule today, dogbones.md decision 4) takes 2.8 mm off the step. A T-bone along the shorter
wall would fit, notched into the long edge. Today the only ways out are to give up bones
for the whole operation or change the drawing.

Dogbones.md decision 1 left a choice per corner for later (TODO Later: "Dogbones: a choice
per corner on the canvas"). This is it.

## 2. What the user can do

The operation's **Corners** row stays as it is: round, dogbones or T-bones, up to an angle.
That is the default for every corner. On top of it, any corner of the operation's contours
on the tool's side (θ < 180°, also beyond the max angle) can be given its own choice:

- **as the operation** (no choice stored),
- **round** (no bone),
- **dogbone**,
- **T-bone along the longer wall** (the notch in the shorter one),
- **T-bone along the shorter wall** (the notch in the longer one).

So a user can leave out the one corner that doesn't fit, or turn a T-bone the other way.
Or the other way round: **start with every corner round** (the Corners row on *round*) and
pick only the corners that get a bone. The markers (§6) show on every corner on the tool's
side whatever the Corners row says, so there is always a corner to click.

## 3. Format (no version bump)

One optional field on profiles and pockets, beside `corners`. Missing or empty: no
choices, today's behaviour.

```json
"corner_choices": [
  {"at": [11.214, 301.0], "bone": "tbone", "along": "shorter"},
  {"at": [511.214, 301.0], "bone": "none"}
]
```

- `at`: the corner, in drawing coordinates (mm), as `starts` are.
- `bone`: `none`, `dogbone` or `tbone`; `along` (`longer` or `shorter`) with `tbone` only,
  and required there.
- Two choices at the same corner (within the matching tolerance, §4) are a problem.
- Older versions refuse the unknown field, which is the safe outcome. Saving leaves the
  field out when it is empty (as `rest_passes`), so a project without choices still opens
  in older versions.

## 4. Which corner a choice is for

- A choice belongs to the corner of the operation's contours (§1 of dogbones.md, without
  the max angle) whose point lies within 0.01 mm (`ARC_APPROX_TOL_MM`) of `at`.
- **A choice that matches no corner is an error** when generating, naming the operation
  and the point: "corner choice at (11.214, 301.000) mm matches no corner; the drawing
  changed?" (decision 3). `check-project` lists it as a problem with its path, and the
  GUI offers *Remove choices that match no corner*.
- **Moving the drawing:** a choice moves with `place` as a start point does
  (drawing-placement.md: when the shape nearest to it is from the DXF being moved), and with
  `shape move/turn/scale` when the corner is on a drawn shape that moves. (Start points
  don't follow drawn-shape transforms today; they would with the same change.)
- **Copies and parts:** the choice is made on the drawing, so every copy gets it.
- **Updating a DXF:** choices whose corner is still there keep working; the rest are the
  error above.

## 5. Checks

Unchanged: each leg is checked against every contour (dogbones.md §6). The refusal names
the corner and now also says when the contour it reaches is the operation's own contour:
"…would cut into another part of contour #30 (its own wall); choose another bone for this
corner". In the GUI, clicking the message selects the operation and shows the corner.

## 6. GUI

- With a profile or pocket selected, the canvas shows a small marker at each of its corners
  on the tool's side, also when the Corners row is *round*: filled where a bone is cut (with the bone drawn: the tool circle at
  the tip), hollow where the corner stays round, and a ring round the marker where the
  corner has its own choice.
- **Click a marker:** a menu with the five choices of §2, the current one ticked. Hovering
  a choice draws its bone on the canvas.
- **The Corners row** in Properties: after it, "2 corners chosen" and *Reset* (removes
  every choice) when there are choices.
- Undo: each choice is one step.

## 7. CLI (first, rule 5)

- `chipload op corners PROJECT --op N`: every corner of the operation on the tool's side,
  with its point, its angle, what it gets (round, dogbone, T-bone along the longer or
  shorter wall) and whether that is a choice. `--json` too.
- `chipload op corner PROJECT --op N --at X,Y --bone none|dogbone|tbone [--along
  longer|shorter]`; `--as-operation` removes the choice; `chipload op corner PROJECT --op N
  --reset` removes them all. `--at` takes units; it must match a corner (§4), or nothing is
  written.

## 8. Tests

- Pipeline: the user's file, pocket #30, 6 mm tool: refused with dogbones (the message
  says its own wall); with `tbone along shorter` at the four step corners, generated and
  checked; with `none` there, generated, and the other corners still get bones.
- A choice beyond the max angle gets its bone; `none` in an operation with dogbones leaves
  the corner round; `dogbone` in an operation with round corners cuts only there.
- A choice matching no corner: the error, and the problem path.
- `place` and `shape move` move the choices; copies get them.
- Format: round-trip; `tbone` without `along` and duplicates refused.
- Golden: unchanged (no corpus project has choices).

## 9. Decisions

1. **Choices stored as corner points in drawing coordinates**, beside the operation's own
   `corners` setting. *Recommended*: the same as start points, so moving the drawing
   already has a rule, and nothing depends on contour numbers, which change when a
   drawing does.
2. **Five choices per corner** (§2), including the T-bone's wall and corners past the max
   angle. *Recommended*.
3. **A choice that matches no corner is an error**, not skipped. *Recommended*, as
   dogbones.md decision 6: a missing bone makes a part that doesn't fit.
4. **No automatic way out:** the T-bone doesn't turn to the other wall by itself, and a
   bone that reaches the contour is still refused, even when, as here, it only reaches
   the pocket's own step. *Recommended*: the user picks per corner, and the message says
   so. Alternative: also let a dogbone round off the far end of a short wall when the
   contour turns back toward the waste there (0.6 mm here), so this file works with
   dogbones as is.
5. **No version bump:** one optional field, safe default. *Recommended*.
6. **Also make start points follow drawn-shape transforms** (§4), in the same change.
   *Recommended*: the same rule for both kinds of point.

## 10. As built

Step 1, 2026-10-26: the format, planning, the CLI, the guide.

- `chipload_job::{CornerChoice, Bone, Wall}`, `corner_choices` on profiles and pockets
  (left out when empty); problems for a T-bone without `along`, `along` without a T-bone,
  two choices within 0.01 mm, and choices on a profile on the line.
- `chipload_toolpath::corners`: `find_corners` finds every corner below 180° on the tool's
  side; `gets` decides what each gets (its choice, else the operation's bone up to the max
  angle); `operation_corners` and `unmatched_choice` check every choice against the
  operation's corners once, before the copies; copies move the choices with the contours.
  A bone that reaches its own contour gets `toolpath-error-corner-cuts-own`.
- `chipload_pipeline::corners::{corners_of, choose_corner, reset_corners}`; `place` moves
  the choices as it moves start points ("Moved 2 corner choice(s) with it."); the setup
  sheet says "2 corners chosen one by one". Rest passes and rest pockets are refused with
  a chosen bone as with `corners`.
- `chipload op corners`, `chipload op corner` (`--bone`, `--along`, `--as-operation`,
  `--reset`).
- The user's file: pocket #30 with a 6 mm tool and dogbones is refused at (11.214, 301)
  with "another wall of its own contour #30"; with T-bones along the shorter wall (or round)
  at the two step corners it generates and passes the checker
  (`crates/chipload-pipeline/tests/corner_choices.rs`).

Step 2, 2026-10-26: the canvas.

- `chipload-gui` `app/corners.rs`: the focused profile's or pocket's corners
  (`chipload_pipeline::corners::corners_in`, found again only when the operation or the
  drawing changes), drawn as diamonds with the tool's circle at the bone's tip
  (`chipload_toolpath::corners::notch_centre`, tested against the planned tips). A click or
  right-click opens the menu (an egui `Popup` beside the corner); hovering an entry draws its
  bone; a click stores it with `set_choice`, commits (one undo step) and generates.
- Found while looking at the user's file: corners the tool can't reach (the slot's end tabs,
  5.8 mm wide for a 6 mm tool) looked as if they got a bone. Planning now returns them
  (`Plan::unreached_corners`, copy 1), and the canvas draws them hollow and amber, with a
  hover that says so.
- Properties: "2 corners chosen one by one" and *Reset* under Corners. The legend has the
  corner diamond. A screenshot step `cornerN` shows a close-up of corner N with its menu.

Step 3, 2026-10-26: points that follow drawn shapes (decision 6).

- `chipload_pipeline::shapes::transform_shapes` (the box, the *Selection* section and
  `shape move/turn/scale`) moves every start point and corner choice whose nearest shape,
  among those the operation selects, is one of the moved shapes, by the same transform
  (`chipload_job::edit::transformed_point`; `placement::nearest_is`, the rule `place` uses).
  The app now passes its operations in, so they follow there too.
- Not moved: points on a shape changed through its own form or point handles (a circle's
  centre, a polyline's *Position*); those edit the shape, as dragging a point does.

Later, 2026-10-26 (the user: dogbones added to a rectangle are lost when the rectangle is
dragged): a single rectangle's or circle's own handles, and a shape's form, change it through
`set_shape`, which didn't move the points. Now every start point, corner choice and tab placed
by hand on the shape follows it (`chipload_job::edit::followed_point`): a rectangle's point
keeps its place in the rectangle (a corner stays that corner, also when resized or turned), a
circle's its angle and share of the radius, a polyline's its vertex or place along the same
segment while the polyline keeps its number of points. This replaces "Not moved" above.
