# Spec: tabs along the grain

Status: built 2026-11-03 (§7); approved 2026-11-03 (decision 1 as the user put it, 2-4 as recommended; the user:
"calling it x or y is good since another user might want the opposite direction").

## 1. Why

The user, 2026-11-03: "could we have the option to place them in x direction or y
direction? This way i can quickly align them to the fibre direction of the wood. Now they
are equally placed around the perimeter and i have to drag them every time".

A tab is a short bridge of wood from the part to the sheet. With the grain running across
the bridge, along the part's edge, the tab is easy to cut loose afterwards: the knife or
chisel follows the fibres along the edge. That is what the user wants (2026-11-03: "i want
the bridge in the same direction as the tab so it is Easy to cut loose"): tabs on the edges
that run with the grain. Today a profile's `count` tabs are spaced evenly round each
contour (docs/specs/tabs.md §2), so some land on edges across the grain, and the user drags
them every time (docs/specs/tabs-by-hand.md).

## 2. What changes

The profile's **Tabs** get a **Placement**:

| Placement | Where the tabs go | For grain |
|---|---|---|
| **Around** (as today) | evenly round the contour, slid off corners | any |
| **Along X** | on the contour's top and bottom edges, which run in X | along X |
| **Along Y** | on the contour's left and right edges, which run in Y | along Y |

X and Y name the edges, not a use: with the grain along the tabs' edges they cut loose
easily (the user's way); with the grain across them, along each bridge, they hold
stronger (another user's). The hover says both.

## 3. Where exactly

For each ring the profile cuts, with *Along X* (*Along Y* the same, X and Y swapped):
- **How many:** `count` tabs in all, half on each edge; an odd count puts the extra one on
  the bottom (left).
- **Where on an edge:** the edge's tabs evenly spaced across the ring's box,
  `x = left + (k + ½)·width / n` for the n tabs of that edge; each tab goes to the point of
  the ring nearest to (x, bottom of the box), or (x, top) — on a rectangle, exactly on its
  bottom and top edges; on a circle or a curved part, near its lowest and highest points.
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
  (`"x"`: on the edges along X, top and bottom; `"y"`: along Y, left and right).
- `chipload op tabs part.chipload --op 2 --along x` (also `--along y`, `--around`).
- A style keeps it (docs/specs/styles.md §3): a style "Cut out 18 mm ply, grain X" places its
  tabs that way on every part.
- The setup sheet says where the tabs are ("4 tabs, along X").

## 5. Tests

- A rectangle: 4 tabs along X sit on its bottom and top edges at ¼ and ¾ of its width;
  along Y likewise on its left and right edges; 3 tabs give 2 at the bottom, 1 at the top.
- A circle: near its lowest and highest points (along X).
- Too many for a small side: refused with the reason.
- A ring with tabs by hand keeps them; *Place this ring's tabs again* follows the placement.
- The field is left out with *Around*; a project with it plans the same as its tabs placed
  by hand at those points (compared by G-code).
- Golden: one new project with *Along X*; existing G-code unchanged.

## 6. Decisions to review

1. **Along X puts the tabs on the edges that run in X** (top and bottom), the grain across
   each bridge, so the tabs cut loose easily (§2). *Agreed 2026-11-03* (the user: "i want
   the bridge in the same direction as the tab so it is Easy to cut loose"). Such a tab is
   also the weaker one while cutting; its height and count are what hold the part.
2. **`count` is the total**, split between the two sides (§3). *Recommended:* the count
   field keeps one meaning. Alternative: `count` per side.
3. **Per operation** (and so per style), not per setup. *Recommended:* simple, and a style
   carries it. Alternative, later: the setup's stock gets a grain direction, and tabs set
   to *Along the grain* follow it, so turning the sheet is one change.
4. **Names:** *Around*, *Along X*, *Along Y* in the window (the hover says which edges);
   `--along x|y` and `"along": "x" | "y"` in the file. *Recommended:* the same word as the
   grain's direction everywhere.

## 7. As built (2026-11-03)

- `Tabs.along: Option<Axis>` (`chipload_job::Axis`, `"x"`/`"y"`), left out when null;
  `chipload_toolpath::tabs::along_tabs` makes the target points on the tool-centre ring's
  box and hands them to `tabs_at`, clamped along the ring clear of its start first.
- Window: *placement* under Tabs (*around*, *along X*, *along Y*, each with its hover);
  *Space this ring's tabs evenly* is *Place this ring's tabs again*, and *Reset*'s hover
  says the same.
- `op tabs --along x|y` / `--around` (then lists the tabs as before). The setup sheet adds
  "along X (bottom and top edges)" to the tabs cell.
- Golden `v35_tabs_along`; a CLI test gives the same G-code with those tabs placed by hand.
