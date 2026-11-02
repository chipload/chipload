# Spec: groups of drawn shapes

Status: **draft 2026-11-02**, decisions 1-8 to review.

The user (2026-11-02), about a traced picture saved to the shape library and inserted
again: "If I save a traced image will the shapes be grouped when inserting or are they free
objects?" They come in as free drawn shapes. "Yes I think we need to be able to group
them."

## 1. Why

An inserted trace is dozens of drawn shapes. Selected once after inserting, they move
together; after a click elsewhere, getting them all back means selecting their layer, which
only works when nothing else is on it. The same goes for a sign put together by hand (a
frame, a border, a few holes) and for any library entry of more than one shape.

A **group** keeps shapes together: one click selects all of it, and it moves, turns, scales,
copies and deletes as one. What it cuts doesn't change: operations select shapes by layer
or by id, as now.

## 2. What a user can do

- **Group:** select shapes and texts, then *Group* (right-click on the canvas, or Ctrl+G).
  The group gets a name, *Group 1*, *Group 2*…, which can be changed.
- **Select a group:** a click on any of its shapes selects the whole group, as a click on
  a letter selects its text. In the Browser, a group is a row under *Drawing*, with its
  shapes under it.
- **One shape inside:** a **double-click** on a shape of a group selects that shape alone,
  to change its values or drag its points (decision 3). Esc, or a click outside, goes
  back to selecting whole groups.
- **Move, turn, scale:** a selected group has box handles, as a picture and a text have
  (docs/specs/box-handles.md). Dragging any of its drawn shapes moves the whole group.
  Texts in the group turn and scale with it (their height scales).
- **Ungroup:** right-click > *Ungroup* (or Ctrl+Shift+G). The shapes stay where they are,
  free again.
- **Delete:** Del on a selected group deletes its shapes and texts, as one undo step.
- **Copy and paste:** copying a whole group pastes a new group (decision 6).
- **Library:** saving a group saves its shapes; inserting an entry of more than one shape
  makes a group named after the entry (decision 5).
- **The same on the command line** (§6).

## 3. What can be in a group

- **Drawn shapes** (circles, rectangles, polylines, polygons, and the shapes of explode and
  inserts) and **texts** (whole, with their letters).
- **Not** a DXF's shapes, nor a traced picture's: they already belong to their file or
  picture, which selects, moves and updates them as one (decision 2). Exploding a DXF, or a
  picture saved to the library and inserted, gives drawn shapes, which can be grouped.
- A shape or text is in **at most one** group. **No groups inside groups** (decision 4).
- Construction geometry can be in a group; it stays construction.

## 4. In the project file

- A new optional list in the drawing, **`drawing.groups`**, each:
  `{"id": 1, "name": "Group 1", "shapes": [4, 5, 6], "texts": [2]}`. Left out of the file
  when there are none, as `texts`, so a project without groups opens in older versions:
  **no schema bump** (decision 7).
- Validation: every id is a drawn shape or a text of the project; none in two groups; a
  group has at least one member; ids unique.
- **Deleting a member** takes it out of its group; a group left empty goes too. Explode and
  other edits that replace shapes keep the group when the new shapes take the old one's
  place (a trimmed polyline's pieces stay in its group).
- Operations keep selecting by layer or by shape id: a group is never something an
  operation cuts by (decision 8).

## 5. The window

- **Canvas:** clicking a grouped shape selects the group: all its shapes highlighted, its
  box with handles. Ctrl+click adds or removes the whole group. A selection box catches a
  group when it catches all of it (the same rule as for texts).
- **Double-click** a grouped shape: that shape alone is selected, with its own handles and
  values in Properties; a thin outline shows the rest of its group.
- **Properties** for a group: its name, how many shapes and texts, the box's size and
  place (typed values, as for a selection of drawn shapes), and **Ungroup**.
- **Right-click on the canvas:** *Group* (two or more things selected, none of them
  imported), *Ungroup* (a group selected).
- **Browser:** *Drawing* gets a row per group, `▣ Group 1 (12)`, opening to its shapes;
  grouped shapes are not listed again under their kinds.
- **Shortcuts:** Ctrl+G, Ctrl+Shift+G, listed under Help > Shortcuts.

## 6. The command line

```
chipload group add part.chipload 4 5 6 --texts 2 --name "Plate"   # prints the group's id
chipload group list part.chipload
chipload group set part.chipload 1 --name "Door plate"
chipload group remove part.chipload 1        # ungroup: the shapes stay
chipload group delete part.chipload 1        # the shapes and texts too
chipload shape move part.chipload --group 1 --dx 10 --dy 0   # also turn, scale: the group's shapes and texts
```

`--json` on each, as every command.

## 7. Where the code goes

| Layer | What |
|---|---|
| `chipload-job` | `Drawing.groups` (`ShapeGroup`), validation, members dropped with their shapes. |
| `chipload-pipeline` | `groups::{group, ungroup, delete_group, rename}`; moving, turning and scaling a group (drawn shapes through `transform_shapes`, texts by their position, turn and height); copy and paste of a group; library insert makes a group. |
| `chipload-cli` | `group …`, `--group` on `shape move/turn/scale`. |
| `chipload-gui` | the selection of a group, double-click into it, box handles, Properties, Browser rows, menu and shortcuts. No geometry. |

## 8. Tests

- Group, ungroup, delete: the file round-trips; a deleted member leaves its group, an empty
  group goes; a member in two groups, or an imported shape, is refused.
- Moving, turning and scaling a group: drawn shapes and texts end where the box says;
  scaled texts keep their words and font.
- Library: inserting an entry of several shapes makes one group named after it; one shape
  makes none.
- An old project (no groups) loads and saves unchanged; a project with groups gives the same
  G-code as without (groups don't change cutting).
- Window: a click selects the group, a double-click one shape, Ctrl+G and Ctrl+Shift+G,
  Del, undo of each.

## 9. Decisions

1. **Groups of drawn shapes and texts, kept in the project, selected with one click.**
   *Recommended.* Alternative: only select-together without storing (a "select similar"
   or saved selections), which is lost on reload.
2. **No DXF or traced-picture shapes in a group.** *Recommended:* they already move and
   select as one through their file or picture, and their updates would fight a group.
   Alternative: allow them, and take them out of the group when the file is updated.
3. **Double-click a grouped shape to work on it alone.** *Recommended:* as in most drawing
   programs; a click keeps the group whole. Alternatives: Alt+click, or ungroup first.
4. **No groups inside groups.** *Recommended:* one level covers a sign, a trace and a
   library insert, and keeps clicks predictable. Alternative: nesting, like Inkscape's.
5. **Inserting a library entry of more than one shape makes a group named after it.**
   *Recommended:* that is what started this; Ungroup frees them. Alternative: free shapes,
   grouped by hand.
6. **Copying a whole group pastes a new group.** *Recommended.* Alternative: pasted shapes
   free.
7. **`drawing.groups`, optional, no schema bump.** *Recommended:* groups don't change what
   is cut, and a file without groups stays as it is. (An older Chipload refuses a file
   that has groups, as for any unknown field, rather than misreading it.) Alternative: a
   `group` field on each shape, which needs a bump and can't hold texts.
8. **Operations don't select by group.** *Recommended for now:* layers and ids already
   cover it, and a group's shapes are often cut differently (a frame profiled, its border
   grooved). Alternative: "Use selection" with a group selected stores the group, so
   shapes added to it later are cut too.
