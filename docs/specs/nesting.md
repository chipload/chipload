# Spec: nesting parts on a board

Status: **draft 2026-11-17, for review.** Reviewed by the user 2026-11-17: "the spec looks
good", with a fitted board made a fixed size and the machine's travel respected (§5,
folded in), and with what doesn't fit switched off, not left off the board (§4, decision
10: the user, "yes, but let's explore pushing them to a second platter, like in a 3D
printer slicer"; explored in §11.1). It replaces the outline of 2026-11-16. Nesting came
into scope on 2026-11-16 (ADR 0017; the user: "Bring nesting into scope"). It builds on
instances (docs/specs/instances.md) and the part itself placed
(docs/specs/part-placement.md), both built.

## 1. What it is

*Nest* places a setup's instances on its board so that they fit without cutting into each
other, using as little of the board as it can. Every enabled part's instances are placed,
instance 1 (the part itself) included.

The result is only placements: each instance's `angle` and `at`, and instance 1's through
the part's `angle` and `at`. These are the same fields a placement made by hand writes.
Everything that checks a hand placement then checks it again: the overlap check, the
checker (cuts inside the stock), the simulation. The nest can be undone in one step and
adjusted by hand afterwards.

ADR 0017's reason for taking nesting in is that only Chipload knows what each instance
really cuts. So the nester places **what the tool cuts**, not the drawing (§3).

## 2. An example

A box: two sides, two ends, a bottom, a lid. Each is a part with a profile and tabs, some
with pockets. The board is 1220 × 610 mm. The user asks for 2 of each side and end, and
turns are allowed, except for the lid (its grain must run along).

*Nest…* on the setup gives a preview: the six parts ghosted where they would go. It says
"8 of 8 placed, board used to 742 mm of 1220 (61 %)". *Apply* writes the placements. The
toolpath, the simulation and the setup sheet then show the nested board.

## 3. What is placed: each instance's footprint

- **The footprint** is the area the instance's tools sweep: every feed move of every
  operation of its part, widened by the radius of the tool that made it. The interior is
  filled in: the part itself, and anything that falls out of it (§9.2), are inside its
  footprint. This is the region `cut_area` already compares for the overlap check
  (`chipload-toolpath` `plan/apart.rs`), with its holes filled.
- **Planned once per part:** each part is planned alone as drawn, with no instances, as
  `copies::cut_size` does today. A part with mirrored instances is also planned once
  mirrored. A turned footprint is the planned moves turned about the part's centre. That is
  exactly what instances.md §3 does to cut a turned instance, so the footprint and the cut
  agree.
- **Not part of a footprint:**
  - a board surface, which faces the whole board (surface-levels.md);
  - shapes that no operation cuts, and construction shapes.
- **Part surfaces are part of it:** they cut under each instance.
- **Dowel holes** belong to the board and are never moved (two-sided.md). Their footprints
  are **keep-out areas** that nothing may be placed on.
- **The gap** is the least distance between two footprints, mm (decision 4). It leaves a web
  of board between neighbouring cuts, which keeps the sheet stiff and the tabs holding.
- **The margin** is the least distance from a footprint to the board's edge, mm, room for
  screws or clamps (decision 4).
- **Turns** are set per part: none, half (0° or 180°), quarter (0°, 90°, 180°, 270°) or every
  15° (decision 5). Wood grain is the usual reason for none or half.
- **Mirrors are never added or removed.** A mirrored instance stays mirrored and is placed
  as it is: its mirrored footprint, turned.

## 4. How it places: bottom-left fill on a raster (decision 1)

The method is from published work. It uses no other nester's source or output (§12).

1. **Rasterise the footprints** on a grid of square cells, aligned to the board's usable
   area (the board less the margin). A cell is occupied if any part of the footprint touches
   it. This is the raster representation of nesting problems in Bennell & Oliveira, *The
   geometry of nesting problems: A tutorial*, European Journal of Operational Research 184
   (2008).
   - **Holes are filled** by a flood fill from outside the footprint's box. Cells the fill
     can't reach are inside the part.
   - **The gap** is added by dilating each placed footprint by the gap, in cells, rounded up.
   - **The cell size** is 0.5 mm, larger only if the board would have more than 8 million
     cells (a 1.4 × 1.4 m board at 0.5 mm).
   - **Rounding only ever adds room:** two footprints never come closer than the gap. They
     may end up as much as one cell farther apart.
2. **Order:** instances are taken largest footprint first (by area). Equal areas go in part
   order, then instance number.
3. **Place each instance bottom-left.** Try every turn the part allows. Take the position
   lowest along the board's longer side, then lowest across it, where its footprint meets no occupied cell and
   no keep-out area. This is bottom-left fill: Jakobs, *On genetic algorithms for the
   packing of polygons*, EJOR 88 (1996), its bottom-left rule; Chazelle, *The bottom-left
   bin-packing heuristic: an efficient implementation*, IEEE Transactions on Computers C-32
   (1983). "Bottom-left" here means along the board's longer side first (decision 6), so
   what is left of the board is one strip at its far end.
4. **Try several orders and keep the best:** sorted by area, by the longer side, by width
   and by height. The best layout places the most instances; among those, the one with the
   least board used along its longer side; among those, the smallest box round everything placed. The
   orders are a fixed list: there is no randomness, so the same project and settings give the
   same placements and the same G-code.
5. **The board's edges are checked exactly, not on the raster.** A footprint's box must lie
   within the usable area (the board less the margin, within the machine's travel, §5),
   compared in millimetres. A part exactly as wide as the usable
   area therefore fits, whatever the cell size.
6. **Verify.** The placements are written into a copy of the job, which is planned in full
   (the overlap check, `check_parts_apart`) and checked (the stock, the machine's travel).
   A nest that fails here is a bug. It is reported as one and nothing is applied: the
   nester's own raster is never the last word.

**What doesn't fit** is never dropped (decision 10). The checker only *warns* about cuts
outside the stock, so leaving it off the board would make a program that cuts beside the
board, into clamps or the spoilboard. Instead:
- the preview shows it parked to the right of the board, in the warning colour, and the
  result names it ("Lid, instance 2 didn't fit");
- *Apply* **switches it off**: an instance that doesn't fit gets `enabled: false`, and a
  part whose instance 1 doesn't fit is switched off as a whole (instance 1 can't be off
  alone, part-placement.md decision 3). It keeps its place where it was, so nothing it had
  is lost, and the report lists what was switched off by the nest;
- the user can make the board larger, allow turns, or move the part to a setup of its own,
  then switch it on and *Nest again* (several boards are a later step, §11).

## 5. What it needs and what it refuses

- **A board of a fixed size and place.** A stock fitted to the drawing (the drawing's
  outline plus a margin, `fit`) or centred on it (`origin: drawing`) follows what is
  placed, so nesting onto it would chase itself. Nesting **changes it to a fixed size**
  (the user, 2026-11-17):
  - the dialog says so ("The board is fitted to the drawing; nesting gives it a fixed
    size") and asks for the size, starting from the machine's travel when it is set, else
    the board's size now;
  - *Apply* sets `fit` to none and the stock to that size, its bottom-left corner where the
    board's is now (`origin: bottom_left` with the drawing moved under it is not done: the
    drawing never moves). Together with the placements, one undo step;
  - on the command line `--board 1220x610` does the same; on a fitted board without it the
    command refuses, naming the option.
- **The machine's travel is respected** (the user, 2026-11-17). With `machine.travel` set:
  - a board side longer than the travel on that axis is used only up to the travel,
    measured from the side where work zero is (from its left or bottom edge when work zero
    is at the drawing origin or the board's centre), and the dialog says so ("The board is
    1220 mm along X, the machine travels 800 mm: 800 mm of it is used"). The usable area is
    the board less the margin, within that window;
  - the verification (§4.6) runs the checker, whose travel check is an error, so a nest
    whose program would move farther than the machine travels is never applied;
  - with no travel set, the board alone limits, and the checker's usual warning ("no
    travel set") stands.
- **Refused with a reason:**
  - a setup cut in pieces: pieces refuse instances (pieces.md);
  - a setup with a back, or one that is the back of another (decision 7): the back's parts
    are placed in the front's coordinates, and moving the front's parts would leave the back
    behind;
  - a setup with no enabled part.
- **Work zero** is left as it is. A work zero at a board corner stays at that corner; one at
  the drawing origin stays there.

## 6. How many of each

Each part's **count** is how many times it is cut: instance 1 and its enabled instances.
The dialog and the command line can set it before nesting (decision 8):
- **more** adds unmirrored instances, each at angle 0, placed by the nest;
- **fewer** removes the highest-numbered instances (never instance 1).

Disabled instances are left alone, neither placed nor counted. Counted that way, "2 of each
side, 1 lid" is one step instead of adding instances one by one.

## 7. In the window

- A setup's right-click and the ribbon's layout group have **Nest…**, which opens a dialog:
  - **Board:** width and height (from the stock), margin and gap.
  - **Parts:** one row each, with its name, count and turns. Turns are kept per part (§10).
  - **Preview:** runs the nest in the background, with the progress bar other long jobs
    have, and can be cancelled. The canvas shows the result as ghosts on the board, with a
    line: "8 of 8 placed, board used to 742 mm of 1220 (61 %)". What didn't fit shows
    parked to the right, in the warning colour.
  - **Apply** writes the placements (and counts, and the board, §5) as one undo step.
    **Cancel** leaves everything as it was.
- After *Apply*, instance 1's row and ghost show for every placed part, as for a hand
  placement (part-placement.md §6). Instances are dragged and turned by hand as before.
- **Nest again** in the setup's menu nests with the settings kept in the file, without the
  dialog, as one undo step. It is there once the setup has been nested.

## 8. Command line

```
chipload nest part.chipload [--setup ID] [--board 1220x610] [--gap 3] [--margin 10]
                            [--turns none|half|quarter|15] [--part ID --turns … --count N]…
                            [--dry-run]
```

- `--turns` without `--part` sets every part's turns. With `--part` it sets that part's.
  Both are kept in the file (§10).
- `--count` adds or removes instances (§6).
- `--dry-run` reports without writing.
- `--json` gives `{ placed: [{part, instance, angle, at}], unplaced: [...], used_x, used_pct }`.
- Run through `chipload mcp` as every command is.
- `part set --turns` sets a part's turns without nesting.

## 9. Interactions

1. **Overlap check:** after a nest, every instance is "placed", so the check applies to all
   of them (apart.rs). Parts drawn on top of each other and cut where drawn are allowed
   today; once nested, they are apart.
2. **Small parts inside a part's holes** (a slug that falls out of a frame): not in the first
   build. Holes are filled (§4.1). Whether a hole is free depends on whether its slug is
   held by tabs, falls out, or is pocketed away. That is a later step (§11), with its own
   rule for which holes count as free.
3. **Tabs, ramps, finishing laps, dogbones** are in the toolpath and so in the footprint.
4. **Reliefs, V-carves, texts:** inside their part's footprint like any cut.
5. **Inlays:** the plug's setup nests like any other; its part is mirrored, and stays so.
6. **Setup sheet:** the picture shows the nested board, as for hand placements. The facts
   gain "Nested: 8 instances, the board used to 742 mm".
7. **DXF export** writes the drawing as drawn (the definitions). `dxf export --as-cut` (each
   instance's shapes where cut, part-placement.md §5) becomes worth building with nesting,
   for a laser or a shop that wants the nested sheet as a drawing. A step of its own (§11).
8. **Changing a part after nesting** (larger shapes, another tool) can make instances
   overlap. The overlap check says so, naming the two instances, and *Nest again* fixes it.

## 10. The file (no schema bump)

Two optional fields, so *Nest again* remembers the settings:

```json
"setups": [ { "id": 1, ..., "nest": { "gap": 3.0, "margin": 10.0 } } ],
"parts":  [ { "id": 2, ..., "turns": "none" } ]
```

- `setup.nest`: missing means never nested. Set by *Apply* and `chipload nest`.
- `part.turns`: `none`, `half`, `quarter` or `every_15`. Missing means `quarter`.

Neither changes what is cut. Placements are in `angle` and `at` as before. So neither
needs a bump, as optional fields with a safe default don't (CLAUDE.md). As with `drawing.texts`, an older Chipload refuses
a file that has them, because `Setup` and `Part` deny unknown fields. Validation: gap ≥ 0,
margin ≥ 0, both finite. Saving writes both explicitly when set.

## 11. Order of work (decision 9)

Each step is a commit, with its tests:
1. **Footprints and the raster**, in `chipload-toolpath` `nest/`: rasterise a part's planned
   moves at a turn, fill holes, dilate, keep-outs. Property tests.
2. **The placer:** bottom-left fill, the orders tried, the exact edge check, parking what
   doesn't fit. Property tests.
3. **The pipeline:** `nest_setup(job, settings) -> NestResult`: plan each part once,
   footprints, place, verify on the full plan. Counts (§6), refusals (§5). The two
   optional fields.
4. **The command line:** `chipload nest`, `part set --turns`, `--json`, a frozen golden of a
   nested project.
5. **The window:** the dialog, the preview in the background, *Apply*, *Nest again*.
6. **Setup sheet, guide page** (`drawings/nesting.md`), schemas, translations.

**As built (step 1, 2026-11-17).** `chipload-toolpath` `nest/`: `Sweep` (a part's feed
moves as capsules, each with its tool's radius; a G2/G3 split into chords widened by the arc
tolerance; turned exactly on quarter turns with `place.rs`'s rotation), `Grid` (a bit per
cell, rows of 64-bit words, laid on another at any cell offset to test or mark), `Frame`
(a grid in the drawing; a cell is occupied when a capsule touches it, measured exactly from
the segment to the cell's square), `Footprint` (its holes filled by a flood from outside,
across sides only) and `Footprint::dilated` (cells nearer than the gap, square to square,
stamped from the edge cells only), `cell_size`. Property tests (`tests/nest.rs`): every
point swept is in the footprint; a capsule occupies only cells it touches; footprints whose
grown and plain cells don't meet are at least the gap apart, measured exactly on the
capsules at every column along a row (a test that tried random places missed a halved gap,
so it scans now); growing from the edge equals growing from every cell; the word-wise grid
equals cell-by-cell; a quarter turn swaps width and height exactly.

**As built (step 2, 2026-11-17).** `nest/fill.rs`: `Sheet` (the usable area, its keep-outs,
packed along its longer side), `Item` with one `Lie` per allowed turn, `fill` giving a
`Layout` (each `Put`: the item, its lie, where its footprint's low corner goes; what didn't
fit; how far along the board it reaches). The board is kept as runs of occupied cells per
line across it; a footprint is tried at each cell along, slid across, jumping past every
run it meets, so the first cell along where it fits anywhere ends the search (the
bottom-left rule needs no more). The far edges are compared in mm (`PLACEMENT_ROUNDING_MM`).
The four orders of §4.4 are tried, repeats once; the best has the most placed, then the
least length used, then the smallest box. Property test: every layout keeps its parts at
least the gap apart and clear of a keep-out hole (exactly, on the capsules), inside the
usable area, loses nothing, and is the same twice; it caught a placer that ignored the gap
and one that let parts overhang by 5 mm, each broken on purpose. Timing (release): 50
instances of five parts with quarter turns on a 2440 × 1220 board, 0.39 s in all, every
one placed, 1503 mm of the board used.

**As built (step 3, 2026-11-17).** `chipload_pipeline::nest::nest_setup(job, request,
cancel)` with a `NestRequest` (setup, board, gap, margin, turns for all or per part,
counts) gives a `NestResult` (the project, what went where, what was switched off, the
length used, whether the board was fixed, the travel's limits) with a `summary` and
`notes` as messages in seven languages. Each part is planned alone once, as drawn, with
one extra instance per mirror its instances use, set far to the right; board surfaces and
dowel holes are left out of its sweep, the dowel holes kept out of the board. Where a
part's instance 1 doesn't fit but another unmirrored instance of it did, instance 1 takes
that place and the other is switched off, so the part stays on and is cut as many times.
The result is checked by `run_setups` for the setup; a placement broken on purpose was
refused there. Turns are stored in −180°…180° (a quarter turn back is −90°, not 270°).
- **One change from §5:** a board that followed the drawing becomes a fixed board with its
  bottom-left corner at the drawing's origin (`origin: bottom_left`), not where its corner
  was. A fixed board can only sit at the origin (by its corner or its centre); the one
  form that sits elsewhere, centred on the drawing, follows the drawing. Since every
  instance, the part itself included, is placed anew, where the drawing lies doesn't
  matter, and the drawing doesn't move.
- `Nest` on a setup and `Turns` on a part; the JSON Schema and the reference page are
  regenerated. A glossary word: *nest* (verschachteln, imbriquer, anidar, annidare,
  nästla, раскроить).

**As built (step 4, 2026-11-17).** `chipload nest PROJECT [--setup ID] [--board XxY]
[--gap L] [--margin L] [--turns TURNS|ID=TURNS]… [--count ID=N]… [--dry-run] [-o OUT]`
and `part set --turns`. Changes from §8:
- **Per part as `ID=VALUE`:** `--turns 2=none` and `--count 2=4`, repeatable, in place of
  `--part ID --turns … --count N` groups, which clap can't pair reliably. `--turns
  quarter` without an id sets every part. Turns are `none`, `half`, `quarter`,
  `every-15` (also `every_15`, `15`).
- **`--json`** has `setup`, `placed` (part, instance, angle in radians, at), `unplaced`,
  `used`, `length`, `used_pct`, `board_fixed`, `travel`; the notes are in the envelope's
  `notes` with every printed line.
- **A slack along the board (§4.3):** a part may go up to 5 mm farther along the board
  (`ALONG_SLACK_MM`) when that puts it lower across. The strict rule sent the badge's
  fourth instance to the top of the second column for a 1 mm gain, leaving the bottom
  free; with the slack it takes the bottom. The property tests and the timing (0.35 s for
  50 instances) are unchanged.
- **The guide page** `drawings/nesting.md` came with this step (the command line), as the
  guide test asks of every command; step 6 adds the window to it.
- **Golden:** `jobs/v40/nest_badge.chipload`, the badge of v38 on a 400 × 300 board before
  nesting; `golden_nest_gcode` nests it (`--count 1=5`) and runs it, twice, against
  `expected/v40_nest_badge.nc`, so the golden pins the placer. The frozen messages gain
  the sample's plan.

**As built (step 5, 2026-11-17).** `chipload-gui` `app/nesting.rs`: the Nest window (a
window, not a modal, so the canvas shows the preview beside it), opened from the setup's
right-click *Nest…* and *Wizards > Nest…*; *Nest again* in the setup's menu once nested.
Changes from §7:
- **Not on the ribbon:** a sixth WIZARDS button left the ribbon no room for its names at
  the usual 1280 px window (the layout test caught it), so *Nest…* is in the Wizards menu.
- **The preview starts when the window opens**, and again with *Preview* after a change;
  a change stops a run for the old settings. *Apply* before a preview nests and applies.
  There is no progress bar: nesting reports no progress, so a spinner shows while it runs.
- **Ghosts:** each instance's outlines where it would go in the selection colour, the
  board nested on dashed, the instances' own ghosts hidden meanwhile; what didn't fit
  parked right of the board in amber (`NestResult::preview_job`, `PARK_GAP_MM` 20 mm
  apart). The window lists the summary and every note.
- **Apply** is one undo step; the summary goes to the status bar, the notes to the Report.
- **Nest again** keeps the instances there are (no counts) and each part's turns.
Tests: preview and apply with undo, parking, Nest again, a refusal in the window, the
window's rows; an ignored picture test draws it.

**Later, each with its own decision:** small parts in holes (§9.2), a second board for what
doesn't fit (§11.1), `dxf export --as-cut`, nesting a front and back together.

### 11.1 A second board for what doesn't fit (to explore)

The user's idea (2026-11-17): what doesn't fit goes onto a second platter, as a 3D printer
slicer adds plates, rather than being switched off. Perhaps out of reach now. What it would
take:
- **A second board is a setup:** the same stock, its own program and setup sheet. That
  part is easy: setups exist.
- **The hard part is that a part belongs to one setup** (`part.setup`), and its instances go
  with it. Instance 3 of "side" on board 2 while instances 1 and 2 stay on board 1 can't be
  written today. Two ways:
  - **an instance names its setup** (an optional `setup` on an instance, the part's
    otherwise): the part stays one definition, cut on several boards. This needs a schema
    bump (an older Chipload would cut every instance on the first board), and planning,
    the overlap check, the sheet and the Browser would all go per setup and instance;
  - **the nest copies the part into the new setup** with the instances that didn't fit:
    no new field, but two parts to keep in step after every later edit. That works against
    what instances are for.
- **The slicer's view** (boards side by side, the active one in colour, the others as
  ghosts) is TODO's *Setups on sheets of their own*. Today every setup's board lies on the
  same drawing coordinates, so a second board would sit on top of the first.

**Recommended:** the first way, as its own spec after nesting is built and tried, together
with sheets of their own, since a second board is only clear when the boards are side by
side. Until then, *Apply* switches off what doesn't fit (decision 10).

## 12. Clean room

The nester is built from the papers named in §4 and from first principles only: rasters,
flood fill, dilation, sorting. No nester's source (Deepnest, SVGNest and the like) is read,
and no other program's output or layouts are used as a model (CLAUDE.md).

## 13. Tests

- **Property tests** (random rectangles, L-shapes and rings, random turns and counts):
  - no two placed footprints are closer than the gap, measured exactly on the moves (the
    `segment_distance_within` test of apart.rs), not on the raster;
  - every placed footprint lies within the board less the margin;
  - nothing is lost: placed plus unplaced equals the count asked for;
  - the same input twice gives the same placements, bit for bit.
- **Boundaries:**
  - no instances to place;
  - a part larger than the board (reported, parked);
  - a part exactly the usable width (fits);
  - gap 0 (footprints may touch, never overlap);
  - a margin that leaves no usable area (refused);
  - a keep-out dowel hole in the only place a part would fit;
  - a mirrored instance turned 90°.
  - a fitted board (made a fixed size, the drawing unmoved) and a board longer than the
    machine's travel (only the travel used; the checker passes);
  - what doesn't fit switched off, a part whose instance 1 doesn't fit switched off whole.
- **Pipeline:** a nested project plans with the overlap check, passes the checker, and its
  simulation shows every instance on the board. The refusals of §5 each have a test.
- **Golden:** one frozen nested project (box parts, a turned and a mirrored instance) and its
  G-code. Nesting is deterministic, so the golden pins the placer too.
- **Performance:** 50 instances of mixed parts on a 2440 × 1220 board, under 10 s in a
  release build (decision 3).

## 14. Decisions

1. **One method: bottom-left fill on a raster of the real footprints.** **Recommended.**
   - It is simple and robust. It never places two cuts closer than the gap, by
     construction.
   - It nests irregular parts into each other's concave spaces, any turn.
   - Every placement is checked again exactly on the full plan.

   It replaces the outline's two steps: rectangles by MaxRects, then shapes by no-fit
   polygons. No-fit polygons need Minkowski sums and polygon unions on arbitrary
   outlines with arcs: the hardest and most fragile geometry there is, for a better fit
   than the raster's half millimetre. MaxRects nests rectangles well, but for real parts it
   is a worse version of the same thing (no worse options). Alternative: the outline's plan,
   MaxRects first, then no-fit polygons.
2. **Place what the tool cuts, holes filled.** **Recommended** (ADR 0017's reason for
   nesting). Alternative: the drawn outlines plus a clearance, which ignores tabs and the
   tool's width.
3. **Speed:** under 10 s for 50 instances on a full sheet, in the background with a progress
   bar and Cancel. **Recommended.** Quality from trying more orders (local search) is a later
   step if layouts disappoint.
4. **Gap and margin defaults:** a gap of 3 mm between cuts and a margin of 10 mm to the edge.
   **Recommended:** a web of board between slots, room for screws. Both can be changed.
   Alternative: the gap as the largest tool's diameter (the outline's), which wastes about
   two diameters between parts: their slots already keep them two diameters apart at gap 0.
5. **Turns per part, kept in the file:** none, half, quarter (default) or every 15°.
   **Recommended**, so grain is remembered. Alternative: one setting per nest, not stored.
6. **Pack along the board's longer side**, from the end nearest X0 Y0, leaving one offcut
   strip across the far end. **Recommended**. Alternative: always along X.
7. **Setups with a back are refused** for now. **Recommended.** Nesting a front and its back
   together, as pairs, is a later step.
8. **Counts in the nest dialog** add or remove instances (§6). **Recommended.**
   Alternative: counts are set by hand first, and the nest only places.
9. **Order of work** as §11. **Recommended.**
10. **What doesn't fit is switched off by *Apply***, kept where it was and listed in the
    report (§4). **Recommended:** the checker only warns about cuts outside the stock, so a
    part parked off the board would be cut beside it. Alternatives: *Apply* refused until
    everything fits; or parked off the board with the run refused while anything is there
    (a new error in the pipeline).
