# Compass: move, turn and array shapes from one handle on the canvas

Status: **approved 2026-11-15**, every decision as recommended ("I will need to try it to
know that it feels right and then we can iterate"); **built 2026-11-15**, all five steps of
§12, for the user to try. What building settled is in §14.

## 1. What it is for

Placing shapes against each other today takes several tools: drag the shape with its handles,
type *Move* values, *Turn* about a point, then *Array* or *Polar array* in their own forms.
The **Compass** puts all of it on one handle at the selection:

- move along an axis, or freely from one snap point to another;
- turn about its centre;
- copy, or make a linear or circular array, by dragging with the right button;
- put the Compass itself anywhere first (Space), so the move or turn is measured from a
  corner, a hole's centre or a line's end.

It is the quickest way to put one shape exactly against another: unlock, drop the centre on
the shape's corner, lock, drag the centre onto the other shape's corner.

## 2. What it works on

The Compass shows when the selection can be moved, in top view, while the project can be
drawn on:

- **drawn shapes**, one or several, and a group selected whole: everything (§4–§6);
- **a whole DXF or SVG, picture, text or 3D model**: move and turn only, through their
  placement as their handles do today; copies and arrays are greyed, with why (only drawn
  shapes copy today);
- **shapes of a DXF or SVG one by one**, and construction geometry with other shapes:
  as for the selection today (imported shapes can't be moved one by one; it isn't shown).

## 3. The Compass

Drawn at the middle of the selection's box when it is shown, a fixed size on screen
whatever the zoom:

- **two arrows**, its X and Y axes, about 60 px long, in the axis colours of the rulers;
- **a ring** between them, about 40 px across;
- **a centre**: a small square.

Its axes start along the drawing's X and Y. Hovering a part highlights it and the status
bar says what a drag does there.

**Showing it:** **Q** with something selected (§9), *Compass* in the canvas's right-click
menu, and a small button in the ribbon's MODIFY group. Q again, Escape, or an empty
selection hides it. While it shows, the selection's own handles (a rectangle's corners, a
picture's box) are hidden, so nothing overlaps; they come back when it hides.

## 4. Locked: dragging moves and turns the selection

| Drag | Does |
|---|---|
| an arrow | moves the selection along that axis. The distance snaps to the grid's step, and to snap points (§7) projected onto the axis |
| the ring | turns the selection about the centre, in steps of 15°; with Alt, 0.1° |
| the centre | moves the selection freely; the centre snaps to snap points (§7) and the grid, so a point of the selection lands exactly on a point of another shape |
| Alt with any drag | no snapping, as everywhere on the canvas |

- While dragging, a label at the cursor gives the value: "X +25 mm", "30°", "ΔX 12.5,
  ΔY −4 mm"; the shapes are shown where they will go.
- **Exact values:** a double-click on an arrow, the ring or the centre asks for the
  distance, the angle, or ΔX and ΔY, in a small field at the handle; Enter applies it.
- The Compass moves and turns with the selection, so the next drag starts from where the
  last ended.
- Each drag is one undo step. Shapes keep their ids, so the operations that cut them follow
  (as `shape move` and `shape turn`).

## 5. Unlocked (Space): placing the Compass itself

A tap of **Space** (pressed and let go without moving the mouse) while the Compass shows
unlocks it: it turns grey and the status bar says "Compass unlocked: drag it into place,
Space locks". Space held while dragging still moves the view, as everywhere.

| Drag, unlocked | Does |
|---|---|
| the centre | moves the Compass, not the shapes; it snaps to snap points (§7), so it can sit on a corner, a hole's centre, a line's middle |
| the ring or an arrow | turns the Compass's axes, not the shapes: in steps of 15°, and onto the direction of a line under the cursor, so an axis can lie along an edge |

Another tap of Space locks it again; the next drag moves the shapes, measured from where
the Compass now is. Right-clicking the centre gives *Back to the middle* and, for the
axes, *Along X and Y*.

## 6. Right-drag: copy and arrays

A drag with the **right** button does what a left drag does, then asks on release, in a
small menu at the cursor:

| Right-dragged | Menu |
|---|---|
| an arrow, or the centre | *Move here* · *Copy here* · **Linear array…** · *Cancel* |
| the ring | *Turn here* · *Copy here* · **Circular array…** · *Cancel* |

- **Copy here:** a copy where the drag ended; the originals stay. The copy becomes the
  selection, so dragging again places the next one.
- **Linear array…** (an arrow or the centre): the form under *Selection* opens with
  *Count* (2 at first: the original and one copy) and *Spacing*: the distance and
  direction dragged, ΔX and ΔY along the Compass's axis however it is turned. The copies are
  previewed on the canvas as the values change; *Create* adds them, *Cancel* nothing.
- **Circular array…** (the ring): the form opens with *Count* (2 at first), *Angle between*
  (the angle dragged), a *Full circle* tick that spreads the count evenly round instead, and
  *Turn the copies* (on). The centre is the Compass's centre. Previewed; *Create* adds them.
- With anything but drawn shapes selected, *Copy here* and the arrays are greyed, with why.
- A right-click without a drag opens the canvas's menu as today.

## 7. Snap points

The same as the drawing tools' (ends, intersections, centres, quadrants, middles; the
selection's own points left out while it moves, so it never snaps to itself), shown with the
same marker. Snapping to the stock's corners and edges as the drawing tools do.

## 8. Command line

Every Compass action is a shape change the command line already makes, but one:

| Compass | Command line |
|---|---|
| move | `shape move P IDS --dx --dy` |
| turn | `shape turn P IDS --angle-deg A --about X,Y` |
| copy here (move) | `shape copy P IDS --dx --dy` |
| copy here (turn) | `shape array-polar P IDS --count 2 --angle-deg A --center X,Y` |
| circular array | `shape array-polar P IDS --count N --angle-deg … --center X,Y [--no-rotate]` |
| linear array | **new:** `shape array-linear P IDS --count N --step DX,DY` |
| a whole DXF, picture, text or model | `place`, `image set`, `text set`, `model set` as today |

`shape array-linear`: N copies counting the originals, each `--step` further than the one
before; refused for a step of 0 with more than one, and above 10 000 shapes, as `shape
array`. In the pipeline beside `array_grid`.

## 9. Keys

**Q** shows and hides the Compass. The quick menu (docs/specs/fewer-clicks.md §2) moves to
**S**, the more usual key for such a menu (the user, 2026-11-15). **S** draws a slot today,
so the slot gets **U** (a slot is a long U; U is free). The quick menu's guide pages, its
status bar hint and the shortcuts window change with it.

## 10. Where the code is

- `chipload-pipeline`: `array_linear` beside `array_grid`; the rest exists.
- `chipload-gui`: `app/compass.rs`: its state (shown, locked, centre, axes' angle), drawing
  it, hit-testing its parts, the drags, the right-drag menu; the array forms take their
  starting values from it. The moves and copies it makes go through the pipeline's shape
  functions as the existing tools' do; no geometry in the GUI beyond the handle's own
  screen positions.
- `chipload-cli`: `shape array-linear`.

## 11. Tests

- `array_linear`: count, step along a turned direction, ids, refused values.
- GUI (egui_kittest): Q shows it on a selection, Escape hides it; an arrow drag moves along
  the axis only and snaps to a corner's projection; a ring drag turns in 15° steps about the
  centre; Space taps lock and unlock, a Space-drag still pans; unlocked, a centre drag
  moves the Compass and not the shapes, and the next drag is measured from there; a
  right-drag on an arrow gives the menu, *Copy here* copies, *Linear array…* opens the form
  with the dragged spacing; on the ring, *Circular array…* with the dragged angle and the
  Compass's centre; each one undo step; a picture moves and turns, its copies greyed.
- Keys: S opens the quick menu, U draws a slot, Q the Compass.

## 12. Order of building

1. `shape array-linear` in the pipeline and the command line.
2. The Compass shown, locked drags (§4) with snapping and exact values.
3. Unlocked (§5).
4. Right-drag, copies and the array forms (§6).
5. Keys (§9), the guide (a *Compass* section on the shapes page; the quick menu's pages),
   the shortcuts window.

## 13. Decisions

1. **The name: Compass.** Chosen by the user.
2. **What it works on** (§2): everything for drawn shapes and whole groups; move and turn
   only for a whole DXF, picture, text or model. *Recommended.*
3. **The handle** (§3): two arrows, a ring, a centre, a fixed size on screen, its axes along
   X and Y at first; the selection's own handles hidden while it shows. *Recommended.*
4. **Locked drags** (§4): arrows along their axis, the ring turns about the centre, the
   centre moves freely with snapping; a double-click for exact values. *Recommended.*
5. **Space taps unlock and lock** (§5); a Space held while dragging still pans.
   *Recommended:* the only way to keep both, since Space-drag panning is how many users move
   the view.
6. **Right-drag menus** (§6): on an arrow or the centre *Linear array…*, on the ring
   *Circular array…* (as the user described), each with *Move/Turn here*, *Copy here* and
   *Cancel*; the arrays open their forms filled from the drag, previewed before *Create*.
   *Recommended.*
7. **A new `shape array-linear`** (§8) for arrays along a turned axis; the grid array stays
   for rows and columns. *Recommended.*
8. **Keys** (§9): Q the Compass, S the quick menu, U the slot. *Recommended.* The other
   way, the Compass on another key and Q kept, leaves the quick menu where it is but
   gives the Compass a less natural key.
9. **No persisted state:** where the Compass sits and how its axes turn is forgotten when it
   hides; nothing goes into the project. *Recommended.*

## 14. As built

- **Groups:** a group selected whole moves and turns with its texts, but *Copy here* and
  the arrays are greyed for it, as for a picture: the array functions copy drawn shapes,
  and a group's texts wouldn't come along. Select its shapes alone (double-click) to copy
  them.
- **Circular array…** opens the existing *Polar array* form, filled with a count of 2,
  the Compass's centre, and the dragged angle as its sweep, rather than a form of its own
  with *Angle between* and *Full circle*. With 2 copies the sweep is the angle between
  them; raising the count spreads them over that sweep. One form for both ways in; the
  user may prefer the other when trying it.
- **Linear array…** opens a new *Linear array* tool (count and step ΔX, ΔY), the Compass's
  only new tool; it isn't on the ribbon, since a grid array covers rows and columns.
- **The ribbon:** the Compass starts the MODIFY group's second row, so the group keeps its
  width (twelve small buttons, six a row).
- **Snapping along an arrow:** a snap point projected onto the axis wins over the grid;
  with neither under the cursor, the distance goes in grid steps. The centre, locked,
  leaves the selection's own points out; unlocked it takes them, to sit on one of its
  corners.
- **Unlocked arrows** lie along the straight line nearest the cursor within the pick
  distance, the way round nearer the arrow's; elsewhere they point at the cursor in 15°
  steps.
- **Code:** `app/compass.rs` (state, hit-testing, drags, the menu), the painting in
  `view.rs` (`paint_compass`, sizes shared with the hit-testing), `array_linear` in the
  pipeline, `shape array-linear` on the command line.

## 15. To point, to centre point (2026-11-15)

The user, after trying it: "We should be able to right click the center or the arrows and
choose 'to point', 'to centerpoint' and click a point to move to. The center moves in x and
y but the arrow moves only along that axis."

- A **right-click** (pressed and let go without dragging) on the centre or an arrow opens
  the Compass's own menu instead of the canvas's: **To point**, **To centre point**,
  *Cancel*. The ring has none (a turn to a point is a drag of the ring). A right **drag**
  still asks as in §6.
- Then the **next click** picks the point:
  - *To point:* the snapped point, as a drag snaps (the selection's own points left out,
    locked); Alt: none.
  - *To centre point:* the centre of the shape clicked: its arcs' common centre (a circle,
    an arc), else the middle of its box. Off any shape the status bar says so and the pick
    goes on.
- **The move:** from the centre, the Compass's centre comes to the point, in X and Y; from
  an arrow, along that axis only, as far as the point lies along it (its projection). One
  undo step; the Compass goes with the selection, as after a drag.
- While pointing, the moved outline previews and the status bar says what a click does;
  **Esc** cancels (before hiding the Compass), and so does a click of *Cancel*.
- **Unlocked** (§5), the same places the Compass itself: the centre onto the point, an
  arrow aimed at it.

As built: the menu is an `egui::Area` as the right drag's; the canvas's context menu isn't
opened on that click; a drag that starts while picking does nothing; `contour_centre` is in
`chipload_pipeline::selection` (no geometry in the GUI). Tests: the right-click through the
window (menu, pick, snap, Escape), an arrow's projection, a shape's centre and a click off
any shape.
