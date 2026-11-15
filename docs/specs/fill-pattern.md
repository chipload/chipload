# Fill pattern: engraving a pattern of grooves inside shapes

Status: **approved 2026-11-15**, every decision as recommended ("Go"); **built
2026-11-15**, both steps of §11. What building settled is in §13. The user asked for it
after fluting ("fill pattern engraving within shapes").

## 1. What it is for

A texture inside a shape: the background of a sign behind raised letters, the panel of a
door, a box lid, a coaster, a decorative band. Today the only way is to draw every line and
cut it as a follow path, clipping each one to the shape by hand. A **Pattern** operation
takes closed shapes, a pattern and its spacing, and engraves grooves that fill the shape
and stop at its edge, leaving its holes alone.

## 2. What is cut

The selected **closed** shapes, as a pocket takes them: a shape with the selected shapes
inside it as holes (`build_region`, the pocket's region), so a sign's background is the
board's outline with the letters selected as holes. Open shapes are skipped with a note, as
a pocket skips them.

Inside each region the pattern is laid out and clipped to it. Each piece left is a
**groove**: the tool on its line, at the operation's depth.

- **Inside the edge** (`inside`, on by default): the grooves are clipped to the region
  shrunk by half the groove's width at the top, so no groove cuts past the shape's line or
  into a hole. Off: clipped to the shape's line itself, each groove running half its width
  past it (for a pattern that runs out under a moulding or an edge cut later).
- Pieces shorter than the groove's width are left out (a speck at a corner).

## 3. Patterns

| Pattern | Settings | Grooves |
|---|---|---|
| **Lines** | angle, spacing | parallel straight lines |
| **Cross-hatch** | angle, spacing, angle between | two sets of lines; 90° between gives squares, 60° diamonds |
| **Rings** | spacing | the shape's outline repeated inwards, one spacing apart (and round its holes), as closed rings |
| **Waves** | angle, spacing, wavelength, height | parallel waves (sine curves) |

- **Spacing** is from one groove's centre to the next.
- **Angle** is the lines' direction, from the X axis, counter-clockwise.
- **Where the pattern lies:** lines and waves are laid out from the drawing's origin, not
  from each shape, so a pattern runs on unbroken across several shapes and over a gap;
  each copy of a part (`copies`) gets its own, as it gets its own shapes. Rings start one
  spacing in from the outline (half a spacing with *inside* off).

Later, if asked for: zigzag, hexagons, dots (a plunge each), a pattern of a drawn shape.

## 4. Depth and ends

- One depth for every groove (`start_depth`, `end_depth`), in equal passes with a step down
  as other operations (none: one pass).
- **Ends** (`ramp`): 0 for a square end, the tool going straight down. More: each groove
  ramps in and out over that length as a flute (docs/specs/fluting.md §2), straight, so
  with a V-bit or a ball nose every groove ends in a point: a softer texture. A groove
  shorter than its two ramps gets them shortened, as a flute's, without a note (there
  are many). Rings have no ends: they go down as a closed follow path does.

## 5. Order and moves

- Grooves are cut in nearest-neighbour order from the nearer end, as flutes
  (fluting.md §4); lines therefore come back and forth, one after the next.
- Each groove is cut as a flute (passes back and forth along it, no retract between them;
  docs/specs/fluting.md §4), then the tool retracts and moves to the next. There is
  material between grooves, so it never stays down between them.
- Rings are cut as a follow path's closed rings (follow-path.md §4), with a plunge or a
  ramp entry.
- Straight lines are G1; waves are short lines within the arc tolerance; rings keep the
  shape's arcs as G2/G3.

## 6. Tools and limits

| Tool | Groove |
|---|---|
| **V-bit** | a V-groove, the usual engraving; no deeper than its cone |
| **Ball nose** | a round-bottomed groove |
| **End mill** | a flat-bottomed slot; square ends need one known to be centre-cutting |
| **Drill** | refused |

- The groove's width at the top is worked out as a flute's and reported.
- **Spacing narrower than the groove:** allowed (the grooves overlap and the area is
  roughened, cleared as a rough pocket would be), with a note.
- **Too many grooves:** refused above 50 000 grooves per operation, with the count and the
  advice to widen the spacing; the time estimate shows the rest.

## 7. Format

A new operation type; no schema bump, as the chamfer and the flute.

```json
{"type": "pattern", "name": "", "enabled": true, "part": 1, "material": null,
 "select": {"shapes": [1, 2, 3]}, "tool": 2,
 "pattern": {"kind": "cross", "angle": 0.7853981633974483, "spacing": 3.0,
             "between": 1.5707963267948966},
 "inside": true, "start_depth": 0.0, "end_depth": 0.5, "step_down": null,
 "ramp": 0.0, "feed": 1500.0, "plunge_feed": 500.0}
```

- `pattern`: one of
  - `{"kind": "lines", "angle", "spacing"}`
  - `{"kind": "cross", "angle", "spacing", "between"}`
  - `{"kind": "rings", "spacing"}`
  - `{"kind": "waves", "angle", "spacing", "wavelength", "height"}`

  Angles in radians, lengths in mm; spacing, wavelength more than 0; height 0 or more (the
  wave's top to its bottom); between more than 0 and at most 90°.
- `inside`: grooves inside the shape's line (true) or centred up to it.
- `ramp`: mm, 0 or more; 0 for square ends.
- `step_down`: null for one pass. No `through`, `side`, `direction` or `entry`.

## 8. Command line and window

- **Command line:**
  `chipload op add part.chipload pattern --shapes 1,2,3 --pattern cross --spacing 3 --angle 45`
  (also `--between 60`, `--wavelength 10 --height 2` for waves, `--ramp 2`, `--on-line`,
  `--depth`, `--tool`, `--layer`); `set …operations.N.pattern.spacing=4`.
- **Window:** *Pattern* in the S menu (key **E**, for engrave: P is the profile), the
  Operation menu (*New Pattern*) and the Browser's new operations; not on the ribbon,
  which has no room (as the flute, fluting.md §13). Its properties: Tool, Pattern (lines,
  cross-hatch, rings, waves) and its settings, Inside the edge, Start depth, Depth with the
  groove's width beneath, Step down, Ends (square, or the ramp length), Feeds.
- **Defaults** for a new pattern: the project's first V-bit, else the usual tool; depth
  0.5 mm (no deeper than a V-bit's cone); lines at 45°; spacing twice the groove's width
  at that depth, at least 1 mm; inside the edge; square ends.
- **Canvas:** the operation's grooves are shown, as every operation's toolpath is, once
  generated.
- **Setup sheet:** "pattern, cross-hatch 3 mm at 45°, inside the edge", with its depths.
- **Guide:** a page *Pattern* under Operations, with a picture of the four patterns.

## 9. Edge cases

- A shape with holes; holes selected and not selected (not selected: no hole, as a pocket).
- A narrow shape the inset closes up: nothing to cut there, a note.
- A pattern line through a corner or along an edge (a line touching the region at a point
  gives no groove).
- Lines and waves across two separate shapes line up.
- Spacing narrower than the groove (note), and above the limit (refused).
- Ramps longer than short grooves.
- Copies of a part.

## 10. Tests

- Clipping: every groove's points lie in the region shrunk by half the width (inside), or
  in the region (on the line); no groove crosses a hole; the grooves of a rectangle with
  lines at 0° are its height over the spacing, each its width less the groove.
- Lines and waves line up across two shapes (the same lines extended); cross-hatch gives
  both directions at the angle between; rings are offsets one spacing apart.
- Moves: back and forth per groove with no retract between passes; a retract between
  grooves; ramped ends as a flute's.
- A proptest: random rectangles and circles with holes, angles and spacings: every cutting
  point inside the shrunk region, outside every shrunk hole.
- Refusals and notes; the width fact.
- Golden: a new `v36/pattern.chipload` (a board outline with a round and a square hole: a
  cross-hatch with a V-bit; a circle with rings; a rectangle with waves and ramped ends)
  and its `lines/` twin; existing goldens unchanged.
- CLI: `op add pattern` and its options; the window: the S menu's E, the form.

## 11. Order of building

1. Format, validation, the patterns, clipping and planning, the command line, goldens.
2. Window: the form, the S menu, the Operation menu; sheet; guide.

## 12. Decisions

1. **A new operation type, *Pattern*** (§7), no schema bump. *Recommended:* users look for
   it by name. Alternatives for the name: *Texture*, *Hatch*.
2. **Four patterns: lines, cross-hatch (with the angle between), rings, waves** (§3).
   *Recommended:* the common ones, built from the same clipping. Zigzag, hexagons and dots
   later if asked for.
3. **Grooves inside the edge by default** (§2): clipped half the groove's width in from the
   line, so the texture never cuts past the shape or into a hole; *on the line* as the other
   choice. *Recommended:* what a sign background needs, with the raised letters' edges kept
   clean.
4. **Laid out from the drawing's origin** (§3), so a pattern lines up across shapes.
   *Recommended.* The alternative, from each shape's own centre, centres the pattern in
   every shape but breaks it between neighbours.
5. **Ends: square, or ramped as a flute's** with one length (§4). *Recommended:* ramped
   ends give the softer, hand-cut look, and the code is the flute's.
6. **Each groove cut as a flute, back and forth, retracting between grooves** (§5).
   *Recommended.* Staying down between grooves would drag through the land between them.
7. **The selected shapes as a pocket takes them, holes included** (§2). *Recommended:* one
   rule for "what's inside" across the program.
8. **Tools: V-bit, ball nose, end mill; a drill refused** (§6). *Recommended.*
9. **Spacing narrower than the groove allowed with a note; more than 50 000 grooves
   refused** (§6). *Recommended:* overlapping grooves are a real texture; the limit stops
   a typo (0.01 mm) from planning for hours.
10. **No schema bump** (§7), as the chamfer and the flute. *Recommended.*
11. **The key E in the S menu, not on the ribbon** (§8). *Recommended*; T (texture) is the
    other free letter. Whether the ribbon should make room for the newer operations (Flute,
    Pattern) is a question for later.
12. **Defaults** (§8): the first V-bit, 0.5 mm deep, lines at 45°, spacing twice the
    groove's width, inside the edge, square ends. *Recommended.*

## 13. As built

- **A line along the edge is no groove:** with *inside* off, a pattern line lying exactly
  on the shape's line (a rectangle's side at a multiple of the spacing) is left out, as a
  line touching a corner is.
- **Clipping** is to the region's loops flattened within the arc tolerance, so a groove
  ends within 0.01 mm of a curved edge; the region (with *inside*) is the pocket's offset,
  shrunk a hair farther when an offset fails at the exact distance (the safe side).
- **Waves** are drawn in pieces within the arc tolerance and clipped piece by piece; only
  the loops' edges near each wave are tried, so a large pattern stays quick.
- **The groove's width** is the flute's (from the cutter's shape); with a tool whose shape
  isn't known, its diameter.
- **Rings** go down with a plunge, as a closed follow path with a plunge entry; an end mill
  not known to be centre-cutting gets the usual note.
- **Grooves** are taken nearest first from wherever the tool is, so a cross-hatch mixes
  its two directions; the moves are short either way.
- **The guide's picture** is the golden sample's simulation (`chipload simulate`), cropped.
- **Code:** `chipload-toolpath/src/pattern.rs` (lines, waves, rings, clipping, order),
  `plan_pattern_op`; the grooves reuse `flute::Flute::of_points` and `flute_moves`;
  `OpKind::Pattern`; `op add pattern --pattern/--spacing/--angle/--between/--wavelength/
  --height/--on-line/--ramp`; the form's `pattern_sections`; `Icon::Hatch`.
