# Fluting: grooves that ramp in and out along a path

Status: **draft 2026-11-15**, for review (twelve decisions, §12). TODO *Ideas*, "More 2.5D
toolpaths", item 2; the user asked for it after the Compass ("we do it before fluting").

## 1. What it is for

A **flute** is a decorative groove: down the face of a column, a table leg or a pilaster,
across a panel, radiating as a sunburst, or along a frame. What makes it a flute and not an
engraved line is its ends: the groove dives in gradually, runs at full depth, and comes up
gradually again, so with a ball nose or a V-bit it narrows to a point at each end.

A follow path can't make that: it goes down at the start and comes up at the end, so its
ends are as wide and deep as its middle. Today a flute needs a 3D model and a relief. A
**Flute** operation takes the path, the depth and how long each end ramps.

## 2. What is cut

The selected **open** shapes (lines, arcs, polylines, open drawn shapes, a DXF's open
contours). Each one is cut on the line, its tool centre following the shape exactly, as a
follow path does (follow-path.md §1). Along each shape the depth goes:

1. from the **start depth** at the shape's first end,
2. down over the **start ramp** length (measured along the shape) to the **depth**,
3. at the depth through the middle,
4. up over the **end ramp** length to the start depth at the shape's last end.

A ramp of 0 is a square end: the flute is at full depth right at that end (a flute that
runs off the board's edge, or into a moulding). Both ramps 0 is a follow path at one depth.

- **First and last end** are the shape's drawn start and end. Most flutes have the same
  ramp at both ends, so it rarely matters; where it does, the canvas shows which end is the
  start when the operation is selected (a small marker, as a profile's start point).
- **Ramps longer than the shape** (start + end > length): both are shortened in the same
  proportion, so the flute reaches its depth at one point only, and a note says so with
  the shape's number. It is never refused: a short flute among long ones is still cut.
- **Closed shapes** are skipped with a note, as a chamfer skips open ones: a closed flute
  has no ends to ramp. A closed groove is a follow path.

## 3. The ramp's shape

| Ramp | Depth along the ramp (u from 0 at the end to 1 at full depth) | Looks |
|---|---|---|
| **Straight** | d · u | an even taper; with a V-bit, straight flanks meeting in a point |
| **Curved** | d · sin(π/2 · u) | steepest at the surface, easing into the floor without an edge, so no line shows where the ramp meets the flat |

*Straight* is the default. Both are used for flutes and look different; neither is the worse
one (decision 4).

## 4. Passes

The depth goes in passes when it is deeper than the step down (none: one pass). Each pass
follows the same profile **cut off at its own level**: pass *k* goes no deeper than *k* ×
the step (equal steps, the last exactly at the depth, as every operation's passes). So:

- the ramps keep their shape and slope in every pass; the floor gets deeper;
- under the tool there is never more than one step of uncut material, on the ramp or the
  floor;
- the last pass is the flute's exact profile.

**Back and forth** (decision 5): pass 1 from one end to the other, pass 2 back, and so on,
as a follow path's open paths. At an end with a ramp the tool is at the start depth between
passes, so nothing is plunged; at a square end it goes straight down one step at the plunge
feed. After the last pass it retracts. Shapes are taken in nearest-neighbour order from
the nearer end (follow-path.md §3); the profile belongs to the shape, so it reads the same
which way it is cut.

## 5. Tools

| Tool | Flute |
|---|---|
| **Ball nose** (plain or tapered) | a round-bottomed flute, the classic; width at the top 2·√(d·(2r − d)) while d ≤ r |
| **V-bit** | a V-groove; width at the top t + 2·d·tan(θ). Refused deeper than its cone, as a follow path |
| **End mill** | a flat-bottomed groove with square sides and sloping ends; a square end (ramp 0) needs a centre-cutting end mill, as a follow path's plunge |
| **Drill** | refused ("use a holes operation") |

The report and the setup sheet give the width at the top at full depth ("the flute is
6 mm wide at the top"), as for a follow path's V-groove.

## 6. Moves and checks

- XY follows the shape exactly; Z follows the profile. Where Z changes evenly along an arc
  (a straight ramp, or the flat floor), the move is a helical G2/G3; a curved ramp is cut in
  pieces short enough that the depth is within the arc tolerance (0.01 mm) of the curve.
- Points are added where the profile has a corner: the ends of each ramp and where a pass's
  level cuts the ramp.
- Feeds: the feed everywhere along the shape, ramps included; the plunge feed only for
  straight-down moves (the start depth at the first end, a square end).
- The checker runs as on every program. The simulation shows it (ball nose, V-bit and end
  mill shapes exist already).
- Shapes shorter than the chaining tolerance are skipped with a note, as in a follow path.

## 7. Format

A new operation type; no schema bump, as the chamfer (chamfer.md §5): an older Chipload
refuses a project with a type it doesn't know.

```json
{"type": "flute", "name": "", "enabled": true, "part": 1, "material": null,
 "select": {"shapes": [4, 5, 6]}, "tool": 3,
 "start_depth": 0.0, "end_depth": 4.0, "step_down": 2.0,
 "ramp_start": 20.0, "ramp_end": 20.0, "ramp_shape": "straight",
 "feed": 1500.0, "plunge_feed": 500.0}
```

- `end_depth`: the depth through the middle, mm down from the stock top, more than
  `start_depth`. No `through`: a flute never goes through.
- `ramp_start`, `ramp_end`: mm along the shape, 0 or more.
- `ramp_shape`: `straight` or `curved`.
- `step_down`: null for one pass.
- No `side`, `direction`, `entry`, tabs or leads: the flute is on the line, and the ramps
  are its entry.

## 8. Command line and window

- **Command line:** `chipload op add part.chipload flute --shapes 4,5,6 --depth 4 --ramp 20`
  (`--ramp` sets both ends; `--ramp-start`, `--ramp-end` one each; `--ramp-shape curved`;
  `--tool`, `--layer` as every `op add`); `set …operations.N.ramp_end=0`.
- **Window:** *Flute* in the ribbon's add-operation list and the S menu (key **L**: F is
  the follow path's). Its properties: Tool, Start depth, Depth, with the width at the top
  shown beneath ("6 mm wide with the 6 mm ball nose"), Step down, Start ramp, End ramp,
  Ramp (straight / curved), Feeds.
- **Defaults** for a new flute: the project's first ball nose, else its first V-bit, else
  the operation's usual tool; depth: a ball nose's radius (a half-round flute), otherwise
  3 mm, no deeper than a V-bit's cone; both ramps 20 mm, straight; the tool's cutting data
  for the feeds.
- **Setup sheet:** "flute 4 mm deep, ramps 20 / 20 mm, straight (6 mm ball nose)".
- **Guide:** a page *Flute* under Operations, with a picture of a sunburst panel.

## 9. Edge cases

- Ramp 0 at one end, at both ends.
- Ramps exactly the shape's length together; longer (shortened, with a note).
- A shape of one arc, and a polyline of lines and arcs where a ramp ends mid-arc.
- A step down that divides the depth evenly, one that doesn't, none.
- Start depth above 0 (a flute in the floor of a pocket).
- V-bit deeper than its cone refused; a drill refused; closed shapes skipped.
- A shape drawn the other way: its ramps swap ends.

## 10. Tests

- Toolpath: the final pass's depth at every point equals the profile (straight and curved,
  within the arc tolerance); every pass's depth is the profile cut off at its level; the
  depth removed by one pass at any point is at most one step; XY of every cutting move lies
  on the shape; back and forth with no retract between passes when both ends ramp; the
  plunge at a square end at the plunge feed.
- A proptest: random polylines with arcs, ramps and steps: the three invariants above.
- Plan: refusals and notes; the width note for each tool kind.
- Simulation: the height map along a flute is the profile (sampled at the start, the ramp's
  middle, the floor).
- Golden: a new `v36/flute.chipload` (three flutes: a ball nose with straight ramps, one
  with a square end, one along an arc with curved ramps) and its `lines/` twin; existing
  goldens unchanged.
- CLI: `op add flute` and its options; the GUI: the S menu's L, the form's fields.

## 11. Order of building

1. Format, validation, planning, the command line, goldens.
2. Window: the form, the ribbon, the S menu, the start marker; sheet; guide.

## 12. Decisions

1. **A new operation type, *Flute*** (§7), no schema bump. *Recommended:* users look for
   fluting by name. The alternative, ramps on a follow path, hides the feature and puts
   settings on every follow path that most never use.
2. **Open shapes only** (§2); closed ones skipped with a note. *Recommended:* a closed
   shape has no ends to ramp, and a closed groove is a follow path already.
3. **Ramps by length in mm, start and end separately** (§2), shortened in proportion when
   they don't fit. *Recommended:* predictable, and the same ramp on flutes of different
   lengths gives ends that look alike. The alternative, ramps as a percentage of each
   shape's length, suits a sunburst of unequal rays but makes the ends of a short flute
   steep and of a long one gentle. Could be added later as an option if asked for.
4. **Two ramp shapes, straight and curved** (§3), straight by default. *Recommended:* both
   are common and look different. Straight alone would be leaner; curved alone hides the
   familiar straight taper.
5. **Passes back and forth, each cut off at its level** (§4). *Recommended:* no retracts,
   the ramps keep their slope, never more than a step under the tool. The alternative,
   every pass the whole profile scaled to its depth, changes the slope each pass and
   leaves the ramp stepped until the last.
6. **The ramps belong to the shape's drawn ends** (§2); the canvas marks the start.
   *Recommended:* the profile doesn't depend on where the tool happens to be. Reversing a
   shape (a new `shape reverse` and a canvas command) would let a user swap the ends of one
   flute; I'd leave that for when it is asked for, since equal ramps are the usual case.
7. **Tools: ball nose, V-bit and end mill; a drill refused** (§5). *Recommended:* all three
   make flutes people cut; the width at the top is reported for each.
8. **Feeds: the feed on the ramps too**, the plunge feed only straight down (§6).
   *Recommended:* a ramp is a cut along the shape, and it keeps the ends' surface even.
9. **Shortened ramps are a note, not a refusal** (§2). *Recommended:* a short ray in a
   sunburst should still be cut; the note says which.
10. **No schema bump** (§7), as the chamfer. *Recommended.*
11. **The key L in the S menu** (§8), F being taken. *Recommended*; G (groove) is the other
    free choice.
12. **Defaults** (§8): the first ball nose, half-round depth, 20 mm straight ramps.
    *Recommended.*
