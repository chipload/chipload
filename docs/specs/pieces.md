# Spec: pieces, a big shape cut as a puzzle

Status: **approved 2026-11-15**, all decisions as recommended. The user's choices: "Tiling" means splitting a
shape larger than the board into pieces that lock together. The shapes are split and the
operations follow; the joints are puzzle knobs or fingers. The other reading, sliding one
long board between tiles, is parked in docs/specs/sliding-tiles.md.

As built, 2026-11-15:
- Steps 1–6 built (the window, step 7, and the guide's own page, step 8, to come).
- The pieces come from `toolpath::pieces` (`piece_set`). Each piece's program comes from
  `pipeline::pieces::run_pieces`: the outside profiles cut the piece's loops in a project of
  its own (its drawing only those loops, its board fitted round them), and every other
  operation's moves come from the whole setup's plan, trimmed by `pieces::Trimmer`.
- `run --piece`, `simulate --piece` (without it, the pieces put together on the whole
  board), the one sheet per setup with its map, and `dxf export --pieces` are built.
- `plan` still plans a setup in pieces as one board, without `--piece`.
- Joints stay two sizes from crossings, not one (§4).
- A board with a fixed size gives each piece a 10 mm margin (`DEFAULT_PIECE_MARGIN`).

## 1. Why

A 1.5 m round table top, a large wall sign, or a map of the country with its regions in
V-carve, on a machine or from boards that are smaller than the piece. Today the user would
have to cut the drawing apart by hand: draw the joints, split the outline, and move the
pockets and text across the seams so they still line up.

With **pieces**, a setup is cut as a puzzle:
- seams split it into pieces that each fit a board;
- each seam has joints that lock the pieces together;
- every operation follows: a groove or a letter that crosses a seam is cut partly on each
  piece, and lines up when the pieces are glued.

## 2. How it works, for the user

1. Draw the whole thing at full size, with its operations, as for one board.
2. In the setup's form, turn on **Pieces**. Give the size of the boards you have (proposed
   from the machine's travel) and choose the joint: *puzzle knobs* or *fingers*.
3. Chipload picks the fewest pieces that fit, such as "3 × 2 pieces", and draws the seams
   with their joints over the drawing, each piece named (A1, A2, …, B3).
4. Generate and export: one program for each piece (`table-A1.nc`, …), each on a board
   fitted round its piece, with its own zero.
5. Cut each piece, and mark it with its name as the setup sheet says. Glue them together
   following the sheet's map.

## 3. The seams (decision 2)

- **A grid:** C columns and R rows of cells over the setup's cuts, the seams evenly spaced.
  The count is the fewest whose pieces fit the board size, counting the knobs that stick
  out and the setup's margin. It can also be given ("3 × 2").
- **Seams can be moved:** each seam's position is a length from the even position, given
  in the form or on the command line, and validated (no two seams closer than three joint
  sizes). This keeps a seam out of a letter or a narrow spot.
- **Only the material is split.** A seam runs from edge to edge of the cuts. Where it
  crosses empty space (outside the outline, or through a hole), it does nothing.
- **Pieces are named** by row letter and column number, from the bottom left: A1, A2, …
  B1. A cell with no material in it is no piece and has no program.

## 4. The joints (decision 3)

Each seam is one curve, shared by the pieces on both sides. Each side cuts the same curve,
so the pieces fit, apart from the clearance (§5).

**Puzzle knobs:** a round head on a narrower neck, as on a jigsaw puzzle.
- The head's diameter is the **joint size** (default 30 mm). The neck is 0.6 times the
  size, and the head stands about one size out from the seam (*as built: 1.19 sizes; the
  fillets are 0.2 sizes in radius, so the tool's radius plus the clearance may be at most a
  fifth of the size*).
- The knob is joined to the seam line by arcs tangent at both ends. The curve is all lines
  and arcs, with no corners, and every arc has a radius of at least the cut-out tool's
  radius plus the clearance. Otherwise the router can't cut the socket.
- Knobs sit every **spacing** along each stretch of seam that crosses material (default four
  sizes), centred in the stretch. A stretch too short for one knob stays straight.
- Knobs **alternate** in and out along a seam, so neighbouring pieces get both knobs and
  sockets.
- Knobs stay at least one size from the outline's edge, and two sizes from where seams
  cross. The socket's thin walls would break there, and two seams' knobs would meet at a
  corner (*as built: two sizes from crossings, one from the edge; fingers the same*).

**Fingers:** square teeth along the seam, with every corner rounded.
- The tooth width is half the spacing, and the tooth length is the joint size (default
  15 mm for fingers).
- The corner radius is the cut-out tool's radius plus the clearance, on both the outer and
  inner corners. Since it's one shared curve, the rounded tip of one piece fits the rounded
  root of the other exactly. Fingers need no dogbones.

**A check:** a joint the cut-out tool can't cut (its tightest arc smaller than the tool's
radius, or a neck narrower than the tool) is a problem on the setup, which names the seam
and the tool. The fix is a bigger joint size, or a smaller tool.

## 5. Fit (decision 4)

- **Clearance** (default 0.2 mm in all): each piece's seam edge is moved inward by half of
  it, so the joints slide together with room for glue. The outline's own edges don't move.
- The setup sheet names it ("joints cut 0.2 mm loose for glue"). Too tight shows only once
  the pieces are cut, so the default leans loose.

## 6. What each piece cuts (decision 5)

The **body** is the material the setup's outside profiles cut out: their outer outlines with
their holes, by nesting. Each piece's body is the body intersected with its cell, the
seams having their joints and clearance.

1. **The cut-out:** each outside profile that cuts the body cuts the piece's own loops
   instead: its share of the outline, plus the seam edges. Everything else on that
   operation stays as set (side, depths, laps, finishing).
   - Tabs set by count or spacing are laid out again on each piece.
   - Tabs placed by hand are kept where their spot lies on the piece's loop, and dropped
     elsewhere, with a note.
2. **Everything else** (pockets, V-carves, text, flutes, holes, chamfers, reliefs) is
   planned **once, on the whole drawing**. Each toolpath is then clipped to the piece's cell,
   grown by the operation's tool radius.
   - A V-carve is clipped, not re-planned. Its depths come from the whole letter, so the
     groove matches on both sides of a seam. Re-planning a cut-off letter would make its
     depth go to zero at the seam.
   - Growing the cell by the tool radius means each side cuts a little past the seam, into
     what is waste on its own board. No ridge is left at the seam.
   - Where a cut is clipped, it ends with a retract and starts again with a plunge at the
     plunge feed. In a pocket cut in levels, or a profile cut in laps, the plunge goes at
     most one step down into fresh material. A V-carve or other 3D cut that's split gets a
     note: "split across pieces A1 and A2: the cutter plunges at the seam".
3. **A cut wholly inside one piece is that piece's alone.** A hole near a seam isn't cut
   twice, because a piece's growth into its neighbour never reaches past the seam into the
   neighbour's own cuts.
   - Precisely: a toolpath share is kept for a piece only where the tool's centre lies within
     the piece's cell grown by the tool radius. A whole cut inside one cell lies wholly
     outside the others' growth only if it's more than a tool radius from the seam.
   - Closer than that, both pieces cut it on their own boards, into their own waste. That's
     right: each board needs it cut.

## 7. Boards, zeros, files

- **Each piece is its own board:** fitted round the piece (knobs included) with the setup's
  margin, at the setup's thickness and material.
- **Its zero:** the setup's work zero, applied to the piece's board. A stock corner becomes
  the same corner of each piece's board; the board's centre becomes its centre. A drawing
  origin or a point that falls off the piece's board falls back to its bottom-left corner,
  with a note.
- **Each piece is its own program,** checked by the checker on its own: with its own board
  as the stock, its own zero and the machine's travel.
- **Files:** `out-A1.nc`, `out-A2.nc`, …; with several tools, `out-A1-1-T1.nc`; with several
  setups, `out-S2-A1.nc`.
- `run --piece A1` writes one piece. `plan`, `sheet` and `simulate` take `--piece` too.
  Without it, `plan` gives the time per piece and the sum, and `simulate` simulates the
  pieces put together.
- `dxf export --pieces` writes the piece outlines, each on a layer named after it, for
  anyone who wants them as a drawing.

## 8. The setup sheet

- **A map:** the whole drawing with the seams and the piece names, as it will go together.
- **For each piece:** the board size to cut ("A1: board 612 × 418 × 18 mm"), its zero, its
  programs, and "write A1 on the back in pencil before you unclamp".
- **Gluing:** "dry-fit first; joints are 0.2 mm loose; glue the rows, then join the rows".

## 9. The window

- **The setup's form:** a **Pieces** section, folded until it's turned on:
  - *Board size*, proposed from the machine's travel;
  - *Grid*, automatic or columns × rows;
  - *Joint: puzzle knobs / fingers*, with its *size*, *spacing* and *clearance*;
  - the result: "3 × 2 pieces, the largest 590 × 410 mm on a 612 × 432 mm board";
  - a seam list with each seam's shift.
- **The view:** the seams with their joints drawn over the drawing, the piece names in the
  cells, and each piece's board drawn faintly.
  - Dragging a seam shifts it. That drawing code stays thin: the drag sets the seam's
    shift, and the pipeline works out the pieces.
- **The toolpath** shown is the active piece's, chosen by clicking a piece, or the whole
  plan when none is chosen.
- **Export** writes every piece of the active setup.
- **Simulate:** the pieces put together, or the active piece on its own board.

## 10. Command line and file

- `chipload setup pieces part.chipload [--setup N] --board 600x400 [--grid 3x2]
  [--joint knobs|fingers] [--size 30] [--spacing 120] [--clearance 0.2mm]
  [--shift-x 2=15] [--shift-y 1=-20]`, and `--off`.
- `run|plan|sheet|simulate --piece A1`.
- **The file:** a setup gets `pieces`:

  ```json
  "pieces": { "board": { "x": 600, "y": 400 }, "grid": null,
              "joint": "knobs", "size": 30, "spacing": 120, "clearance": 0.2,
              "shift_x": [0, 15], "shift_y": [-20] }
  ```

  It's optional (null: one board as today). `grid` is null when it's worked out from the
  board size; the shifts are per inner seam. **Schema 37**, a bump (decision 6). The
  migration from 36 sets `pieces: null`. It comes with a frozen v36 sample, and a golden:
  a 900 × 500 mm rounded rectangle with a pocket and a V-carved word across the seam, cut
  as 2 × 1 knob pieces.
- **Validation:**
  - board, size, spacing and clearance must be positive; the spacing at least two sizes;
  - the shifts must match the grid's inner seams;
  - pieces aren't allowed together with a back setup (two-sided) or with copies (§11);
  - the joints are checked against the cut-out tool (§4).

## 11. Not in this step

- **Laying several small pieces out on one board** (nesting). Each piece gets its own
  board for now; the sheet lists the board sizes, and the user cuts them from a sheet.
- **Seams other than a straight grid,** such as along a drawn line, or a region's border on
  a map. A good next step, using the same joints.
- **More joints:** dovetails, zigzag (triangular teeth), straight seams for biscuits.
- **Pieces with two-sided work or copies.**
- **Engraving each piece's name on its back.**

## 12. Correct means

- Each piece's loops are closed, tangent where the joint curve is, and the union of all the
  pieces, each grown by half the clearance, gives back the body (within the arc tolerance).
- Neighbouring pieces' seam edges are the same curve, offset by the clearance; checked by
  distance in tests.
- No joint arc is tighter than the cut-out tool's radius.
- A toolpath clipped to all pieces covers the whole plan. Each share stays within its cell
  grown by its tool radius. A property test checks both, over random seams and paths.
- Every piece's program passes the checker on its own board.

## 13. Decisions

1. **Pieces are a setting of a setup**, worked out when planning, not stored as shapes or
   setups. **Recommended:** the drawing and operations stay as drawn; changing the board
   size or the joint redoes everything; turning it off undoes it. Alternative: split the
   drawing into real shapes and one setup per piece. That's editable by hand, but every
   change after that is N times the work.
2. **An even grid of straight seams, each seam shiftable,** sized automatically from the
   board size (§3). **Recommended.** Alternative: seams along lines the user draws. That's
   more freedom (a map's borders), and a good later step.
3. **Puzzle knobs and fingers, every curve made of arcs no tighter than the tool,** knobs
   alternating in and out (§4). **Recommended.**
4. **Clearance 0.2 mm in all, by default,** on seam edges only (§5). **Recommended.**
5. **The cut-out operations cut each piece's loops; every other operation is planned once
   and its toolpath clipped to the piece's cell, grown by the tool's radius** (§6).
   **Recommended:** V-carves and patterns match across the seams. Alternative: clip the
   geometry and plan each piece afresh. That gives cleaner pockets, but V-carve depths go
   to zero at the seams and patterns lose their phase.
6. **Schema 37 for `pieces`, a bump.** **Recommended:** an older Chipload would otherwise
   cut the whole thing as one board, larger than the boards and maybe the machine.
7. **Order of work** (each step a commit):
   1. the model, schema 37 with its migration, validation, `setup pieces`;
   2. the seam curves and joints (`chipload-toolpath`, `pieces.rs`): knobs, fingers, the
      tool check, with tests of the shared curve and its radii;
   3. the piece bodies (booleans with the clearance), and the cut-out on piece loops;
   4. clipping the other toolpaths, with property tests;
   5. running per piece: boards, zeros, files, `--piece`, the checker, the report;
   6. the setup sheet, the simulation put together, `dxf export --pieces`, the golden;
   7. the window: the Pieces section, the seams in the view, dragging a seam, export;
   8. the guide page (*Setups ▸ Pieces*), schemas, translations.
   **Recommended.**
