# Spec: direct position, rotation and scale for DXFs and pictures

Status: approved 2026-10-14 (the user: "go ahead").

## 1. Why

User request (2026-10-14): "When selecting a DXF, the options for position, scale and
rotation are complicated. They should be more direct: pos (0, 0) (reset), rotation (90)
(reset), scale 1.00 (reset). We should also allow any degree rotation for images and DXFs."

Today a DXF's properties show "scaled ×2, turned 90°, moved X… Y…", a *move by* X Y, a
*bottom left* margin, size W and H, and 90° turn buttons. A turn is in quarter turns only.

## 2. The rows

For a DXF:
```
Position   X [ 10.00 ]  Y [ 10.00 ] mm   (reset)
Rotation   [ 0.0 ]°  ⟲90  ⟳90            (reset)
Scale      [ 1.000 ]                      (reset)
Size       120.0 × 80.0 mm
```
- **Position** is the lower-left corner of the drawing's bounding box, in the drawing's
  coordinates (decision 1). Typing moves the drawing so its corner is there.
- **Rotation** is the angle from the DXF as drawn, in degrees counter-clockwise, any value;
  it is shown in [0, 360). ⟲90 and ⟳90 add or take 90°. A change turns the drawing about
  its box's centre (decision 2).
- **Scale** is the factor from the DXF as drawn, 0.001 to 1000. A change scales about the
  box's centre.
- **Size** is information: the box's width and height.
- **Reset** per row: the position back to where the DXF has it (rotation and scale stay),
  rotation 0, scale 1.
- A change applies when the edit is finished (Enter, Tab, or a drag released): one undo
  step each.

A picture has the same rows, with **Width** instead of Scale (its height follows from the
picture) and the reset of the position going to X0 Y0.

## 3. The file (schema 24)

- `placement.quarter_turns` (0–3) becomes **`placement.rotation`**: degrees, `0 <= r < 360`.
  A DXF point p goes to `scale · rotate(p, rotation) + (dx, dy)`, as before with the angle
  in place of the quarter turns.
- Each image gets **`rotation`**: degrees, `0 <= r < 360`, about the picture's centre. `x`,
  `y` and `width` keep their meaning: the picture's lower-left corner and width before it
  is turned. Its traced shapes are turned after the trace, so the trace doesn't change.
- **Migration 23 → 24:** `rotation = 90 × quarter_turns`; images get `rotation: 0`.
- **Exactness:** a rotation that is a multiple of 90° is computed exactly, as quarter turns
  were, so migrated projects give the same G-code (golden tests).

## 4. Command line

- `place --rotate DEG` takes any angle (it took ±90° steps), turning about the box's centre.
- `place --at X,Y` puts the box's lower-left corner there.
- `image set --rotate DEG` sets a picture's rotation (absolute, like `--width` and `--at`).

## 5. Decisions

1. **Position = the box's lower-left corner.** *Approved.*
2. **Rotation and scale in place,** about the box's centre. *Approved.*
3. **Any angle,** with ⟲90/⟳90 buttons. *Approved.*
4. **Scale as one factor;** size shown. *Approved.*
5. **Pictures get position, rotation and width,** turned after the trace. *Approved.*
6. **Reset per row.** *Approved.*
7. **CLI:** `place --rotate` any angle, `place --at`, `image set --rotate`. *Approved.*

## 6. Tests

- The v23 sample with a quarter-turned DXF migrates to rotation 90 and gives its golden
  G-code.
- Rotating by 30° then −30° gives the shapes back within the placement rounding; by 90°
  four times gives them back exactly.
- A picture at 45° has its traced shapes turned about its centre; its trace (shape count)
  is the same as at 0°.
- Position: setting X Y puts the box's corner there.
