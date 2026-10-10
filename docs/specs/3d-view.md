# Spec: 3D view of the toolpath

Status: **approved 2026-10-10** (Q1–Q5 as recommended).

User request (2026-10-10): view the canvas in 3D to see the depth of the generated paths,
with a button back to the top view. The drawing's geometry always sits on top of the stock.

Display only: nothing here changes a toolpath, the G-code or the project file.

## 1. Approach

- The canvas keeps drawing with egui's 2D painter. The view's transform becomes an **orbit
  camera**: a rotation (turn around the vertical axis, tilt towards the viewer), then the
  same pan and zoom as now. Every 3D point is projected to the screen and drawn as lines,
  exactly like today. No 3D engine, no new dependency, no GPU code.
- **Top view is the camera with no rotation**: the projection then equals today's mapping, so
  the 2D view and everything that works in it (picking, drawing, material-left shading)
  stays as it is.
- **Orthographic projection** (no perspective): straight lines stay parallel, distances
  along each axis keep one scale, zoom works as now. *(Question 4.)*

## 2. What is drawn in 3D

All in drawing coordinates (mm), with Z as the program uses it (stock top at Z 0 for
`z_zero: top`, at Z = thickness for `bottom`).

- **Stock**: a wireframe box, its top and bottom outlines and the four vertical edges.
- **Geometry** (contours, the selection, the operation highlight): at the stock top.
- **Toolpath** with its real Z: cutting moves solid, rapids dashed, plunges and retracts as
  vertical lines. Cutting moves are **shaded by depth**: lighter near the stock top, darker
  towards the deepest cut, so passes stand apart.
- **Tabs** at their own height (where the tool lifts over them).
- **Material left** shading at the stock top; plunge markers and operation numbers at the
  plunge point's top.
- The work-zero axes (X, Y, and now Z) at the work zero.

## 3. Controls

| | Top view (as now) | 3D |
|---|---|---|
| Left-drag | pan | **orbit** (turn and tilt) |
| Right-drag, or Shift+left-drag | pan | pan |
| Wheel | zoom at the cursor | zoom at the cursor |
| Click | select a contour | select a contour (picked on the stock top) |

- Toolbar and View menu: **3D** (switches on, keeping the zoom and starting from a
  three-quarter view) and **Top view** (back to straight down, as before). Keys: `T` top
  view; *Fit* (`F`) fits in either view.
- Tilt is limited between straight down and horizontal, so the view can't flip upside down.
- Drawing tools work in top view only: choosing one switches back to top view.

## 4. Depth exaggeration

A 6 mm pocket in a 200 mm part is hard to see at true scale. A **Z scale** choice in the View
menu (×1, ×2, ×5, ×10) stretches Z in the 3D view only; anything other than ×1 is shown next
to the 3D button so it's never mistaken for real proportions. *(Question 3.)*

## 5. Tests

GUI-free, in the view module:
- Top view (no rotation) projects every point exactly as the current 2D transform.
- A point straight above another projects onto it in top view and above it when tilted.
- Orbit limits: tilt is clamped; turning wraps.
- Fit in 3D keeps the whole stock box on screen.
- The toolpath shape keeps Z: a plunge is a vertical segment; a depth pass's lines carry its Z.
- Picking in 3D finds the contour under the cursor on the stock top.

And a screenshot check of a tilted view.

## 6. Decisions (approved 2026-10-10)

1. **Approach** (§1): an orbit camera drawing lines with the existing painter, no 3D engine.
   **Yes**: it covers seeing depths, and stays small and dependency-free. Solid,
   lit surfaces (a simulated machined part) would need a real 3D renderer: a separate,
   much larger step.
2. **Controls** (§3): left-drag orbits in 3D, right-drag or Shift-drag pans; buttons
   *3D* / *Top view*. **Yes.**
3. **Z scale** (§4): ×1 by default, ×2/×5/×10 to choose, labelled when not ×1.
   **Yes.**
4. **Orthographic** projection (§1). **Yes**; perspective could be an option
   later.
5. **Drawing only in top view** (§3). **Yes.**
