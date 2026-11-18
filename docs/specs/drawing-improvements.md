# Spec: mirror about a line, arithmetic in number fields, arc slots

Status: **draft, 2026-11-18**, for review (decisions in §5).

The first three suggestions from the review of the drawing tools on 2026-11-18 (the user:
"go ahead with your suggestions and then we take the rest in step 2"). The rest are in
TODO (12f). Each part below is one commit, with the guide, translations and tests.

## 1. Mirror about a line, with a copy

**Today:** *Draw > Mirror left–right* and *Mirror up–down* (and `shape mirror --axis x|y`)
flip the selected drawn shapes in place, about the middle of the box round them.

**Wanted:** mirror about any line, keeping the original when asked. The common case is a
symmetric part: draw one half against a centre line, then mirror it into a whole outline.
The guide already suggests drawing "a centre line to mirror about", but Mirror can't use
one today.

**In the window: *Mirror* in the MODIFY group** (and the Modify menu, key **I**, free):

- **Steps:**
  1. Select drawn shapes.
  2. Start *Mirror*.
  3. Click two points of the mirror line. They snap, and Shift holds the line to 15°
     steps. **Or** click a straight line or a polyline's straight side: that is the
     mirror line.
- **Preview:** the mirrored shapes follow the cursor's line before the second click.
- **On the status bar:** *Keep the original* (checked by default, remembered for the
  session). Checked: the mirrored shapes are new copies on the same layers, selected
  afterwards. Unchecked: the shapes flip in place and keep their ids, so operations follow
  them, as today's mirror.
- **Joining the halves:** with *Keep the original*, an open polyline (or open spline) whose
  ends both lie on the mirror line, within `CHAIN_TOL_MM`, is joined with its mirror image
  into one closed shape. It keeps the original's id, so a half outline drawn against a
  centre line becomes the whole outline, ready to cut (decision 2).
  - A polyline and its image join into one polyline: the image's points in reverse order,
    their bulges reversed and negated.
  - A spline and its image join into one closed spline through all their points, the
    shared points on the line kept once.
- **Esc** ends the tool. Each mirror is one undo step.
- *Draw > Mirror left–right* and *up–down* stay, as quick flips in place.

**Geometry** (`chipload_job::edit`, a new `mirrored_about(geometry, a, b)` beside
`mirrored`), reflecting about the line through a and b at angle θ:

| Kind | Mirrored |
|---|---|
| Circle | its centre reflected |
| Rectangle | centre reflected, angle 2θ − angle |
| Polygon | centre reflected, first corner's angle 2θ − angle |
| Polyline | points reflected, bulges negated |
| Spline | points reflected |

Turns by multiples of 90° stay exact, as `transformed` does.

- **Operations' points** (start points, corner choices, tabs placed by hand) follow shapes
  flipped in place, as they follow today's mirror. Copies get none, as copies do today.

**Command line:**

```
chipload shape mirror P 3 4 --line "0,0 0,100"            # about that line, in place
chipload shape mirror P 3 4 --line "0,0 0,100" --copy     # copies (joined halves as above)
chipload shape mirror P 3 4 --about-shape 7 --copy        # about straight shape 7
```

`--axis x|y` (about the box's middle) stays, and refuses `--copy`.

## 2. Arithmetic in number fields

**Today:** a length is a number, a fraction (`1/4`, `1 1/2`) or either with a unit (`12mm`,
`1/2in`).

**Wanted:** `18+0.2`, `600/4`, `2*6.35`, `(100-6)/2`, typed straight into a field. This is
also the first step towards named values (the drawing roadmap's phase 4).

**Grammar** (one pure function in `chipload_pipeline::units`, used by `parse_length` and so
by every length field in the window, every typed value while drawing, and every length
option on the command line):

- `+ − * /` (also `×`, `÷` and `x` between numbers: no, decision 4), parentheses, and a
  leading minus.
- **Numbers:** as today, `12.5`, `1/4` and `1 1/2`.
  - A space between a whole number and a fraction is a mixed number, as now.
  - Anywhere else a space is only a gap between tokens.
- **Units:** a unit may follow any number (`1in+3mm`); a number without one is in the
  field's units. A unit carries into a product or quotient once: `2*1in` is two inches,
  while `1in*1in` and `1/2in/3mm` are refused, since they are no length.
- **Order:** `*` and `/` before `+` and `−`, left to right.
- **Refused:** division by zero, a result that isn't finite, and anything else that doesn't
  read. The field shows red with the reason, as an unreadable number does now.
- **The decimal comma:** where the window shows a comma, a field reads `12,5+3` as today's
  fields read `12,5`. In a typed `x;y` point, each part is an expression.
- **Angles:** typed angles (`d<a`, `<a`, the angle fields) take the same arithmetic without
  units: `<360/7`.

**What is kept:** the result. The field shows the number after Enter, and the project
stores the number, as today. Keeping the expression is phase 4's named values (a format
change), not this.

**Where:**

- The pure parser lives in `chipload_pipeline::units` (no I/O, testable). The window's
  fields and the CLI already call `parse_length`, so this needs no new plumbing.
- The window's angle fields and the typed angles go through a sibling
  `parse_number_expr`.

## 3. Arc slots

**Today:** a slot is straight, drawn by the centres of its two ends and its width.

**Wanted:** a slot that bends round a centre, as clamp and adjustment slots do. It is a
**variant on the slot button** (drawing-variants.md):

- **Clicks:**
  1. the arc's centre;
  2. the start of the slot's centre line (its radius and start angle);
  3. its end, by direction only, the way round the cursor went, as the arc by its centre
     does;
  4. its width.
- **Typed values:** step 2 `d` the radius; step 3 `<a` the end's angle; step 4 `d` the
  width.
- **Stored as** a closed polyline of arcs, so no format change (decision 5): the outer arc,
  a half-circle cap, the inner arc back, and the other cap. An arc of more than a half
  circle is split in two, since a bulge is at most a half circle.
- **Refused:**
  - a width of zero, or of twice the radius or more (the inner arc vanishes);
  - caps that would meet across the gap, when the slot goes nearly all the way round;
  - an end in the start's direction.
- **Key:** **U** cycles straight and arc, as R does the rectangles.
- **Command line:** `chipload shape add P arc-slot --center 0,0 --radius 40 --start-deg 30
  --end-deg 150 --width 8`. It makes the same polyline; this is a way of adding one, not a
  new kind.
- **Readout:** R and the angle while drawing the centre line, then W.

## 4. Tests

- **Mirror:**
  - each kind about a slanted line, a horizontal and a vertical one, exact at multiples of
    90°;
  - in place keeps ids and operations' points;
  - a copy joins a half outline (a polyline and a spline) into one closed shape that cuts;
  - shapes not touching the line are copied apart;
  - CLI `--line`, `--about-shape`, `--copy`, and the refusals;
  - the tool by clicks, and by a picked line.
- **Arithmetic:**
  - the parser on numbers, fractions, mixed numbers, operators, precedence, parentheses,
    a leading minus, units on terms;
  - the refusals: dimension errors, division by zero, garbage;
  - the decimal comma;
  - typed drawing entries;
  - a CLI length option.
- **Arc slots:**
  - the construction (caps, inner and outer arcs, both ways round, more than a half
    circle);
  - the refusals;
  - clicks and typed values;
  - the CLI;
  - a pocket of an arc slot passes the checker.
- **Goldens unchanged:** nothing here changes how existing shapes cut.

## 5. Decisions (recommendations first)

1. **Mirror lives in MODIFY** as a tool with two clicks or a picked line, *Keep the
   original* on by default. Today's two quick flips stay in the Draw menu.
2. **Join the halves** when copying: open shapes with both ends on the mirror line join
   their image into one closed shape. Alternative: never join, and the user runs *Join
   ends*.
3. **Key I** for Mirror (free).
4. **Operators `+ - * /` and parentheses only:** no `x` for times (it reads as a size,
   `600x400`), no functions.
5. **An arc slot is a closed polyline**, editable as one afterwards, with no format change.
   Alternative: a new kind that stays an arc slot with its values editable (schema 43).
6. **Results only, no expressions kept**, until named values (phase 4) get their own spec.
