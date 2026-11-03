# Spec: tabs along the grain

Status: draft 2026-11-03, decisions in §6 to review.

## 1. Why

The user, 2026-11-03: "could we have the option to place them in x direction or y
direction? This way i can quickly align them to the fibre direction of the wood. Now they
are equally placed around the perimeter and i have to drag them every time".

A tab is a short bridge of wood from the part to the sheet. Its fibres decide how strong it
is: with the grain running along the bridge it holds; across the bridge it can split off
while the rest is cut, and the part comes loose. Today a profile's `count` tabs are spaced
evenly round each contour (docs/specs/tabs.md §2), so on a long part some land where the
grain runs across them, and the user drags them every time (docs/specs/tabs-by-hand.md).

## 2. What changes

The profile's **Tabs** get a **Placement**:

| Placement | Where the tabs go | For grain |
|---|---|---|
| **Around** (as today) | evenly round the contour, slid off corners | any |
| **Left and right** | on the contour's left and right sides | along X |
| **Top and bottom** | on the contour's top and bottom sides | along Y |

With *Left and right*, a tab's bridge runs in X, so wood whose grain runs in X has its
fibres along the bridge. *Top and bottom* is the same turned 90°.

## 3. Where exactly

For each ring the profile cuts, with *Left and right* (top and bottom the same, X and Y
swapped):
- **How many:** `count` tabs in all, half on each side; an odd count puts the extra one on
  the left (bottom).
- **Where on a side:** the side's tabs at evenly spaced heights over the ring's box,
  `y = bottom + (k + ½)·height / n` for the n tabs of that side; each tab goes to the point
  of the ring nearest to (left edge of the box, y) — on a rectangle, exactly on its left
  side; on a circle or a curved part, near its leftmost point.
- **Checked as tabs placed by hand are** (`tabs_at`): two tabs too close along the ring,
  or a tab where every pass plunges, are refused with the reason, and a smaller count or
  *Around* is the way out.
- **A ring with tabs placed by hand** keeps them, as today: the placement is for rings
  without. *Space this ring's tabs evenly* (right-click a tab) becomes **Place this ring's
  tabs again**, by the placement chosen.
- **Holes** cut through with tabs get the same rule on their own box.

## 4. Format, command line, styles

- One optional field in `tabs`, left out when *Around* (as `at`), so no schema bump and
  projects without it open in older versions:
  `"tabs": {"count": 4, "width": 8, "height": 3, "flank_angle": 0.52, "along": "x"}`
  (`"x"`: left and right; `"y"`: top and bottom).
- `chipload op tabs part.chipload --op 2 --along x` (also `--along y`, `--around`).
- A style keeps it (docs/specs/styles.md §3): a style "Cut out 18 mm ply, grain X" places its
  tabs that way on every part.
- The setup sheet says where the tabs are ("4 tabs, left and right").

## 5. Tests

- A rectangle: 4 tabs left and right sit on its left and right sides at ¼ and ¾ of its
  height; top and bottom likewise; 3 tabs give 2 left, 1 right.
- A circle: near its leftmost and rightmost points.
- Too many for a small side: refused with the reason.
- A ring with tabs by hand keeps them; *Place this ring's tabs again* follows the placement.
- The field is left out with *Around*; a project with it plans the same as its tabs placed
  by hand at those points (compared by G-code).
- Golden: one new project with *Left and right*; existing G-code unchanged.

## 6. Decisions to review

1. **"X" means tabs on the left and right sides**, their bridges running in X, for grain
   along X (§2). *Recommended:* the fibres then run along the bridge, the strong way.
   Alternative: the other way round, if you think of the tab as lying across the grain.
2. **`count` is the total**, split between the two sides (§3). *Recommended:* the count
   field keeps one meaning. Alternative: `count` per side.
3. **Per operation** (and so per style), not per setup. *Recommended:* simple, and a style
   carries it. Alternative, later: the setup's stock gets a grain direction, and tabs set
   to *Along the grain* follow it, so turning the sheet is one change.
4. **Names:** *Around*, *Left and right*, *Top and bottom* in the window; `--along x|y` and
   `"along": "x" | "y"` in the file. *Recommended:* the window says where they go, the
   file and command line say which axis.
