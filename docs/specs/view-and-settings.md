# Spec: one place for what's shown, and no Settings menu

Status: approved and built 2026-10-14 (the user: "I agree", with clearer canvas buttons).

## 1. Why

User request (2026-10-14):
- "Show stock, show grid, show toolpath, show images should be consolidated to the same
  place; right now two are in the header and two are at the bottom."
- "The settings menu is hard to discover and contains very little; we can remove it. The
  checkbox for the setup sheet should be in the job settings, the theme could be in the
  header, and the tool library and machines should be in the header bar."
- "The canvas toolbar icons are a bit small and hard to understand unless hovered."

## 2. The canvas's navigation bar

- **Every button gets a short word under its icon,** as in the ribbon, and is a little
  larger: **Fit**, **Top**, **3D** | **Stock** (the simulation), **Toolpath**, **Images**,
  **Grid**. A gap separates how you look from what is shown.
- **The bar stays centred** at the canvas's bottom, sized to its buttons.
- **Drawing** (added 2026-10-16, user: "a show/hide for geometry in the case I want to only
  look at the simulated stock"), between Toolpath and Images: off, the drawing's contours,
  their copies and the selection and operation highlights aren't drawn, and clicks on the
  canvas pick nothing. A new selection (from the Browser) or starting to draw a shape turns
  it on again, so nothing is edited unseen; clearing the selection (Esc) doesn't. Not
  remembered between sessions, like Toolpath and Images. Also *View > Show drawing*.
- **Images** (fixed 2026-10-16, user: "the show/hide image button does not do anything"):
  the pictures were painted under the simulated stock, which is on by default, and only in
  top view. They are now painted over the stock and under the drawing, on the stock top,
  in top view and 3D.
- **Too narrow for the names** (2026-10-16): when the bar with names doesn't fit between
  the legend (bottom left) and the canvas's right side, its buttons show icons only, with
  the name on hover; the bar moves right of the legend if it can't be centred clear of it.
- **The ribbon's VIEW group keeps Measure.** Its menu still has everything (show
  toolpath, images, grid, simulation, Z scale, legend, update automatically), plus the
  colour theme.

## 3. No Settings menu

- **A LIBRARY group in the ribbon:** **Tools** and **Machines** open the tool library and
  machine profiles windows. Its caption's menu has the same two items.
- **The colour theme** is a button in the top bar (sun, moon, or half for System). A click
  goes on to the next, System → Light → Dark, and the hover text says which is on.
- **The setup sheet** is a row in Job settings > Machine: "Setup sheet — write one with
  the G-code (all projects)". It stays an app setting, as today. The label says it applies
  to every project, because Job settings is otherwise saved with the project
  (decision 2a).
- **The top bar's right end** has the theme, Help and the version.

## 4. Tests

- The navigation bar and the ribbon draw.
- Drawing off: nothing is picked, no handles; a Browser selection or a new sketch shows it
  again, Esc doesn't. Images: the picture is painted in 3D, and not when switched off.
- The setup sheet row sets the app setting.
- The theme button goes System → Light → Dark → System.
- No change to project files or G-code.
