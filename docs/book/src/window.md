# The window

## The Browser

The **Browser** on the left holds the whole project as a tree:

- **The project**, named after its file.
  - **Job settings:** stock, machine, post-processor and tools, shown on the right when
    clicked. A red dot means one of them has a problem.
  - **Tools:** the project's tools. They are edited in the job settings.
  - **Drawing:** what the shapes come from. Each DXF has its layers under it; then come
    each picture, and the shapes drawn in Chipload by kind (circles, rectangles,
    polylines). See [Several drawings](drawings/several.md).
  - **Operations**, in cutting order. A line marks each tool change.

![The Browser: two DXFs with their layers, the second hidden, a picture, and four
operations; the first DXF's placement window is open](images/drawing.png)

**Rows:**
- **Click** a row to select it.
  - A DXF, picture or layer selects its shapes for a new operation, and Ctrl+click adds
    to the selection.
  - An operation's row, or *Job settings*, shows its settings on the right.
- **Double-click** a DXF to place it, or a picture to edit it.
- **Right-click** a row for the rest:
  - **A DXF:** *Place…*, *Update from a DXF…* and *Remove*.
  - **A picture:** *Edit…* and *Remove*.
  - **An operation:** *Move up* and *Move down* (also Alt+↑ and Alt+↓), *Duplicate*, *Add
    rest pocket* and *Delete*.
  - **The Drawing folder:** *Add DXF…* and *Import image…*.
  - **The Operations folder:** a new operation for the selected shapes.
- **The triangle** opens and closes a folder, and so does a click on a plain folder like
  *Tools*.

**The eye** hides a DXF, layer, picture or kind of drawn shape on the canvas: it isn't
drawn and can't be clicked. **Hidden shapes are still cut.**

**The tick** in front of an operation turns it on or off. An operation that is off isn't
in the program, and its row is dimmed.

**On an operation's row:**
- **Right end:** its tool, and how many hidden contours it cuts ("3 hidden · T1").
- **A red dot** marks a problem.
- **Hover** the row for its kind, its geometry, its depths, its time once generated and
  its first problem.
- **Drag** the row onto another to move it there.

## Properties

The **Properties** panel on the right shows what is open: the job settings, or an
operation's settings under its kind and number ("Pocket · Operation 1").

While shapes are selected, a **Selection** section comes first. It holds:
- what is selected, by contour number;
- a drawn shape's exact values, to edit;
- *Copy as selector*, which copies the selection as JSON for a project file.

Its triangle folds it away, so the operation's settings come up.

![Properties: a drawn polyline selected, with its points, above operation 1's settings](images/properties.png)
