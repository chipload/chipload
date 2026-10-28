# The header: the top bar and the ribbon of grouped buttons (crates/chipload-gui/src/app/header.rs).

# Export's hover text. $count is the number of parts that are cut (always more than one),
# $part the name of the part the preview shows.
header-export-hover = Save the checked G-code
header-export-hover-parts = Save the checked G-code of every part that is cut ({ $count } parts)
header-export-hover-parts-shown = Save the checked G-code of every part that is cut ({ $count } parts); the preview shows only "{ $part }"
header-export-generating-hover = Save the G-code once it is generated
header-export-first-hover = Generate first

# The top bar's icon buttons.
header-open-hover = Open a project
header-save-hover = Save the project
header-undo = Undo
header-redo = Redo
header-help = Help
# The colour theme button: what is set now, a click goes on to the next.
header-theme-system-hover = Colour theme: as the system is set. Click for the next.
header-theme-light-hover = Colour theme: light. Click for the next.
header-theme-dark-hover = Colour theme: dark. Click for the next.

# The tab with the open project's file name.
header-new-project = New project
header-tab-unsaved-hover =
    { $path }
    Unsaved changes
header-tab-not-saved-hover = Not saved yet

# The ribbon's groups: their captions (shown in capitals) open the menu of the same name.
header-group-file = File
header-group-draw = Draw
header-group-operation = Operation
header-group-library = Library
header-group-view = View
header-group-hover = Everything in { $group }

# FILE: short words under icons.
header-new = New
header-new-hover = New project
header-open = Open
header-import = Import
header-import-hover = New project from a DXF
header-save = Save

# DRAW: short words under icons, and what each does.
header-circle = Circle
header-circle-hover = Draw a circle: centre, then a point on it
header-rectangle = Rectangle
header-rectangle-hover = Draw a rectangle: two opposite corners
header-polyline = Polyline
header-polyline-hover = Draw a polyline: click points
header-text = Text
header-text-hover = Write a text: click where it starts, then type it in Properties
header-image = Image
header-image-hover = Trace a picture into shapes, for V-carving or a follow path
header-add-dxf = Add DXF
header-add-dxf-hover = Another part: a DXF placed and updated on its own
# "FILE > Import": the ribbon group FILE and its Import button.
header-add-dxf-first-hover = Import a DXF first (FILE > Import)
header-import-image-menu = Import image…
header-add-dxf-menu = Add DXF to project…

# OPERATION: "Follow" is the short word under the icon for a Follow path operation.
header-follow = Follow
# $operation is what the operation does (a hover text of its own).
header-needs-selection-hover = { $operation }. Select shapes first.
header-select-shapes-first = Select shapes first
header-new-profile = New Profile
header-new-pocket = New Pocket
header-new-vcarve = New V-carve
header-new-follow-path = New Follow path
header-new-holes = New Holes
header-new-surface = New Surface

# LIBRARY
header-tools = Tools
header-tools-hover = Your tool library, under This computer in the Browser
header-machines = Machines
header-machines-hover = Your machine profiles, under This computer in the Browser
header-tool-library = Tool library

# VIEW
header-measure = Measure
header-measure-hover = Two clicks give the distance, ΔX, ΔY and the angle. What is shown is on the canvas's bar.

# Generate and Export at the ribbon's right end.
header-export = Export
header-cancel = Cancel
header-cancel-hover = Stop generating
header-generate = Generate
header-generate-hover = Plan the toolpaths, write and check the G-code
header-arc = Arc
header-arc-hover = Draw an arc: start, end, then a point it passes through
header-slot = Slot
header-slot-hover = Draw a slot: the centres of its ends, then its width
header-polygon = Polygon
header-polygon-hover = Draw a regular polygon: the centre, then a corner (the number of sides on the status bar)
header-group-modify = Modify
header-offset = Offset
header-offset-hover = A shape at a distance from the selected ones: move to the side, click or type the distance
header-fillet = Fillet
header-fillet-hover = Round a corner of the selected shape with an arc: click the corner; Shift+click rounds every corner
header-chamfer = Chamfer
header-chamfer-hover = Cut a corner of the selected shape straight across: click the corner; Shift+click cuts every corner
header-trim = Trim
header-trim-hover = Remove the piece of a drawn shape under the cursor, up to where other shapes cross it: click the piece
header-extend = Extend
header-extend-hover = Grow an open drawn shape's straight end until it meets the next shape: click near the end
header-break = Break
header-break-hover = Split a drawn shape at the point clicked: an open one in two, a closed one opens there
header-union = Union
header-union-hover = Join the selected closed shapes into one: everything any of them covers
header-subtract = Subtract
header-subtract-hover = Cut the selected closed shapes out of the shape you click next
header-intersect = Intersect
header-intersect-hover = Keep what the selected closed shapes all cover
