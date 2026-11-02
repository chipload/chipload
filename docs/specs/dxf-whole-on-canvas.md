# Spec: a DXF or SVG selected whole on the canvas, moved by dragging

Status: **draft** 2026-11-02, decisions in §5 to review.

The user (2026-11-02): "imported dxfs and svgs should be able to be dragged similar to text
and images. One click selects the whole dxf or svg and it can be dragged around and into
the shape library."

## 1. Today

- **A click on a DXF's contour** selects that contour alone (`CanvasSelection::Shapes`). The
  whole DXF is selected from its Browser row (`CanvasSelection::Whole { of: Dxf }`).
- **A click on a picture's, a text's or a group's contour** selects the whole picture,
  text or group:
  - Ctrl+click leaves a contour out, or puts it back;
  - a double-click picks one shape inside a group.
- **Pictures, texts and groups have a box** with handles: the middle one moves, the one
  beyond the right side turns in 15° steps, and the corners scale. Dragging one of their
  lines moves them too, or carries them onto the shape library (commit 5822429).
- **A DXF has no box on the canvas.** It is moved, turned and scaled through *Place* and its
  properties (`placement::place_dxf`: `Move::By`, `Rotate`, `Scale`, about its bounding
  box's centre). Its start points and tabs follow it.

## 2. What changes

- **A click on a DXF's or SVG's contour selects the whole DXF**, as for a picture:
  - Ctrl+click leaves a contour out, or puts it back;
  - a **double-click picks one contour alone**, as inside a group (decision 1). That is
    how a single contour is chosen for an operation from the canvas;
  - a box drag (a rectangle from empty space) still selects the contours it takes, one by
    one, as today.
- **Its box** is drawn round it, with the same handles as a picture's (decision 2):
  - **move**: the middle handle, or dragging one of its lines, grabbed where the pointer
    snaps;
  - **turn**: about its centre, in 15° steps, or 1° with Alt;
  - **scale**: by a corner, keeping its proportions, the opposite corner staying put.

  Each drag is one `place_dxf` change and one undo step. Start points and tabs follow, as
  they do for *Place*.
- **Carried onto the Shapes tab**, it is saved to the shape library, not moved. The
  library already saves a DXF's shapes as drawn polylines and circles.
- **With contours left out** (Ctrl+click), there is no box, as for a picture. A drag on its
  lines then selects with a box (decision 3).

## 3. Where the code lives

All in `chipload-gui`, no geometry:
- `click` treats `ShapeFrom::Dxf` as it treats a picture's;
- `selected_box` gets a DXF's box from `placement::dxf_box` and its turn;
- `end_box_drag` turns the box's change into `Move::By`, `Rotate` and `Scale` steps for
  `place_dxf`;
- the double-click case in `canvas.rs` takes a DXF's contour as it takes a group's.

The CLI already does all of this (`place --dxf`); nothing new there.

## 4. Tests

- A click on a DXF contour selects the whole DXF. A double-click selects the contour alone,
  and an operation made from it cuts that contour only.
- Dragging a line of the selected DXF moves its placement by the dragged distance (snapped);
  its start points move with it; one undo step.
- Turning by the handle: the placement's turn changes in 15° steps about the box's centre.
  Scaling by a corner: the opposite corner stays put.
- Carried onto the Shapes tab: saved, not moved.
- An SVG behaves the same.
- The window tests that click single DXF contours are changed to double-click, or to the
  Browser.
- The guide: *Drawings > DXF files* and *The window*; the screenshots taken again if they
  show a selection.

## 5. Decisions to review

1. **A double-click picks one contour of a DXF**, as inside a group. *Recommended:* the
   same gesture everywhere. *Instead:* Alt+click. That is free, but nothing else uses it,
   and Alt already means "no snapping" while dragging.
2. **The DXF's box has the picture's handles: move, turn, scale.** *Recommended:*
   `place_dxf` already does all three, and one box looks and works the same everywhere.
   *Instead:* move only, with turning and scaling left to Properties and *Place*.
3. **No box with contours left out.** *Recommended:* the placement is the whole DXF's, so a
   box round part of it would move what isn't shown as selected. Same rule as pictures.
4. **Box selection stays per contour.** *Recommended:* dragging a rectangle is how several
   contours from several DXFs are picked at once. *Instead:* a rectangle that touches a
   DXF takes all of it.
