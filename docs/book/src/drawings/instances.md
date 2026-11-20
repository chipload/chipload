# Instances

Each [part](../operations/parts.md) can be cut several times. Every extra time is an
**instance** of the part: the same shapes and operations, cut again somewhere else, moved,
turned and perhaps mirrored. Instances are linked to the part: change the part, and every
instance follows.

In the window:
- **Right-click the part** in the Browser: *Add instance* places it again beside what it
  cuts, 10 mm away; *Add mirrored instance* does the same, mirrored left to right.
- **On the canvas**, instances are placed in their setup's tab
  ([the canvas](../window.md#the-canvas)): each drawn where it is cut, with a badge below
  its bottom left: a four-way arrow, its part's name and its number (`Side #2`), and a
  mirror mark when it is mirrored. One that is off is dashed. Click an instance (its badge
  or outline) to select it, drag it to move it. Double-click it to open its part in
  *Drawing*. A selected instance has a round turn handle
  with a curved arrow: drag it to turn the instance about its middle. Under the pointer an
  instance's outline thickens and the pointer becomes a hand, so you see what a drag
  takes. Hold Shift while moving to keep it to 15° steps from where you grabbed it
  (straight across, up or at 45°). Turning snaps to quarter turns within 3°; hold
  Alt to turn freely. Escape during a drag puts it back; each drag is one undo step.
- **Drag the part in:** drag a part's row from the Browser onto a setup's tab of the
  canvas: its outline follows the pointer, on the grid's steps (hold Alt to place it
  freely), and letting go places a new instance there, not turned or mirrored, selected.
  Dropped on another setup's sheet beside the board, it goes on that setup. From
  *Drawing*, hold the dragged part over a setup's tab for half a second: the tab opens.
  Escape during the drag places nothing. A board cut in
  [pieces](../concepts/pieces.md) that already holds the part refuses it, and the Report
  says why.
- **More of them on the canvas:**
  - **Right-click an instance** (its badge, or its outline): *Duplicate*, *Turn 90° left*,
    *Turn 90° right*, *Mirror*, *Reset to where drawn*, *Don't cut* (or *Cut*) and
    *Remove*. *Reset to where drawn* puts it back where the part is drawn, not turned; its
    mirror and whether it is cut stay.
  - **Duplicate** places a new instance, turned and mirrored like this one and on the same
    board, beside everything the part cuts, and selects it. Duplicate again and the next
    goes beside that one.
  - **Ctrl+drag** (⌘ on a Mac) an instance to drag a copy of it off; the original stays.
    Escape during the drag drops the copy.
  - **Ctrl+C, then Ctrl+V,** with an instance selected, works as *Duplicate*, as many times
    as you paste. It copies the placement, not the shapes.
- **In the Browser**, each instance has a row with its tick under its setup ("Side #2"),
  never under its part: copies and placing are setups' work, so they are listed under the
  setup that cuts them, even with one board (*Setup 1*). Right-click it for the same menu
  as on the canvas, and *Move to setup ▸*; Delete removes the selected one.
- **Several setups:** drag a part's row onto a setup to cut it there, every instance of it
  moved (right-click it for *Move to setup ▸*); hold Ctrl (⌘ on a Mac) as you drop to
  place it there too, a new instance where the part is drawn (*Place on setup ▸*). Drag an
  instance's row onto another setup to cut it there, its number and place kept. The part's
  properties list *Where it is cut*, its instances by setup.
- **Its properties** set whether it is cut, its mirror, its rotation (any angle) and how
  far it is moved.
- **A grid:** the part's properties make one under **Instances**.

On the command line, `chipload part instance` places them one by one. With several parts,
give `--part`.

```sh
chipload part instance add part.chipload                  # beside the part, 10 mm from its cut
chipload part instance add part.chipload --angle 90 --mirror left-right
chipload part instance add part.chipload --at 300,0       # moved 300 mm right of the part
chipload part instance add part.chipload --like 3         # turned, mirrored and on the board of instance 3
chipload part instance list part.chipload                 # their numbers, places and turns
chipload part instance set part.chipload 3 --angle 45     # turn instance 3 (also --at, --mirror, --off, --on)
chipload part instance set part.chipload 3 --reset        # back where the part is drawn, not turned
chipload part instance remove part.chipload 3             # or --all: the part is cut once
chipload part instance array part.chipload --grid 3x2 --gap 10   # a grid in place of them
chipload part instance move part.chipload 3 --setup 2     # instance 3 cut from setup 2's board instead
chipload part set part.chipload 1 --order by-instance     # every operation on one instance, then the next
```

- **Boards:** each instance is cut from one setup's board. `add` and `array` put new
  instances on the board the part is cut from, or on the one `--setup` names; `move` puts
  an instance on another board, keeping its number and place. With several setups,
  `list` says each instance's board.

- **Numbers:** the part as first placed is instance 1 ([placed](#placing-the-part) or where
  drawn); the others are numbered from 2. Every instance is alike: instance 1 too can be
  mirrored, switched off, moved to another setup or removed, and numbers are kept when one
  is removed or moved.
- **Place:** an instance is mirrored first (`left-right` or `top-bottom`), then turned
  (`--angle`, degrees counter-clockwise), both about the middle of the part's box, then
  moved by `--at` from where the part is drawn. A new instance without `--at` goes
  beside everything the part cuts, 10 mm away, its bottom level with the part's.
- **Climb stays climb:** a mirrored instance is cut in the same direction as the part,
  not as a plain mirror image would be. Tabs, starts, tabs along X or Y and a pattern's
  lines turn with the instance, so every instance comes out the same.
- **A grid:** `part instance array` (or the **Instances** section) makes the part the
  grid's first cell and the other cells its instances, numbered row by row from 2, in
  place of the instances it had. *Pitch* is the distance from cell to cell; *gap* works
  the pitch out from the part's real cut, tool included. A negative pitch grows the grid
  left or down. `chipload copies` is the same command under its old name, kept for
  scripts: `copies --grid 3x2 --gap 10`, `copies --order by-instance`, and `copies --off`
  removes every instance.
- **Order:**
  - **by operation** (the default): each operation on every instance, then the next, so
    tool changes happen once;
  - **by instance:** every operation on one instance, then the next. The instances are cut
    in their listed order, a grid's row by row, back and forth. This was called *by copy*,
    and *by part* before Chipload 0.17; `--order by-copy` and `--order by-part` still
    work.
- **Checked:**
  - every instance is planned and checked like the part;
  - instances whose cuts would overlap each other, or another part, are refused when the
    project is run, and the commands warn as soon as you place one there;
  - instances have to fit the stock and the machine like anything else.
- **Limits:** up to 100 columns or rows in a grid, and 1 000 instances.
- **Display:** the setup's tab draws the instances where they are cut, the setup sheet
  faded.
  The setup sheet and the playback say which instance is being cut, and the sheet says
  where a part placed elsewhere than drawn is cut.

## Placing the part

The part's first instance can be placed too: cut moved and turned, while its shapes stay
where they are drawn. Its shapes are what you edit; every instance,
the first included, is a placement of them. This lets you lay parts out on a sheet without
moving the drawing, for instance when several parts come from one DXF.

In the window:
- **Right-click the part** in the Browser, *Place…*: its first instance is selected in
  its setup's tab, with its round turn handle, and its form opens with its rotation and
  how far it is moved. Drag it, turn it by its handle, or press Q for the Compass.
  *Reset placement* puts it back where drawn.
- **On the canvas**, once placed, it is `#1` in its setup's tab where it is cut, picked,
  dragged and turned like any instance. Its shapes stay where they are drawn, in
  *Drawing*; edit them there, and every instance follows. *Drawing* shows no board behind
  a part that isn't cut where it is drawn.
- **In the Browser**, its instances are under *Setup 1* ("Side #1", "Side #2"), the
  setups listed once the part is placed.
- **The Compass** (Q) works on a selected instance, the first included: drag an arrow to
  move it along, the ring to turn it, the centre to put it on a point, or type a value.
  It moves and turns instances only; copies and arrays are for shapes.

```sh
chipload part place part.chipload --at 300,0 --angle 90   # the part cut 300 mm right, turned
chipload part place part.chipload --reset                 # back where drawn
chipload part instance set part.chipload 1 --at 350,0     # the same, as instance 1
```

- **The part's own mirror** (`part set --mirror`) mirrors its shapes, so every instance;
  an instance's mirror mirrors that one only.
- **Its instances stay** where they were: they are placed from the part as drawn, not from
  instance 1.
- **The board:** a board that fits the drawing holds the part where it is cut, not its
  shapes where they are drawn.
- **Dowel holes** belong to the board: they are cut where drawn, once, never placed or
  repeated per instance.
- **Checked** like an instance: a placed part may not cut into another part. Parts cut
  where they are drawn may still overlap each other, as before.
- In a setup cut in [pieces](../concepts/pieces.md) the part can't be placed yet.

Instances aren't nested automatically yet: to fit different parts on one sheet, place each
part and its instances, or arrange them in CAD.

Projects saved before Chipload 0.38 kept a grid of copies; they open with that grid as
instances, cut in the same order, so the G-code stays the same.
