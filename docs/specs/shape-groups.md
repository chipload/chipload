# Spec: groups of drawn shapes

Status: **agreed and built 2026-11-02**, decisions 1-10 as recommended (the user: "Good go
ahead").

As built:
- **Moving, turning and scaling** on the command line is `group move/turn/scale ID`, not
  `--group` on `shape move` (§6): the group's id is the one thing they need.
- **Ungroup refuses** a group with texts while an operation cuts it: an operation can't
  name a text's letters by id, and cutting them silently differently is not allowed. The
  message says to choose the operation's shapes again, or take the texts out first.
- **Texts in a group** move, turn and scale with their letters moved as they are (no font
  needed); their height and spacing scale. Their rows stay where texts are in the Browser;
  the group's row lists its drawn shapes.
- **A group's row** has no eye: its shapes show and hide by their layers and kinds.
- **Members that are gone** (a shape deleted) are taken out by every edit
  (`tidy_groups`); a gone member in a file is not a problem. A trimmed or combined shape's
  new pieces join its group.
- **Pasting a copied group** groups the pasted shapes; a group's texts aren't copied (texts
  aren't copied at all yet).
- **The canvas's right-click menu** got the selection's items: Group, Ungroup, Add to
  group ▸, Remove from group, Save to shape library… (the last was only in DRAW ▾ before).

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
- **Select a group, as a picture or a DXF is selected** (the user, 2026-11-02: "the
  selection in a group does not follow the workflow when selecting shapes in an image or
  a DXF"; docs/specs/leave-out-shapes.md, decision 10):
  - a click on any of its shapes selects the whole group, as a click on a traced contour
    selects its whole picture; its Browser row does the same, as a DXF's row does;
  - **Ctrl+click** on a shape of the selected group **leaves it out** of the selection,
    and Ctrl+click again puts it back, as for a picture or a DXF. The selection reads
    "Group 1, 2 shapes left out". Leaving out changes only what is selected, not the
    group;
  - Ctrl+click on a shape outside the group goes on as today (shapes by id).
- **One shape inside:** its row under the group in the Browser, or a **double-click** on
  it on the canvas, selects that shape alone, to change its values or drag its points
  (decision 3). A click elsewhere goes back to selecting whole groups.
- **Move, turn, scale:** a selected group has box handles, as a picture and a text have
  (docs/specs/box-handles.md). Dragging any of its drawn shapes moves the whole group.
  Texts in the group turn and scale with it (their height scales).
- **Add to a group** (the user, 2026-11-02: "how do I add or subtract to a group?"):
  select the group and the shapes or texts to add (Ctrl+click), then *Group* (Ctrl+G).
  With one group in the selection, *Group* adds the rest to it rather than making a new
  one; with two or more groups, they and the rest become one group, named after the first.
  Or right-click a free shape: *Add to group ▸* the project's groups. In the Browser, drag
  a shape's row onto a group's row.
- **Take out of a group:** select the shape alone (its Browser row, or a double-click),
  then right-click > *Remove from group*. It stays where it is,
  free. In the Browser, drag its row out of the group onto *Drawing*. A group left with no
  members goes.
- **Ungroup:** right-click > *Ungroup* (or Ctrl+Shift+G). The shapes stay where they are,
  free again.
- **Delete:** Del on a selected group deletes its shapes and texts, as one undo step.
- **Copy and paste:** copying a whole group pastes a new group (decision 6).
- **Library:** saving a group saves its shapes; inserting an entry of more than one shape
  makes a group named after the entry (decision 5).
- **Cut it:** with a group selected, *V-carve*, *Pocket*, *Profile* or any operation in the
  ribbon takes its shapes, as for any selection (the user, 2026-11-02: "when I insert it
  again I would like to be able to select it and choose V-carve"). A traced picture saved
  to the library and inserted is drawn polylines that V-carve as the trace did: checked on
  the golden ring (v19 image_vcarve), saved, inserted into an empty copy and carved, its
  program passing the checker.
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
- An operation's `select` may name a group, `{"group": 1}`, with the same optional
  `exclude` as a DXF or a picture: `{"group": 1, "exclude": [7]}` (decisions 8, 10). It
  cuts the group's shapes and its texts' letters as they are at each run, but the ones
  left out. A left-out shape that leaves the group is a problem to fix, as for a DXF.

## 5. The window

- **Canvas:** clicking a grouped shape selects the group: all its shapes highlighted, its
  box with handles. Ctrl+click on its shapes leaves them out and back in, as for a picture
  (§2). A selection box catches a group when it catches all of it (the same rule as for
  texts).
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
chipload group set part.chipload 1 --name "Door plate" --add 9,10 --remove 5 --add-texts 3
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
  Del, undo of each; Ctrl+G with a group and free shapes adds them; Remove from group.
- An operation made from a group cuts a shape added to the group later, not one taken out;
  after Ungroup it cuts the same shapes as before.
- Ctrl+click leaves a shape of the selected group out and back in; the operation gets
  `{"group": 1, "exclude": [...]}` and doesn't cut it; the group keeps it.

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
8. **An operation made from a whole group follows the group** (changed 2026-11-02 after
   the user's question: decision 8 seemed to counteract cutting a group). *Recommended:* as
   an operation made from a DXF or a picture follows it, shapes added to the group later
   are cut too, and shapes taken out aren't. Ungrouping turns the operation's selection
   into the shapes' ids, so what it cuts doesn't change; deleting the group leaves it with
   nothing to cut, flagged, as deleting its shapes does. In the file: an optional `group`
   in an operation's `select`. Alternative: the operation keeps the shapes' ids only.
9. **Adding to and taking out of a group** (§2): *Group* with one group selected adds to
   it, with several merges them; *Remove from group* on shapes picked inside it; dragging
   rows in the Browser. *Recommended:* no new tool to learn. Alternative: ungroup and
   group again.
10. **Selecting a group works as selecting a picture or a DXF** (the user, 2026-11-02):
   click for all of it, Ctrl+click to leave shapes out and back in, the operation
   following the group with `exclude`. *Recommended:* one way to pick everywhere. Leaving
   out is a selection, not a change to the group: *Remove from group* does that.
