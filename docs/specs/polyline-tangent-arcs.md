# Spec: arcs while drawing a polyline

Status: **draft, 2026-11-21**, for review (decisions in §7). The second item of step 2 of
the drawing review (TODO, *Next* 1; docs/specs/drawing-improvements.md).

Builds on drawing-tools.md (the polyline tool), drawing-input.md (keys, typed values,
Shift's 15° steps, the readout) and polyline-radii.md (the *Radius* while drawing).

## 1. What is missing today

A polyline is drawn with straight sides only. An arc in it means drawing the polyline
first, then typing a bulge into its table, or drawing an arc separately and joining the
two. A slot with a bent end, a cam, a handle or a bottle's shoulder, each one line into
an arc into a line, takes several tools and some arithmetic.

## 2. The tangent arc (general geometry)

From the last point P, where the path runs in direction t, to the end E:

- **One arc** starts at P going in direction t and ends at E. The angle between t and the
  chord P→E is half the arc's sweep (the tangent–chord angle, from elementary geometry).
- **As a bulge** (DXF meaning, as the polyline stores it): θ is the signed angle from t
  to the chord; the sweep is 2θ and the bulge is tan(θ / 2). No new stored value: the
  polyline keeps its points and bulges.
- **E straight ahead** (θ = 0): a straight side. **E straight behind** (|θ| near 180°):
  no arc. The arc would be a whole circle, so the click is refused with the reason, as
  for other shapes with no size.
- **The direction t:** the direction in which the side drawn last ends, whether a line or
  an arc. After an arc it is the arc's end direction, so arcs drawn one after another
  join smoothly.

The calculation is a pure function in `chipload-draw::construct` beside the other arc
constructions, with tests.

## 3. In the window

- **Two kinds of side while drawing a polyline:** *Line* (as today) and *Arc*. They are
  two toggle buttons on the status bar, beside *Radius*, each naming its key.
- **Keys:** with the polyline started (at least one point placed), **A** switches to arcs
  and **L** back to lines (decision 2). Before the first point, A and L start the Arc and
  Polyline tools, as today.
- **An arc side:** the preview shows the tangent arc from the last point to the cursor.
  The click places its end, and the next side goes on from there.
- **The first side as an arc** (decision 3): it needs a direction to go on from. When the
  polyline starts on the end of an open drawn polyline (snapped there), it goes on in that
  shape's end direction. Otherwise the first side is a line even in arc mode, and the
  status bar says why.
- **A line after an arc** goes wherever it is clicked, as any line does; the *Tangent* and
  *Perpendicular* snaps already help there. With a *Radius* set, a corner between an arc
  and a line gets rounded too (fillet-between-shapes.md §4). Where the sides meet
  tangentially there is no corner.
- **Closing:** a click on the first point closes the polyline with the kind of side in
  use: in arc mode, a tangent arc to the first point.
- **Backspace** takes away the last point and its side; the mode stays as it is.
- **Typed values** (drawing-input.md §2) give the arc's end point: `x,y`, `@dx,dy` and
  `d<a` as for a line. A plain `d` is the chord's length, towards the cursor. Typing an
  arc by its radius or its sweep is left for later (decision 5).
- **Shift:** holds the arc's sweep to multiples of 15°. The end lies on that arc at the
  cursor's distance from the last point (the chord's length), so a quarter or half circle
  is easy to draw.
- **Readout:** beside the cursor, the arc's radius and its sweep in degrees (for a line,
  its length and angle, as today).
- **A new polyline starts with lines,** whatever was used last (decision 4).

## 4. Command line

`--bulges` (shape add, shape set) takes `t` for a side that is a tangent arc: its bulge is
worked out from the side before it, as in §2. It may not be the first side, unless the
polyline is closed and its last side isn't a `t` itself.

```
chipload shape add P polyline --open --points "0,0 40,0 60,20 60,40" --bulges "0 t 0"
```

In this example the second side is the quarter circle from (40, 0) to (60, 20). Its
direction comes from the first side (along +X), so its bulge is tan(22.5°). The file
stores the number, never `t` (decision 6).

## 5. Tests

- **Construction:** a quarter and a half circle, an end straight ahead (bulge 0), one
  behind (refused), and the arc's start direction equal to t; a property test that the
  arc starts in direction t and ends at E.
- **Sketch:** line, arc, arc, line; closing with an arc; Backspace; the first side in arc
  mode (a line, and an arc when started on a shape's end); Shift's 15° sweeps; typed `d`.
- **CLI:** `--bulges` with `t`, including after an arc side; a `t` first, refused.
- **Window:** A and L switching while drawing; the status bar buttons; the preview.

## 6. Order of work

One commit: the construction, `t` in `--bulges`, then the window, the guide
(drawings/shapes.md) and the seven languages.

## 7. Decisions (recommendations first)

1. **Tangent arcs, one click per side:** the arc's end is clicked, its start direction
   is that of the side before it. *Alternative:* three-point arcs inside the polyline (an
   extra click through a point), which also gives arcs that aren't tangent. That job
   already belongs to the Arc tool, followed by *Join ends*.
2. **A and L switch the side's kind while a polyline is being drawn.** A key you can
   press without moving the mouse away from the drawing, and the same letters as the
   tools, so they are easy to remember.
3. **The first side in arc mode needs a direction:** taken from the shape the polyline
   starts on, else the first side is a line. *Alternative:* the first arc starts along
   +X, which gives arcs that are seldom what was meant.
4. **A new polyline starts with lines.** Arc mode doesn't carry over to the next
   polyline, so the tool never surprises with an arc.
5. **No typed radius or sweep yet:** the end point, typed or snapped, fixes the arc.
   Later, if missed.
6. **`t` in `--bulges` on the command line,** so the feature works from the command line
   before the window, as CLAUDE.md asks. The stored polyline keeps plain bulges.
