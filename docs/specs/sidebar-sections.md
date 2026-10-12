# Spec: a clearer left sidebar (operations and shapes)

Status: approved (2026-10-12; decisions 1–4 yes, 5 later). User request 2026-10-12.

## 1. The idea

The left sidebar mixed operations and the canvas selection under one "Project" heading,
with the buttons that make operations under "Selection". It becomes three clearly separate
parts, top to bottom:

1. **Job** row: "⚙ Job settings (stock, machine, tools)", alone at the top (not an operation).
2. **Operations (N)**: a shaded header bar, the buttons **+ Profile**, **+ Pocket**,
   **+ V-carve** (for the selected shapes; disabled with "Select shapes first" when
   nothing is selected), the operation cards and the ↑ ↓ Duplicate Delete row. Scrolls on
   its own.
3. **Shapes**: a shaded header bar below a draggable split; what is selected, the editor
   of a selected drawn shape (or the facts of an imported one), **Copy as selector**, and
   **Layers** as an open list: name, number of shapes, a Select button. Scrolls on its own.

Nothing else changes: same actions, same shortcuts, no file or G-code changes. The GUI
stays thin (CLAUDE.md rule 6).

## 2. Decisions (answered 2026-10-12)

1. Two stacked sections with shaded headers and a draggable split, not tabs: **yes**.
2. The new-operation buttons in the Operations header, disabled without a selection:
   **yes**.
3. Job settings as their own row above Operations: **yes**.
4. Layers as an open list with counts: **yes**.
5. A list of every shape to click and select: **later** (TODO).

## 3. Tests

- The layer list counts shapes per layer, sorted by name.
- Rendering the sidebar with and without a project and a selection doesn't panic (an app
  pass, as the other GUI tests).
