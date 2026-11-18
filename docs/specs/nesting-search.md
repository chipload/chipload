# Spec: a better nest, by search and with a goal

Status: **approved 2026-11-18** (the user: "go ahead"), being built: steps 1 to 3 done. It changes how the nester of docs/specs/nesting.md
chooses places (its §4, decisions 3 and 6), not what it places or what it writes. The user,
2026-11-18: "the nesting should start from the lower left corner and try to minimize the
total area occupied", "I don't think we should place the parts in a fixed order, that could
lead to a suboptimal nesting, we need a more clever algorithm", and "perhaps there should be
three options, min box, min y and min x, so the user can choose which way he or she wants to
optimize".

## 1. What changes, and why

Today's nester (nesting.md §4) places parts one by one along the board's **longer** side
first, in four fixed orders, and keeps the best layout. Two weaknesses:

- **Along the longer side first** sends the next part beside the last one on a tall board,
  turned if need be, instead of above it: a strip across the board, not a compact block (the
  user's finding of 2026-11-17, a tall board).
- **Four fixed orders** see very few of the ways parts can be ordered and turned. Mixed part
  sizes are where it shows.

What this spec adds:
1. **A goal** the user chooses: a compact block, or the parts kept low, or kept to the left
   (§2). The default is the compact block.
2. **A search** over the order the parts are placed in and the turn of each (§4), with the
   same one-by-one placing inside it (§3).

What stays: the footprints, the raster, the gap and margin, the board's edges checked
exactly, the travel, keep-out dowel holes, the verification by a full plan and the checker,
what doesn't fit, overflow onto more boards, the refusals (nesting.md §3 to §6;
parts-and-setups.md §9). Everything starts from the **lower-left corner** of the usable
area (the board less the margin, within the machine's travel).

## 2. Three goals

| Goal | Window | `--goal` | What is left of the board |
|---|---|---|---|
| The smallest box round the parts | **Compact** | `compact` | an L round a block in the lower-left corner |
| The lowest top edge | **Keep the top free** | `low` | a strip across the top, the full width |
| The leftmost right edge | **Keep the right side free** | `left` | a strip down the right, the full height |

*Compact* suits a board that is used up by this job. The other two leave an offcut that is
one rectangle, to saw off and keep. They are not a better and a worse way of doing one thing
(no worse options): which is right depends on what the user does with the rest of the board.

**A layout's score**, compared in order, the first difference deciding:

1. **The most instances placed**, then **the most footprint area placed**: whatever the
   goal, more on the board wins. (With overflow, the fewest boards follows from this, each
   board taking as much as it can.)
2. **The goal's own measure**, in raster cells (whole numbers, so comparisons are exact):
   - *Compact*: the area of the box round the placed footprints;
   - *Keep the top free*: the top edge of that box;
   - *Keep the right side free*: its right edge.
3. **The other measures** break ties: *compact* then the top edge, then the right edge;
   *low* the box area, then the right edge; *left* the box area, then the top edge.
4. Then the search's own order (§4): the first layout found wins.

The box is round the footprints as placed, without the gap. Its corner is the usable area's
lower-left corner, not the lowest part: the parts always start there, so the box measures
the board used.

## 3. Placing one part (the decoder)

The search hands the placer an **order** of the instances and a **turn** for each. The
placer places them one by one, as today, on the raster with the gap. Where each one goes
depends on the goal:

- **Keep the top free:** the lowest place where the footprint fits, then the leftmost
  (bottom-left fill, Jakobs 1996, nesting.md §4.3).
- **Keep the right side free:** the leftmost place, then the lowest.
- **Compact:** the place where the box round everything placed so far grows least, then the
  lowest, then the leftmost. The places tried are, for every column, the lowest place in it,
  and for every row, the leftmost place in it: the places a part slid down or left would come
  to rest, so no candidate floats in the open.

**As built (steps 1 and 2):** for compact, every column's and row's place was too slow for
a search that places hundreds of layouts. Two places are tried: one growing the box upward,
the lowest place within the box's width (else, the part wider than the box or no room in
it, the leftmost anywhere), and one growing it rightward, the leftmost within its height
(else the lowest anywhere). A place inside the box is the lowest within its width, so a
hole in the block is still filled. The box is counted in cells from the corner, so scores
compare exactly. A row (or column) is passed over at once when the footprint's longest run
finds no free gap as long on its line, within the cells the footprint may use.

Today's slack rule (up to 5 mm farther along when that is lower across, `ALONG_SLACK_MM`)
goes: the goals and the search take its place.

**The turn** is the one the search gave. If the footprint doesn't fit anywhere in that turn,
the placer tries the part's other allowed turns in their order (nesting.md §3, `turns`), and
the turn it used is what the layout records. Only when no turn fits is the instance left
over (what doesn't fit, nesting.md §4, or the next board with overflow).

## 4. The search

The method is a local search over orders and turns, with the placer of §3 to turn each
candidate into a layout. Searching over the sequence and decoding it by a bottom-left rule is
from Jakobs, *On genetic algorithms for the packing of polygons*, EJOR 88 (1996), and Burke,
Hellier, Kendall & Whitwell, *A new bottom-left-fill heuristic algorithm for the
two-dimensional irregular packing problem*, Operations Research 54 (2006), which improves the
order by hill climbing and tabu search. Swapping two parts as the basic move is the
2-exchange of Gomes & Oliveira, *A 2-exchange heuristic for nesting problems*, EJOR 141
(2002). The acceptance rule is late acceptance hill climbing: Burke & Bykov, *The late
acceptance hill-climbing heuristic*, EJOR 258 (2017).

1. **Start:** today's four orders (largest area, longest side, widest in X, tallest in Y),
   each placed with the placer choosing the turn as it goes: for every instance, the allowed
   turn whose place scores best (§2). Their turns are recorded, and the best of the four is
   where the search starts. Each part's first instance placed is also tried in each of its
   allowed turns as a start of its own, so the first part's turn, which shapes everything
   after it, is chosen, not taken as drawn.
2. **A move** changes the best-so-far candidate in one of three ways, picked at random:
   - **swap** two instances in the order;
   - **move** one instance to another place in the order;
   - **turn** one instance to another of its allowed turns.

   A move that changes nothing (two instances of the same part in the same turn swapped, a
   part with no other turn) is drawn again, at most a few times, then skipped.
3. **Accept** the new candidate when its layout scores no worse than the current one, or no
   worse than the current one did a fixed number of steps ago (late acceptance, a list of
   fixed length). That lets the search walk through layouts slightly worse than the best on
   its way to better ones, with no temperature to tune, and it compares scores only by
   their order, which suits the score of §2, a list of measures, not one number.
4. **Keep** the best layout seen.
5. **Stop** after a fixed number of layouts (the **budget**), or on Cancel. The progress bar
   counts layouts tried.

**Deterministic.** The random choices come from a small generator written here, seeded with a
fixed number (SplitMix64: Steele, Lea & Flood, *Fast splittable pseudorandom number
generators*, OOPSLA 2014). The budget is a count, never a time, so the same project and
settings give the same placements, and the same G-code, on any computer. The budget is a
constant in the code, set by measuring in step 1 so that 20 instances of mixed parts take
1 to 2 s in a release build, and 50 instances on a 2440 × 1220 board stay under 10 s
(nesting.md decision 3). It may depend on the number of instances, never on the clock.

**As built (step 2):** 600 layouts for up to 20 instances, fewer beyond by the square of
the share (a layout costs more with more instances, each on a fuller board), at least 100;
the late acceptance list is 30 long. Measured (release): 20 mixed instances on 1200 × 600,
1.3 s for compact, 0.3 to 0.5 s for the others; 50 on a full sheet, 2.5 s. The search never
gives a worse layout than its starts (the best is kept); on 24 mixed parts it places 21
where the starts place 20. Swapping two instances of the same part in the same turn is
never drawn, as it changes nothing. The Nest window shows the share of layouts placed as a
progress bar.

**With overflow** (parts-and-setups.md §9), each board is searched in turn with the whole
budget: the first board takes as much as it can (§2.1), what is left goes to the next.

## 5. Nest again tries harder, never worse

*Nest again* (and `chipload nest` on a setup already nested) starts the search from the
layout the board has now, as well as from the four orders: its instances in the order of
their places under the goal (lowest first for *compact* and *keep the top free*, leftmost
first for *keep the right side free*), each in its turn now. It is scored like any layout.

- If the search finds a better layout, it is applied, one undo step, as today.
- If not, nothing moves and the status says so: "No better nest found; the board is as it
  was."

So pressing *Nest again* repeatedly can only improve the board, and each press searches on
from where the last one ended. No *try harder* setting is needed.

**As built (step 3):** the search's random numbers are seeded from the board as it is
(FNV-1a over each instance's lie and corner), so a search from a better board takes new
steps. After "No better nest found" the board hasn't changed, so pressing again gives the
same answer: there is no hidden state to search on from. An instance turned by hand to an
angle its part's turns don't allow, or one that meets another or lies off the usable area,
counts as not placed (its place is rounded to the nearest cell to be scored). A board that
takes what didn't fit on another (overflow) is nested afresh, though it carries the first
board's settings. Unchanged, the placements are not written back, and the window makes no
undo step for no change; what doesn't fit there is switched off as in any nest. `--json`
says `unchanged`.

The layout now is scored from the instances' footprints where they are (§2), so a board
placed or adjusted by hand is compared fairly. A hand-placed board whose parts overlap, or
lie off the usable area, scores as having placed only those that don't, so the search will
replace it.

## 6. The file (no schema bump)

`setup.nest` gains an optional `goal`: `"compact"`, `"low"` or `"left"`. Missing means
`compact`. Set by *Apply* and `chipload nest --goal`, kept for *Nest again*. An optional field
with a safe default needs no bump (CLAUDE.md); nothing already nested moves, since nesting
runs only when asked. Saving writes it explicitly when `nest` is written.

## 7. In the window and on the command line

- **The Nest window** gains *Goal*: three choices, *Compact*, *Keep the top free*, *Keep the
  right side free*, with a hover text each saying what is left of the board. It starts at the
  setup's kept goal, else *Compact*.
- The preview runs the search in the background, as the fill does now, with the progress
  bar and Cancel.
- **The summary line** says the box: "8 of 8 placed, in 742 × 380 mm of the board's
  1220 × 610 (38 %)". The percentage is the box's area over the usable area's. The setup
  sheet's *Nested* fact says the same box.
- `chipload nest … [--goal compact|low|left]`. `--json` gains `goal` and `extent: [x, y]`
  (the box's size, mm); `used_pct` becomes the box's share of the usable area. `used_x` goes:
  nesting has not been released, so nothing outside depends on it.
- Translations: the three goal names, their hover texts, the "No better nest found" status
  and the new summary line, in the seven languages.

## 8. Tests

- **Goals:**
  - two equal 300 × 200 parts on a 600 × 1200 board: *compact* never puts the second beside
    the first turned (a 500 × 300 box); *low* puts both on the bottom, side by side; *left*
    stacks them at the left;
  - for each goal, a property test: no single move of the result's order or turns gives a
    better score than the result, for small cases (4 to 6 instances) checked by trying
    every move.
- **The search:**
  - never worse than today's best of four orders (a property test over random rectangles,
    L-shapes and rings);
  - a mixed case where the fixed orders leave a part off the board and the search places it
    (a test fixed by its seed, so it stays meaningful);
  - the same input twice gives the same placements, bit for bit; the same with Cancel never
    pressed and the budget unchanged;
  - Cancel stops it and nothing is applied.
- **Nest again:** never makes the score worse; on a board already at its best, nothing moves
  and the status says so; a hand-placed overlapping board is replaced.
- **The first part's turn:** a long part that only leaves room for the others when it is
  turned first.
- **Everything nesting.md §13 asks still holds:** the gap measured exactly on the moves, the
  board's edges, nothing lost, the verification, the performance bound.
- **The user's tall-board project**, once they send it, in the corpus with a test first.
- **Golden:** the two nested goldens (v40_nest_badge, v41_nesting_demo) are nested by the
  test, so they change with the placer, in the steps that change it, each explained in its
  commit.

## 9. Order of work

Each step a commit with its tests, the guide and translations with the step that shows them:
1. **The goals and the placer** (§2, §3) in `chipload-toolpath` `nest/`, the score, and the
   first part's turns tried; the fixed orders as now. The budget measured. **Built
   2026-11-18:** the pipeline nests *compact* until step 4; its summary still says the
   board used along its longer side. Measured (release): 50 instances, quarter turns, on a
   2440 × 1220 board: 20 starts in 1.5 s, about 75 ms a layout with the placer picking the
   turns, a quarter of that with them given.
2. **The search** (§4), with its generator, progress and Cancel. **Built 2026-11-18.**
3. **Nest again from the board as it is** (§5). **Built 2026-11-18.**
4. **The goal in the file, the window and the command line** (§6, §7), the summary line, the
   setup sheet's fact, the guide (nesting page) and translations.

## 10. Clean room

Built from the papers named in §3 and §4 and first principles only. No nester's source
(Deepnest, SVGNest and the like) is read, and no other program's layouts are used as a model
(nesting.md §12, CLAUDE.md).

## 11. Decisions

1. **Three goals: compact (default), keep the top free, keep the right side free.**
   **Recommended** (the user's idea). Alternative: one goal, the compact box.
2. **The box, not the convex hull**, as compact's measure. **Recommended:** what is left of
   a board is sawn in straight lines, and the box keeps it rectangular; the hull rewards
   staggered blocks that leave triangles. Alternative: the hull's area.
3. **Local search over order and turns, decoded by the placer, late acceptance.**
   **Recommended:** it improves the longer it runs, it needs no tuning of a temperature, and
   it compares scores only by order. Alternatives: a beam search over partial layouts (the
   same cost every time, no improving with more time); a genetic algorithm (more parts to
   tune for the same result on a few dozen instances).
4. **A budget counted in layouts, never in time**, for the same G-code on every computer.
   **Recommended.**
5. **Nest again searches on from the board as it is and never makes it worse.**
   **Recommended**, instead of a *try harder* setting. Alternative: Nest again nests afresh,
   as today.
6. **The places tried for compact** are the lowest in each column and the leftmost in each
   row. **Recommended:** parts come to rest against something, as if slid. Alternative:
   every free cell (far slower, rarely better). *As built:* four places, not every column's
   and row's (§3 as built).
7. **Names:** *Compact*, *Keep the top free*, *Keep the right side free*; `compact`, `low`,
   `left` on the command line and in the file. **Recommended:** they say what the user gets.
