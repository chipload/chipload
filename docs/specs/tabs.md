# Spec: holding tabs on profile operations

Status: **approved 2026-10-08**

Tabs are short bridges of material left at the bottom of a profile cut so the part stays
attached to the stock after the last pass. Without them a through-cut part comes loose on the
last pass and can move into the cutter or be thrown.

Builds on `v0-profile-pipeline.md`. Out of scope: tabs on pockets, manual tab placement by
clicking in the viewer (later), and ramped tabs (v1 tabs are square, decision 2026-10-08).

## 1. Job model

New optional field on `ProfileOp`, default `null` (no tabs, so existing jobs are unchanged):

```json
"tabs": { "count": 4, "width": 8, "height": 3 }
```

| Field | Meaning |
|---|---|
| `count` | integer ≥ 1, tabs per contour |
| `width` | mm, **width of the material bridge left on the part**, measured along the part edge (decision 2026-10-08) |
| `height` | mm, thickness of the bridge, measured **up from the bottom of the cut** |

Optional with a safe default, so no schema version bump (CLAUDE.md). Job files written
with tabs can't be read by older Chipload versions, which refuse unknown fields: that's the
safe outcome.

## 2. Placement

Per contour (each tool-centre ring, after offsetting and orienting), with L = ring length and
D = tool diameter:

- **Tab gap on the tool path.** The cutter's radius eats into the bridge from both sides, so
  the tool must lift for `width + D` along its path to leave a bridge of `width` on the part.
  (For "on" profiles the same rule leaves `width` of uncut material.)
- **Nominal positions, equal spacing by arc length.** With spacing `S = L / count`, tab i's
  nominal centre is at `(i + ½) · S`, measured from the ring's start point.
- **Slide off corners** (decision 2026-10-08). A tab gap is *straight enough* if the path turns
  by less than `TAB_MAX_TURN` (15°) in total within it (sum of the absolute turning angles at
  the vertices inside the gap). Outside-profile corners become small arcs of the tool radius
  on the path, so they count as turning.
  - Each tab may move by at most `S / 4` from its nominal centre, and its gap must stay inside
    its own slot `[i·S + D/2, (i+1)·S − D/2]`. Slots don't overlap, so tabs can never overlap
    each other, every tab keeps at least D of normal cutting from its neighbours, and the
    start point (a slot boundary, where every pass plunges) is never inside a tab.
  - Candidate centres are tried in steps of `TAB_SLIDE_STEP_MM` (0.25 mm). The straight-enough
    candidate closest to the nominal centre wins; on a tie the forward one wins.
  - If no candidate is straight enough (e.g. a small round part), the tab stays at its nominal
    centre and the report says so, once per contour.
- **Errors (never skip silently):**
  - `height >= depth`: the tab would be the whole cut.
  - The tabs don't fit: `count × (width + D)` must leave at least one tool diameter of normal
    cutting between tabs, i.e. `L / count − (width + D) ≥ D`. The error names the contour and
    the largest count that fits.
- **Warning:** tabs on an `inside` profile (a hole). Allowed, since sometimes you want the slug
  held, but usually unintended.

## 3. Toolpath

- **Tab top:** `z_tab = z_bottom + height`, where `z_bottom` is the cut depth (`top_z − depth`).
- Passes at or above `z_tab` are unchanged.
- Passes below `z_tab`: the tool cuts normally until a tab gap starts, then
  1. moves up to `z_tab` (G1 at `plunge_feed`),
  2. feeds across the gap at `z_tab` (G1 at `feed`),
  3. moves back down to the pass depth at the end of the gap (G1 at `plunge_feed`),
  4. continues.
- Tab-gap endpoints are inserted into the ring at the exact arc-length positions (splitting
  segments), so the bridge width is exact rather than snapped to existing vertices.
- Square tabs only in v1 (decision 2026-10-08). Ramped tabs may come later as an option.
- No extra depth pass is added at `z_tab`. If a pass is only a little below `z_tab`, the lift is
  short.

## 4. Checking

- A post-condition after building the moves: on every ring, the total XY length cut below
  `z_tab` must equal `L − count × (width + D)` within `CHAIN_TOL_MM` per tab, and no move inside
  a tab gap may be below `z_tab`. Otherwise → `GeometryError`.
- The independent G-code checker needs no change: lifts and drops are ordinary G1 moves, and
  `z_tab` is above the stock bottom.

## 5. Viewer

`RunResult` carries the tab spans per ring (start and end points on the tool path) and the
viewer draws them in a distinct colour over the path, so tabs are visible in plan view.

## 6. Tests

- **Placement:** count, nominal centres at `(i + ½)·L/count`, start point never inside a tab,
  gap length `width + D`, segments split at exact positions.
- **Sliding:** a rectangle whose nominal tab centre sits on a corner gets the tab moved onto the
  straight edge (closest position, forward on ties); a small circle keeps nominal positions
  and reports one note; slides never leave the slot.
- **Boundaries:** `height = depth` → error; height just below depth; one tab; tabs that only
  just fit, and one tab too many → error naming the largest count that fits; tool larger than
  the tab width (still fine: the gap is `width + D`).
- **Passes:** with depth 18, step 6 and height 3, only the last pass lifts (to −15); with height
  8, the last two passes lift.
- **Hypothesis:** for random rings, counts, widths and heights that fit, the post-condition
  holds and the bridge left on the part is `width` long.
- **Golden:** the v2 sample with tabs on the outline, and `messy_2` lower outline with 4 tabs.

## 7. Known limitations
No manual placement; square tabs only; no tabs on pockets; on parts with no straight stretch
tabs stay on curves.
