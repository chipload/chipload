# Spec: text on an arc

Status: **draft, 2026-11-21**, for review (decisions in §8). The third item of step 2 of
the drawing review (TODO, *Next* 1; docs/specs/drawing-improvements.md).

Builds on text.md (texts and their letters), more-fonts.md (letters told apart) and
sign-maker.md (the sign wizard, its round frame).

## 1. What is missing today

A text is written on straight lines only. A round plaque, a coaster, a clock face or a
badge has its words along the circle: a name along the top, a date along the bottom. Today
that means writing each letter as its own text and turning it by hand.

## 2. What it does

A text can be **on an arc**: its line follows a circle.

- **The circle:** its centre is the text's position (`x`, `y`); its radius is given.
- **Which side** (decision 2):
  - **Top:** the letters stand on the outside of the circle and read clockwise, as a name
    along the top of a plaque.
  - **Bottom:** the letters stand upright at the bottom and read counter-clockwise, as a
    date along the bottom.
- **The radius is the circle through the letters' middle**, half a capital height above
  the baseline (decision 3). A top text and a bottom text with the same radius therefore
  lie in one ring, the same distance from the plaque's edge.
- **Where on the circle:** an angle, the middle of the text (*Center* alignment), its start
  (*Left*) or its end (*Right*). Top texts start at 90° (straight up), bottom ones at 270°
  (straight down).
- **Several lines** stack the way they're read: the first line on the radius, the next
  one line spacing nearer the centre for a top text, further out for a bottom text.
- **Spacing:** letters keep their spacing along the line through their middles. On a top
  text their tops spread a little apart and their feet close in; on a bottom text it's
  the other way round, as on any lettered ring.

## 3. The geometry (from first principles)

The font gives the letters on a straight line (`chipload-text`, as today). Each letter is
then **moved and turned whole, not bent**:

- The letter's position along the line is the middle of its box (in x). Along the circle
  that becomes an arc length, and so an angle: angle = length ÷ radius.
- The letter is turned so its upright stands on the radius at that angle: pointing out
  for a top text, in towards the centre for a bottom one. It is moved so its middle (in x,
  and half a capital height in y) lies on its line's circle.

The letters stay exact lines and arcs, as the font fitting made them; nothing is
approximated again. Bending each letter along the curve would warp their shapes slightly
for little gain at sign sizes (decision 4).

**Refused, with the reason:**
- a radius of 0 or less;
- a line longer than its circle, which would wrap round onto itself;
- a line of a bottom text, or a later line of a top text, whose letters would reach the
  centre: a radius less than half a capital height.

## 4. Format

```json
{"id": 2, "text": "MAKER SPACE", "font": "FreeSans", "height": 12, "x": 0, "y": 0,
 "anchor": "center", "rotation": 0, "align": "center", "spacing": 0, "line_spacing": 1.4,
 "layer": "text", "arc": {"radius": 60, "side": "top", "angle": 90}}
```

- **`arc`:** optional; without it the text is straight, as today. `radius` (mm), `side`
  (`top` or `bottom`), `angle` (degrees, counter-clockwise from +X).
- **On an arc**, `x` and `y` are the circle's centre whatever the `anchor` says, and
  `rotation` turns nothing: `angle` places the text.
- **No schema bump** (decision 5): an optional field with a safe default. An older
  Chipload refuses the file, because texts refuse unknown fields, and says which field it
  doesn't know. This is how every optional field has gone so far (sign-maker.md §4).

## 5. In the window and on the command line

- **The text's form:** an *On an arc* checkbox. When on, three more fields: *Radius*,
  *Side* (Top / Bottom) and *Angle*. The position fields then name the centre.
- **On the canvas:** the text's box shows as the ring it fills. Moving the text moves the
  circle's centre; its turn handle changes the angle.
- **Command line:**

  ```
  chipload text add P "MAKER SPACE" --height 12 --at 0,0 --arc 60 --side top
  chipload text add P "EST. 2026" --height 8 --at 0,0 --arc 60 --side bottom
  chipload text set P 2 --arc 70 --angle 100      # --straight: off the arc again
  ```

## 6. The sign wizard (decision 6)

With the **round frame** (the oval with equal width and height), *New sign…* offers
**Words: straight / round**. Round puts the first line along the top and the second along
the bottom, on one radius inside the innermost border (the gap and the letters' height
taken off). A third line or more stays straight in the middle. The frame still follows
the texts: a frame round words on arcs is the circle round their ring.

## 7. Tests

- **Placement:** one letter at 90° on the top stands straight up, its middle on the
  circle; a bottom text reads left to right with its letters upright; left, centre and
  right alignment; a second line stacked the right way for each side.
- **Exact:** the letters' arcs stay arcs (no flattening), and the same text twice gives
  the same shapes.
- **Refusals:** radius 0, a line longer than the circle, a bottom line reaching the
  centre.
- **Format:** a round trip with `arc`; without it nothing changes, and every text golden
  stays the same.
- **CLI:** `--arc`, `--side`, `--angle`, `--straight`.
- **Window:** the form's fields; the turn handle changing the angle.
- **Sign wizard:** a round sign with words on arcs; the frame round them.
- **Golden:** a round plaque, V-carved: a top and a bottom text on one radius inside a
  round border.

## 8. Decisions (recommendations first)

1. **Text on an arc is a property of a text,** so its words, font and size stay editable
   and the letters are made again when anything changes. *Alternative:* a command that
   bends existing letters once, which leaves nothing editable.
2. **Two sides, top and bottom,** which read the way a plaque reads. A text running the
   other way round (top text read counter-clockwise, letters facing in) is left out; turn
   the letters with *Mirror* if ever needed.
3. **The radius is through the letters' middles,** so a top and a bottom text with the
   same radius share one ring. *Alternative:* the radius at the baseline, which is the
   usual number on a drawing but makes the top and bottom texts need different radii.
4. **Letters move and turn whole, not bend,** so they stay exact lines and arcs and look
   like the font. Bent letters would need refitting and gain little at sign sizes.
5. **No schema bump,** as for other optional fields: an older Chipload refuses the file
   by naming the unknown field.
6. **The sign wizard gets *Words: round*** for the round frame, in the same piece of work,
   since round plaques are what this is mostly for. *Alternative:* text on an arc first,
   the wizard later.

## 9. Order of work

1. The text on an arc: `chipload-pipeline`'s lettering (§3), the format (§4), the command
   line, the form and the canvas (§5), the guide, the seven languages.
2. The sign wizard's round words (§6), with the golden plaque.
