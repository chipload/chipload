# Spec: a drawing space and a space per setup

Status: **approved and built 2026-11-20** (all decisions as recommended; §14). From the user's testing of
nesting and instances: "when we select the setup we should not see the parts, only the
instances, it gets quite messy on the screen", then "could we separate drawing space and
setup space?", and "we should be able to drag in parts from the browser to the setup to
create instances".

Replaces §7 (sheets side by side) and §8a (a part or a setup is shown) of
parts-and-setups.md for the window; the model of parts-and-setups.md §2 stays as it is.
Builds on canvas-handling.md §3 (instances from the canvas).

Window behaviour only: no file format, G-code, message or command changes, except
`part instance set --reset` (§8).

## 1. Why

Since schema 41 the file keeps two kinds of thing apart: a **part** is shapes and
operations, drawn where they are drawn; a **setup** is a board holding **instances**, each a
placement of a part (moved, turned, perhaps mirrored). The window still draws both on one
canvas: in the setup view, every part's shapes lie where drawn, with a dashed *not cut here*
box round those cut elsewhere, and every instance as a faded ghost over them. After nesting,
the board's layout and the drawing lie on top of each other.

Each space shows one kind of thing: shapes are drawn and edited in the drawing, placed and
laid out on a board. CAM users know a separate design and layout space, and slicers lay
parts out on plates the same way.

## 2. The spaces

```
 [ Drawing ] [ Setup 1 ] [ Setup 2 (back) ] [ + ]
```

- **Tabs along the top of the canvas:** *Drawing*, then one per setup in the Browser's order,
  named as the setup is (a back "Setup 2 (back)", decision 3), and *+* to add a setup (as
  the project's *Add setup*). The active tab is drawn as selected; the others plain.
- **One space at a time** fills the canvas. Each keeps its own view (pan and zoom), so going
  back finds it as it was. A new setup's view fits its board.
- **What is selected picks the tab** (as parts-and-setups.md §8a picked the view):
  - a part, one of its operations or their passes, a drawn shape, a layer, a group, a text,
    a picture, a model or a drawing file → *Drawing*;
  - a setup, its Stock, Work zero or board surfaces, an instance, or the project → that
    setup's tab (the project: the active setup's);
  - anything else (tools, the library, styles) leaves it as it is.
- **Clicking a tab** shows that space; a setup's tab makes it the active setup and selects
  it, so its form opens.
- **3D, the simulation and playback** work in either space on what it shows (as the part
  and setup views do today).

## 3. The drawing space

- **Shows:** every part's shapes where drawn, the active part's in full colour, the others'
  dimmed (still picked, snapped to and edited; picking one makes its part active). Drawing
  files, pictures, texts, models, construction geometry and layers as today.
- **Doesn't show:** instances, badges, *not cut here* boxes, nesting, other setups' sheets.
- **The board, faintly:** when the active part is cut where it is drawn (an instance at the
  drawn place on its board), that board's outline and work zero show faintly behind the
  shapes, to draw against and snap to its corners and edges (decision 2). Its rulers and
  cursor readout are the drawing's coordinates, which are that board's.
- **Toolpaths:** the active part's, planned alone where drawn (today's part view,
  parts-and-setups.md §8a). Generate here is the part's; Export is the active setup's, as
  today with a part shown.
- **Tools:** every drawing and modify tool, handles, the Compass on shapes, the shape
  library, text, pictures: everything that makes or changes shapes lives here.

## 4. A setup's space

- **Shows:** its board, work zero, board surfaces and dowel holes; **its instances drawn
  solid** in the part's colours (no longer faded: here they are the real thing), each with
  its badge (`#2`); the toolpaths of the setup; the other setups as sheets beside it
  (today's §7, kept: decision 4), a click on a sheet goes to its tab.
- **Doesn't show:** the drawn shapes, construction geometry, pictures or texts as drawn;
  only what the instances cut, where they are cut.
- **An instance is picked, moved, turned, mirrored, duplicated, copied and removed** as in
  canvas-handling.md §3 and instances.md: its outline (anywhere, no shape lies under it now)
  or badge, the turn handle, the Compass, its right-click menu.
- **Double-click an instance:** its part in the drawing space, selected.
- **Nest…, Face the board, Add surface** act on this setup.
- **Drawing keys** (L, R, C and the rest) or a drawing tool from the ribbon switch to the
  drawing space first, then start the tool there (decision 5). A modify tool with shapes
  selected was started from the drawing space anyway.
- **Snapping** while moving an instance: to the board's corners and edges and to other
  instances' outlines' corners (their parts' points, placed), as a shape's move snaps today.

## 5. Dragging parts in

- **Drag a part's row from the Browser onto a setup's space:** a new instance of it is
  dropped where the pointer is: the middle of the part's box under the pointer, not turned,
  not mirrored, on that setup, selected, one undo step. While dragging, the part's outline
  follows the pointer on the board.
- **Onto a sheet** beside the board: the instance goes on that sheet's setup, where dropped
  on it.
- **Onto a tab:** holding a dragged part over a setup's tab for half a second opens that tab,
  so a part can be dragged from the drawing space onto any board (decision 6).
- **Refused, with the reason in the status bar,** where the board can't take it: a setup in
  pieces already holding the part (pieces.md: each part at most once), a part with no
  shapes.
- **Snapping:** the dropped outline snaps as an instance's move does (§4); Alt drops it
  freely; Escape during the drag drops nothing.
- The Browser's own drags stay: a part's row onto a setup's row moves every instance there,
  Ctrl+drop places one where drawn (parts-and-setups.md §8).

## 6. The simple case

One board, each part once where drawn (parts-and-setups.md §2a):

- **The two spaces look alike:** the part where drawn, on the board, in both; the drawing
  space with the board faint behind it (§3), the setup's with the part as its instance,
  solid. A new project opens on the drawing space, so a beginner draws, adds operations,
  generates and exports without leaving it.
- **No badge, no instance row** for a part once where drawn, as today (§2a); the setup's
  tab and the Browser's *Setups › Setup 1* are there for when they're wanted.

## 7. What goes

- the *not cut here* boxes (`placed_elsewhere`): a drawing space shows no cuts, and a setup
  space no drawn shapes;
- instance ghosts over the drawing, and the rule that the selected instance shows on its
  part's shapes "to be placed" (part-placement.md §6): an instance is placed in its setup's
  space;
- the canvas switching its content when the selection changes without anything saying so:
  the tabs show which space it is.

## 8. Reset to where drawn

- **An instance's menu** (canvas and Browser) gains *Reset to where drawn*: moved back to
  where the part is drawn and not turned; its mirror and on or off stay (as the part's
  *Reset placement*, decision 7). Greyed when it is already there.
- **Command line:** `chipload part instance set P 3 --reset` (with `--at` or `--angle` it
  is refused: one or the other).

## 9. Command line

Nothing changes but `--reset`: the command line has no canvas. `part instance add --at`
already places an instance anywhere, and `--like` like another.

## 10. Guide

- **window.md:** the canvas's tabs and which space shows what; the screenshots renewed.
- **first-part.md:** drawing happens in *Drawing*; one line that the setup's tab shows the
  board as it will be cut.
- **drawings/instances.md, drawings/nesting.md, concepts/setups.md, operations/parts.md:**
  instances are
  placed in their setup's space; dragging parts in; *Reset to where drawn*.
- **drawings/placement.md:** placing a part is placing its instance in the setup's space.

## 11. Tests

- **Selection picks the tab:** each kind of row in §2 and the tab it shows; the project
  shows the active setup's; tools leave it.
- **What each space shows:** the drawing space has no instance ghosts and no *not cut here*
  boxes; a setup's space has no drawn shapes and its instances solid; the faint board only
  when the active part is cut where drawn.
- **Views are kept** per tab across switches.
- **Dragging a part in:** dropped where the pointer is, on the right setup (board, sheet,
  a tab held open), one undo step; refused on a pieces setup holding it; Escape drops
  nothing.
- **A drawing key in a setup's space** switches to the drawing space and starts the tool.
- **Double-click an instance** opens its part in the drawing space.
- **Reset to where drawn**, in the window and `--reset` on the command line.
- **The simple case:** a new project opens on *Drawing*; the usual job (draw or import, a
  profile, generate, export) never needs a tab click, counted by a window test as §2a does.
- **Pictures:** each space headless, each language, a narrow window (the tabs fit or
  scroll).

## 12. Decisions (recommendations first)

1. **Separate spaces, with tabs that follow the selection.** Alternative: one canvas, the
   drawn shapes hidden where nothing is cut (the smaller proposal of 2026-11-20).
2. **The board shows faintly in the drawing space when the active part is cut where
   drawn,** to draw against and snap to. Alternative: never a board in the drawing space.
3. **A back is a tab of its own,** "Setup 2 (back)". Alternative: one tab per board with a
   front/back flip.
4. **The other setups stay as sheets beside the active board,** as now, besides their tabs.
   Alternative: tabs only, one board at a time.
5. **Drawing keys in a setup's space switch to the drawing space** and start the tool.
   Alternative: they do nothing there.
6. **A dragged part held over a setup's tab opens it.** Alternative: drag only within the
   space shown (switch tabs first).
7. **Reset to where drawn keeps the mirror** and on or off. Alternative: clear the mirror
   too.
8. **The active part's shapes in full colour, the other parts' dimmed** in the drawing
   space, all editable. Alternative: the other parts hidden, as the part view does today.

## 13. Order of work

1. The tabs, each space's content and the selection rule (§2–§4, §7); the views kept.
2. Dragging parts in (§5), the tab held open.
3. *Reset to where drawn* and `--reset` (§8).
4. The guide and its screenshots (§10), in each step's commit for what it changes.

## 14. As built

- **Step 1** (the tabs, each space's content, the selection rule, the views kept):
  - **The project leaves the space as it is** (changed while building, from §2): a
    project opens on *Drawing* (§6) with the project's form open, and selecting the
    project in the Browser doesn't move a user out of the drawing they are in.
  - **The tabs lie over the canvas's top edge,** below the ruler, so the canvas keeps its
    whole area: a fitted or zoomed view stays as it was.
  - **Opening Nest** shows its setup's tab, where its preview is drawn.
  - **A setup's legend** lists instances, selected and stock only.
  - **The other parts' shapes in Drawing** are dimmed as the part view's ghosts were.
- **Step 2** (parts dragged in):
  - **The drop snaps to the grid** (Alt: free), the part's middle under the pointer.
    Snapping an instance to the board's corners and to other instances (§4, §5) is not
    built: instances don't snap while moved either. Later, if wanted.
  - **A refusal goes to the Report** with its reason (as *Add instance*'s), not the status
    bar.
  - **The part's outline** dragged is the contours its operations cut
    (`chipload_pipeline::parts::part_outlines`, shared with the instances' outlines).
- **Step 3** (*Reset to where drawn*): in an instance's menu, canvas and Browser, greyed
  when it is already there; `part instance set --reset` (refused with `--at` or
  `--angle` by the argument parser). *Reset placement* of the part is the same for its
  first instance.
- **The badge, after the user's testing (2026-11-20):** every instance has one, a part
  cut once where drawn too (it was left out, as parts-and-setups.md §2a had it, and the
  instance then had no visible handle until moved); it reads the part's name and the
  number ("Side #2"), with a mirror mark (two triangles across a dashed line, upright for
  left to right, across for top to bottom) when the instance is mirrored; and it sits
  below the instance's bottom left, clear of the turn handle above.
- **A part with nothing to cut** (no operation yet) has no instance drawn in a setup's
  space: there is nothing to show or place; it is listed in the Browser as before.
- **A right-click on empty canvas in a setup's space** (the user, 2026-11-20) gives the
  setup's own menu, the same items as its Browser row (`setup_menu_items`): export it,
  face the board, nest and nest again, a new part, a new setup, its back side, remove it.
- **The boards stay put** (the user, 2026-11-20: "when selecting a setup the selected setup
  moves to the leftmost position … I want them to stay"): every board lies in one row in
  the setups' order (`row_offsets`), the active one where it is cut; making another active
  moves the view by as much as its board moved, so nothing moves on screen. This replaces
  each setup's own view (§2): the setups' tabs share one, the drawing has its own.
