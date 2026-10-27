# Help > Keyboard shortcuts (crates/chipload-gui/src/app/shortcuts.rs).

shortcuts-window = Keyboard shortcuts

# The window's group headings: menus (File, Edit, View, Help), the Generate button, and
# what is done on the canvas or while drawing a shape.
shortcuts-group-file = File
shortcuts-group-edit = Edit
shortcuts-group-generate = Generate
shortcuts-group-view = View
shortcuts-group-help = Help
shortcuts-group-canvas = Canvas
shortcuts-group-drawing = Drawing

# What each keyboard shortcut does.
shortcuts-copy-selector = Copy the selection as a selector (JSON)
shortcuts-save-as = Save as
shortcuts-redo = Redo
shortcuts-copy = Copy the selected drawn shapes
shortcuts-paste = Paste them (at the cursor, over the canvas)
shortcuts-new = New project
shortcuts-open = Open a project
shortcuts-import = New project from a DXF
shortcuts-save = Save
shortcuts-export = Export G-code (once generated)
shortcuts-quit = Quit
shortcuts-undo = Undo
shortcuts-move-up = Move the operation up
shortcuts-move-down = Move the operation down
shortcuts-generate = Generate the toolpath and G-code
shortcuts-fit = Fit the view
shortcuts-top-view = Top view
shortcuts-grid = Show or hide the grid
shortcuts-measure = Measure: two clicks give the distance and angle
shortcuts-panels = Fold or open the side panels, for a larger canvas
shortcuts-this-list = This list

# Mouse actions and keys that aren't menu shortcuts: the keys column (key names as on the
# keyboard), then what they do.
shortcuts-click = Click
shortcuts-click-select = Select a contour
shortcuts-ctrl-click = Ctrl+click
shortcuts-ctrl-click-what = Add a contour to the selection, or take it out
shortcuts-right-click = Right-click
shortcuts-right-click-what = Select a contour or its layer; reset a moved ring start
shortcuts-scroll = Scroll
shortcuts-scroll-what = Zoom at the cursor
shortcuts-drag = Drag
shortcuts-drag-what = Pan (top view); orbit (3D view)
shortcuts-pan-drag = Right or middle drag, Shift+drag
shortcuts-pan-drag-what = Pan (3D view)
shortcuts-delete = Delete
shortcuts-delete-what = Delete the selected drawn shapes
shortcuts-escape = Escape
shortcuts-escape-clear = Clear the selection
# A ring: a profile's closed path; its start can be dragged on the canvas.
shortcuts-drag-start = Drag a ring's start
shortcuts-drag-start-what = Move where the ring starts
shortcuts-drop = Drop a file
shortcuts-drop-what = Open a .chipload project, or start one from a .dxf
shortcuts-click-point = Place a point (snaps to ends, centres and middles)
shortcuts-alt = Alt
shortcuts-alt-what = Hold: don't snap
shortcuts-enter = Enter, double-click
shortcuts-enter-what = Finish a polyline; with no tool, draw with the last one again
shortcuts-backspace = Backspace
shortcuts-backspace-what = Take back the last point
shortcuts-escape-stop = Stop drawing
shortcuts-drag-handle = Drag a handle
shortcuts-drag-handle-what = Move a point, the centre or a corner; turn a rectangle
shortcuts-drag-middle = Drag a segment's middle
shortcuts-drag-middle-what = Insert a point there
shortcuts-shift-drag-middle = Shift+drag a segment's middle
shortcuts-shift-drag-middle-what = Bend the segment into an arc
shortcuts-draw-polyline = Draw a polyline
shortcuts-draw-circle = Draw a circle
shortcuts-draw-rectangle = Draw a rectangle
shortcuts-type-value = Type a number
shortcuts-type-value-what = An exact point: x,y  @dx,dy  length  length<angle
shortcuts-shift = Shift
shortcuts-shift-what = Hold: the line in steps of 15°
