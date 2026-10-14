# Spec: a corner radius for drawn rectangles

Status: approved 2026-10-14 (the user: "go ahead", with the recommendations below).

## 1. Why

User request (2026-10-14): "the rectangle shape should have a corner radius option". Rounded
rectangles are common (boxes, panels, pockets that a round tool can follow), and drawing
them today needs a polyline with bulges or a DXF.

## 2. The value

A drawn rectangle gets **`corner_radius`**, mm, `0 <= r <= min(length, width) / 2`. 0 is a
sharp corner, as before. All four corners have the same radius.

- **Segments:** with `r = 0` the four lines as before, exactly. With `r > 0`, counter-
  clockwise from `r` right of the local lower-left corner: bottom line, arc, right line,
  arc, top line, arc, left line, arc. Each arc is a quarter circle of radius `r` about the point `r` in from
  both sides of its corner, turned with the rectangle. A side as long as `2r` (within the
  chaining tolerance) has no line: `r = width / 2` gives a slot shape, and a square with
  `r = side / 2` gives four arcs that make a circle.
- **Arcs, not segments:** the corners are true arcs, so the G-code has G2/G3 where the post
  has arcs on (lines within the arc tolerance where it has them off).
- **Too large a radius is refused**, not shrunk: a problem on the shape
  (`corner_radius: at most N mm, half the shorter side`). Dragging a corner handle smaller
  than the radius allows stops there, as any drag that would make the shape invalid does;
  lower the radius first.
- **Mirroring and moving** keep the radius.

## 3. The file (schema 25)

- `corner_radius` is a required field of a `rectangle` shape (with `center`, `length`,
  `width`, `angle`); saving always writes it.
- **Migration 24 → 25:** every rectangle gets `corner_radius: 0`, so it stays as it was.
- The sample `tests/corpus/jobs/v24/rectangle_part.chipload` (v14_round_corners frozen at
  v24) migrates and gives the same G-code. A v25 sample with a rounded rectangle gets
  golden G-code of its own.

## 4. Command line

- `shape add rectangle … --corner-radius R` (default 0).
- `shape set ID --corner-radius R`; refused for a circle or polyline.
- `shape list` / the shape's values show `corner radius R` when it isn't 0.

## 5. App

- The rectangle's form gets a **Corner radius** row (mm, not below 0) with a reset button
  back to 0, applied like its other values.
- Drawing a new rectangle gives radius 0.

## 6. Decisions

1. **One `corner_radius` field, schema 25 with a migration.** *Approved.*
2. **Too large is refused** (at most half the shorter side). *Approved.*
3. **True arcs.** *Approved.*
4. **CLI:** `--corner-radius` on `shape add` and `shape set`. *Approved* (proposed as
   `--radius`; named `--corner-radius` because `--radius` is the circle's).
5. **App:** a Corner radius row with reset. *Approved.*

## 7. Tests

- `r = 0` gives the same four lines as before; `r > 0` gives 8 segments that join end to
  start within the chaining tolerance, arcs of radius `r`, and the same box.
- `r = width / 2` leaves out the short sides (6 segments); a square at half its side gives
  4 arcs.
- A radius above half the shorter side, below 0 or not finite is a problem.
- Turned by an angle, the segments are those of the unturned rectangle turned about the
  centre.
- Migration: a v24 rectangle gets `corner_radius: 0`; the v24 sample's golden G-code holds.
- CLI: `shape add rectangle --corner-radius`, `shape set --corner-radius`, refused on a
  circle.
