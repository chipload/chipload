# The main window: opening, importing, saving and exporting, parts, rest passes, the
# selection (crates/chipload-gui/src/app/mod.rs).

# The heading of the right-hand panel.
app-properties = Properties

app-start-hint = Import a DXF or SVG (Ctrl+I) to start a project, or open one (Ctrl+O).

app-cannot-open = Cannot open { $file }:
app-project = Project: { $file }
app-old-schema = This file is schema version { $version }; saving writes version { $current }.

app-new-project = New project.
# $what: a machine or tool from the user's library, e.g. machine "Router".
app-started-with = Started with { $what }.
app-new-project-next = Draw shapes from the Draw menu, or bring in a drawing with Add DXF or SVG. Check the board under the setup, the machine under Machine, and the tool under Tools.
app-units-missing = { $file } does not say which units it uses; choose mm or inch.
app-px-missing = { $file } gives its size in pixels only; choose its pixels per inch.
app-cannot-import = Cannot import { $file }:
app-new-project-from = New project from { $file } ({ $shapes } shapes).
app-import-next = Select contours, then New profile or New pocket. Check the board under the setup, the machine under Machine, and the tool under Tools.

# Followed by the list of problems.
app-project-incomplete = The project is not complete:
app-lost-geometry = Some operations lost their geometry: choose it again
app-could-not-update-drawing = Could not update the drawing:
app-could-not-add = Could not add { $file }:
app-could-not-move-drawing = Could not move the drawing:

# The distance between a grid's cells; $dy and $gap carry their unit.
app-pitch-set = Pitch { $dx } × { $dy } ({ $gap } gap)
app-pitch-needs-plan = Could not set the pitch from a gap: the part has to be planned first.
app-instance-needs-plan = Could not place the new instance: the part has to be planned first.

app-deleted-part = { $count ->
    [0] Deleted part { $name } (Ctrl+Z brings it back)
    [one] Deleted part { $name } and its operation (Ctrl+Z brings it back)
   *[other] Deleted part { $name } and its { $count } operations (Ctrl+Z brings it back)
}
app-part-needed = A project needs at least one part
app-board-surface-stays = A surface of the whole board belongs to its setup and stays first in it

# A rest pass clears what a larger tool left in a pocket.
app-could-not-add-rest-pass = Could not add a rest pass: { $error }
app-could-not-add-floor-pass = Could not add a floor pass: { $error }
app-setup-refused = Could not change the setups: { $error }
app-dowels-placed = Placed the dowel holes again, clear of both sides' cuts. The work zero moved with the first pin if it was there.
app-setup-has-operations = The setup has parts with operations: move them to another setup, or delete them, first.
app-exported-setup = Setup “{ $name }” only: make another setup active and export it too.
app-floor-no-endmill = No end mill in the project or your tool library
app-rest-with-corners = This pocket has dogbone or T-bone corners, and rest passes can't be combined with them yet. The corners already reach where a rest pass would.
# $tool: a tool number such as T3.
app-rest-tool-missing = Tool { $tool } isn't in the project: give the pocket a tool first.
app-rest-no-smaller = No endmill smaller than { $tool } ({ $diameter }) in the project or your tool library. Add one under Tools, then right-click the pocket again.
app-now-rest-pass = Operation { $op } is now a rest pass of operation { $parent }; it is cut after the run of pockets it belongs to

app-generate-before-export = Generate (F5) before exporting G-code.
app-planning-then-export = Planning every part, then exporting
app-planning-part-then-export = Planning part { $name }, then exporting
app-could-not-export = Could not export { $path }:
app-dxf-exported = Wrote { $path }.
app-could-not-export-dxf = The drawing was not exported:
app-sheet-failed = Exported the G-code, but could not write the setup sheet { $path }:
app-exported-n-files = Exported { $count } files: change the tool and set Z zero between them, keeping X and Y zero.
app-sheet-for-operator = Setup sheet for the operator: { $path }
app-exported-files = Exported { $count } files next to { $path }
app-exported-files-sheet = Exported { $count } files and the setup sheet { $sheet } next to { $path }
app-exported = Exported { $path }
app-export-all-running = Planning and writing every setup…
app-exported-all = Exported { $setups } setups, { $count } files, next to { $path }
app-export-all-failed = Every setup could not be exported; nothing was written
app-export-all-stopped = The export of every setup stopped unexpectedly; nothing may have been written
app-exported-sheet = Exported { $path } and the setup sheet { $sheet }

app-cannot-save-yet = The project can't be saved yet:
app-could-not-save = Could not save { $path }:
app-saved = Saved { $path }

# What is selected on the canvas.
app-nothing-selected = Nothing selected.
# $count contours selected; $what says how (one of the texts below); $list their numbers.
app-selected = { $count } selected ({ $what }): { $list }
app-selection-layer = layer { $layer }
app-selection-dxf = drawing { $id }
app-selection-image = image { $id }
app-selection-model = model { $id }
app-selection-text = text { $id }
# $source: one of the three texts above.
app-selection-left-out = { $source }, { $count } left out
app-selection-contours = contours
# Open contours (not closed) are not cut.
app-open-not-cut = Open (not cut): { $list }

# File menu, for a project started without a drawing.
app-import-dxf-into-project = Import DXF or SVG into project…
app-import-dxf-into-project-path = Import DXF or SVG into project from path…
# File menu, a new version of the project's drawing.
app-update-drawing-from-dxf = Update drawing from DXF or SVG…
app-update-drawing-from-path = Update drawing from path…

# The Nest window (docs/specs/nesting.md §7)
nest-menu = Nest…
nest-menu-hover = Nest…: every instance of the setup's parts placed on its board as close as they fit, apart by the gap; a preview first, in a dialog
nest-again = Nest again
nest-again-hover = Nest the setup again with the gap, margin and turns it was nested with last, no dialog
nest-all = Nest all setups again
nest-all-hover = Nest every board again as one pool per stock: the first filled first, then the next; what fits on none onto new boards. A board left empty is kept.
nest-all-disabled-hover = Needs two setups or more that can be nested (a size of their own, not two-sided or in pieces)
nest-all-running = Nesting every setup again…
nest-title = Nest: { $setup }
nest-title-one = Nest
nest-board-follows = The board follows the drawing: nesting gives it this size, its corner at the drawing's origin.
nest-board = Board
nest-gap = Gap
nest-gap-hover = The least distance between what two instances cut: a web of board that keeps the sheet stiff and the tabs holding
nest-margin = Margin
nest-margin-hover = The room left at the board's edge, for screws or clamps
nest-goal = Goal
nest-goal-hover = What the nest makes as small as it can, after placing as much as it can
nest-goal-compact = Compact
nest-goal-compact-hover = The smallest box round the parts: a block in the lower-left corner, the rest of the board an L round it
nest-goal-low = Keep the top free
nest-goal-low-hover = The parts as low as they go, across the whole width: a strip across the top is left free
nest-goal-left = Keep the right side free
nest-goal-left-hover = The parts as far left as they go, up the whole height: a strip down the right is left free
nest-part = Part
nest-count = Times
nest-count-hover = How many times the part is cut: instances are added or removed first, never the part itself
nest-turns = Turns
nest-turns-none = As it lies
nest-turns-half = Half turns
nest-turns-quarter = Quarter turns
nest-turns-every-15 = Every 15°
nest-running = Nesting…
nest-stale = Settings changed: Preview to see the nest
nest-apply = Apply
nest-preview = Preview
nest-stopped-unexpectedly = Nesting stopped unexpectedly
