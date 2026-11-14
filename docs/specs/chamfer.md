# Chamfer: a V-bit chamfer on any outline

Status: **draft 2026-11-14**, for review (TODO *Ideas*, "More 2.5D toolpaths", item 1; the user asked for it after the pocket finishing pass).

## 1. What it is for

Most wooden parts look finished, and stop splintering at the edge, once their top edge is
broken with a small chamfer: a box, a tray, a shelf, a sign, the rim of a pocket or a hole.
Today only *New sign…* makes one, as a follow path centred on the sign's outline
(sign-maker.md §6). Any other shape needs that worked out by hand: a follow path, its depth
from the bit's angle, and a groove that is half in the waste. A **Chamfer** operation takes
the width you want and does the rest, on any closed shape, outside or inside.

## 2. What is cut

A closed shape, its waste on one side (`side`):

- **outside:** the shape is a part's outline, the waste around it (a box side, a sign);
- **inside:** the shape is a hole's or a pocket's rim, the waste within it.

With a V-bit of included angle α (half angle θ = α/2) and tip diameter *t* (0 for a pointed
bit), a chamfer of width *w* at the surface, measured from the drawn line into the part:

- The tool centre runs **t/2 into the waste**, parallel to the line (an offset, verified
  as every offset is).
- It goes down to **d = w ÷ tan θ** below the start depth. The bit's flank then passes
  through the surface *w* into the part and meets the line's wall at depth *d*: a 45°
  chamfer *w* wide and *w* deep with a 90° bit; a steeper one with a 60° bit.
- The flat of the tip stays in the waste, so a flat-tipped bit leaves no step on the part.
  The other flank grooves the waste, which the cut-out or the pocket takes away.

Worked example: a 90° bit with a 0.2 mm flat, *w* = 2 mm → the centre 0.1 mm outside the
line, 2 mm deep.

## 3. Corners

- **The part's outside corners** (convex): the centre goes round them on an arc of radius
  t/2, so the chamfer turns round the corner as a cone, as a chamfer bit would. With a
  pointed bit the corner stays sharp and the two faces meet in a mitre.
- **The part's inside corners** (concave): the offset has a corner there and the two faces
  meet in a mitre. The wall below keeps the cut-out's own radius; the chamfer cuts into it.

## 4. Passes, depth and limits

- One pass by default. With a step down, a deep chamfer goes down in equal passes, the last
  at *d* (as a follow path's passes).
- Refused when *d* is deeper than the bit's cone (the V-bit's cutting height from its
  angle and diameter): the shank would rub. The message gives the widest chamfer the bit
  can make.
- Refused with a tool that isn't a V-bit (an end mill has no flank to chamfer with), with a
  width of 0 or less, and on open shapes, which stay follow paths (a note names them, as a
  pocket names open shapes it skips).

## 5. Format

A new operation type (schema 37; an older Chipload refuses the project):

```json
{"type": "chamfer", "name": "", "enabled": true, "part": 1, "material": null,
 "select": {"shapes": [1]}, "tool": 2, "side": "outside", "width": 2.0,
 "start_depth": 0.0, "step_down": null, "direction": "climb",
 "feed": 1500.0, "plunge_feed": 500.0, "entry": {"type": "plunge"}}
```

- `width`: mm at the surface, more than 0.
- `side`: `outside` or `inside`: where the waste is.
- `start_depth`: where the edge is, mm down from the stock top: 0 for the top, more for
  the edge of a step an earlier operation cut.
- `step_down`: null for one pass.
- `direction`: climb or conventional, as a profile.
- `entry`: `plunge` or `ramp` along the path.

Migration 36 → 37 changes nothing in the file: chamfers are new. The frozen sample is a v36
project, and its golden G-code must not change.

## 6. Command line and window

- **Command line:** `chipload op add part.chipload chamfer --shapes 1 --width 2 --tool 2`
  (also `--layer`, `--side inside`); `set …operations.N.width=3`.
- **Window:** *Chamfer* in the ribbon's add-operation list and the Q menu (key **C**). Its
  properties: Tool (V-bits only), Side, Width, then the depth it makes shown beneath ("2 mm
  deep with the 90° bit"), Start depth, Step down, Direction, Feeds, Entry.
- **Defaults** for a new chamfer: the project's first V-bit; side from the shapes (outside
  for an outermost shape, inside for one within another, as a new profile's); width 1 mm;
  the V-bit's cutting data for its feeds.
- **Setup sheet:** "chamfer 2 mm (2 mm deep, 90° V-bit), outside".
- **Guide:** a page *Chamfer* under Operations.
- **Sign maker:** *New sign…* adds a Chamfer operation for its edge from then on. Signs made
  before keep their follow paths, which stay valid.

## 7. Cut order

A chamfer on a part's outline is best cut **before** the cut-out, while the part is still
held by the board. A new chamfer is therefore placed just before the first profile in its
part that cuts the same shapes, and otherwise at the end of the part, like any new
operation. It can be moved like any other.

## 8. Edge cases and tests

- A rectangle outside and inside, with a pointed and a flat-tipped bit: the centre path is
  t/2 from the line on the waste side, and the depth is w ÷ tan θ (60° and 90° bits).
- Simulated: the height map at the line is *d* deep, and at *w* into the part it is 0.
- A circle (all one arc, kept as arcs in the G-code), a shape with inside corners.
- A step down that divides the depth evenly, and one that doesn't.
- Refused: deeper than the cone, an end mill, width 0; open shapes skipped with a note.
- Start depth above 0 (the edge of a step).
- Climb and conventional.
- The checker passes every program; the v36 sample migrates with an unchanged golden; a
  new golden: a box side with an outside chamfer and a hole with an inside one.

## 9. Order of building

1. Format, migration, validation, planning, the command line, goldens.
2. Window, Q menu key, sheet, guide.
3. The sign maker's edge onto it.

## 10. Decisions

1. **A new operation type, *Chamfer*** (§5), with a schema bump. *Recommended:* it is a
   thing users ask for by name and look for in the list. The alternative, a chamfer option
   on a follow path, needs no bump but hides the feature and gives a follow path two
   meanings.
2. **The bit's flank on the line** (§2): the centre t/2 into the waste, the depth from the
   width. *Recommended:* the width you type is the width you get, with no step from a flat
   tip. The alternative, the centre on the line (as the sign maker does now), is simpler
   but makes the chamfer narrower than typed by t/2 and leaves a step with a flat tip.
3. **Width as the setting** (§5), the depth shown beneath it. *Recommended:* the width is
   what the drawing or the customer gives. The alternative is typing the depth.
4. **Closed shapes only** (§4); open shapes stay follow paths. *Recommended.*
5. **One pass, or equal passes with a step down; refused deeper than the cone** (§4).
   *Recommended.*
6. **V-bits only** for now (§4); a round-over bit when tools can describe one (sign-maker.md
   §8). *Recommended.*
7. **Placed before the profile that cuts the same shapes** when added (§7). *Recommended.*
8. **Corners as in §3**: round over outside corners by t/2, mitres elsewhere, no option.
   *Recommended.*
9. **The sign maker moves onto it** (§6); old signs keep their follow paths, with no
   migration. *Recommended.*
10. **Q menu key C** (§6). *Recommended.*
