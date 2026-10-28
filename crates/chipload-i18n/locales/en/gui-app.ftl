# The main window: opening, importing, saving and exporting, parts, rest passes, the
# selection (crates/chipload-gui/src/app/mod.rs).

# The buttons of a folded side panel; P is the key that folds and unfolds both panels.
app-show-browser-hover = Show the Browser (P: both panels)
app-show-properties-hover = Show Properties (P: both panels)
# The heading of the right-hand panel.
app-properties = Properties

app-start-hint = Import a DXF or SVG (Ctrl+I) to start a project, or open one (Ctrl+O).

app-cannot-open = Cannot open { $file }:
app-project = Project: { $file }
app-old-schema = This file is schema version { $version }; saving writes version { $current }.

app-new-project = New project.
# $what: a machine or tool from the user's library, e.g. machine "Router".
app-started-with = Started with { $what }.
app-new-project-next = Draw shapes from the Draw menu, or bring in a drawing with File > Import DXF into project. Check stock, machine and tool under Job settings.
app-units-missing = { $file } does not say which units it uses; choose mm or inch.
app-px-missing = { $file } gives its size in pixels only; choose its pixels per inch.
app-cannot-import = Cannot import { $file }:
app-new-project-from = New project from { $file } ({ $shapes } shapes).
app-import-next = Select contours, then New profile or New pocket. Check stock, machine and tool under Job settings.

# Followed by the list of problems.
app-project-incomplete = The project is not complete:
app-lost-geometry = Some operations lost their geometry: choose it again
app-could-not-update-drawing = Could not update the drawing:
app-could-not-add = Could not add { $file }:
app-could-not-move-drawing = Could not move the drawing:

# The distance between copies of a part; $dy and $gap carry their unit.
app-pitch-set = Pitch { $dx } × { $dy } ({ $gap } gap)
app-pitch-needs-plan = Could not set the pitch from a gap: the part has to be planned first.

app-deleted-part = { $count ->
    [0] Deleted part { $name } (Ctrl+Z brings it back)
    [one] Deleted part { $name } and its operation (Ctrl+Z brings it back)
   *[other] Deleted part { $name } and its { $count } operations (Ctrl+Z brings it back)
}
app-part-needed = A project needs at least one part

# A rest pass clears what a larger tool left in a pocket.
app-could-not-add-rest-pass = Could not add a rest pass: { $error }
app-rest-with-corners = This pocket has dogbone or T-bone corners, and rest passes can't be combined with them yet. The corners already reach where a rest pass would.
# $tool: a tool number such as T3.
app-rest-tool-missing = Tool { $tool } isn't in the project: give the pocket a tool first.
app-rest-no-smaller = No endmill smaller than { $tool } ({ $diameter }) in the project or your tool library. Add one under Tools, then right-click the pocket again.
app-now-rest-pass = Operation { $op } is now a rest pass of operation { $parent }; it is cut after the run of pockets it belongs to

app-generate-before-export = Generate (F5) before exporting G-code.
app-planning-then-export = Planning every part, then exporting
app-could-not-export = Could not export { $path }:
app-dxf-exported = Wrote { $path }.
app-could-not-export-dxf = The drawing was not exported:
app-sheet-failed = Exported the G-code, but could not write the setup sheet { $path }:
app-exported-n-files = Exported { $count } files: change the tool and set Z zero between them, keeping X and Y zero.
app-sheet-for-operator = Setup sheet for the operator: { $path }
app-exported-files = Exported { $count } files next to { $path }
app-exported-files-sheet = Exported { $count } files and the setup sheet { $sheet } next to { $path }
app-exported = Exported { $path }
app-exported-sheet = Exported { $path } and the setup sheet { $sheet }

app-cannot-save-yet = The project can't be saved yet:
app-could-not-save = Could not save { $path }:
app-saved = Saved { $path }

# What is selected on the canvas.
app-nothing-selected = Nothing selected.
# $count contours selected; $what says how (one of the texts below); $list their numbers.
app-selected = { $count } selected ({ $what }): { $list }
app-selection-layer = layer { $layer }
app-selection-dxf = DXF { $id }
app-selection-image = image { $id }
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
