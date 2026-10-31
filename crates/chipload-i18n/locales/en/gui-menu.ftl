# The menu bar and its menus (crates/chipload-gui/src/app/menu.rs).

menu-language = Language
menu-language-system = System
menu-language-system-hover = The operating system's language when Chipload has it, else English
menu-language-review = { $language } (review wanted)
menu-language-review-hover = Translated, not yet read by a native speaker who machines. Corrections are welcome: see the user guide, Languages.

# File menu.
menu-new-project = New project
menu-open = Open…
menu-open-recent = Open recent
# Empties the list of recent projects.
menu-clear-list = Clear list
menu-import-dxf = Import DXF or SVG…
menu-import-image = Import image…
menu-import-image-hover = Trace a picture into shapes, for V-carving
menu-open-path = Open path…
menu-import-image-path = Import image from path…
menu-several-dxfs = Several drawing files: update one from the Browser (right-click it)
menu-add-dxf = Add DXF or SVG to project…
menu-add-dxf-hover = A DXF or SVG into this project, beside what is there: placed and updated on its own
menu-add-dxf-path = Add DXF or SVG from path…
menu-save = Save
menu-save-as = Save as…
menu-save-path = Save to path…
menu-export = Export G-code…
menu-export-path = Export G-code to path…
menu-export-dxf = Export drawing as DXF…
menu-export-dxf-hover = The shapes the canvas shows, on their layers, for a laser cutter, a CAD program or a quote
menu-construction = Construction
menu-construction-hover = Construction geometry: shown dotted and snapped to, but never cut. For guide lines and circles to draw against
menu-toggle-construction = Construction geometry on or off
menu-quit = Quit

# Edit: undo, and the selected drawn shapes.
menu-undo = Undo
menu-redo = Redo
menu-only-drawn = Only drawn shapes; imported ones change only with their file
menu-copy = Copy
menu-paste = Paste
menu-mirror-left-right = Mirror left–right
menu-mirror-up-down = Mirror up–down
menu-mirror-hover = Flip the selected drawn shapes about the middle of their box
menu-delete-shapes = Delete shapes
menu-delete-shapes-hover = Deletes the selected drawn shapes; imported ones change only with their file
# A selector: the selection written as JSON, for scripts.
menu-copy-selector = Copy as selector

# Job menu.
menu-cancel-generation = Cancel generation
menu-generate = Generate

# View menu.
menu-fit-view = Fit view
menu-top-view = Top view
menu-measure = Measure
menu-measure-hover = Two clicks give the distance, ΔX, ΔY and the angle
menu-3d-view = 3D view
menu-z-scale = Z scale in 3D
menu-show-grid = Show grid
menu-show-grid-hover = Lines on the snap grid, a stronger one every ten
menu-show-legend = Show legend
menu-show-browser = Show Browser
menu-show-properties = Show Properties
menu-fold-panels = Fold both panels
menu-open-panels = Open both panels
menu-panels-hover = The largest canvas: the Browser and Properties folded to strips
menu-auto-update = Update toolpath automatically
menu-auto-update-hover = Generate by itself half a second after each change
menu-show-toolpath = Show toolpath
menu-show-toolpath-hover = Hide the toolpath to see the drawing or the simulated stock
menu-show-drawing = Show drawing
menu-show-drawing-hover = Hide the drawing to see the stock or the toolpath alone
menu-show-images = Show images
menu-show-images-hover = The pictures traced into shapes, faintly over the stock
# The stock: the material being cut, as the simulation shows it.
menu-simulate = Simulate
menu-simulation-hint = Show the stock as the program leaves it: every move cut with the tool's shape (darker is deeper; red is below the stock). Display only.
menu-units = Units
menu-units-mm-hover = Lengths in mm, feeds in mm/min
# The unit's name, as a choice next to "mm".
menu-units-inch = inch
menu-units-inch-hover = Lengths in inches, feeds in in/min; projects stay in mm
menu-theme = Colour theme
menu-theme-system = System
menu-theme-system-hover = Light or dark, as the operating system is set
menu-theme-light = Light
menu-theme-light-hover = Light panels and canvas
menu-theme-dark = Dark
menu-theme-dark-hover = Dark panels and canvas

# Draw menu.
menu-draw-circle = Circle
menu-draw-rectangle = Rectangle
menu-draw-polyline = Polyline
menu-draw-text = Text
menu-draw-text-hover = Write a text, then click where it starts
menu-draw-frame = Frame…
menu-draw-frame-hover = A frame for a sign: round the selection, or to a size, with border lines

# Help menu.
menu-user-guide = User guide
menu-shortcuts = Keyboard shortcuts
menu-about = About and licences

# The status bar under the canvas. What the toolpath is of, with several parts:
menu-status-part-of = part { $part } of { $count }
menu-status-all-parts = all parts
# $time: how long the program runs, e.g. "12 min 30 s".
menu-status-cycle = cycle { $time }
menu-status-time-hover = Estimated cycle time (the report has the details)
menu-status-active-part = active part: { $name }
menu-status-active-part-hover = New operations go into the active part (Browser, right-click a part)
menu-status-units-hover = Lengths are shown in these (View > Units)
# $step: the grid's step with its unit, e.g. "10 mm".
menu-status-grid = grid { $step }
menu-status-grid-hover = The grid's step: clicks while drawing snap to it (G hides it)
menu-status-post-hover = The post-processor (Machine)
# Pieces in all, from every part that is cut.
menu-status-pieces = { $count ->
    [one] { $count } piece
   *[other] { $count } pieces
}
menu-status-copies = { $count ->
    [one] { $count } copy
   *[other] { $count } copies
}
# The cursor in program coordinates (from the work zero), after the drawing's X and Y.
menu-status-work = (work X { $x }  Y { $y })
# $target: what the cursor snapped to, e.g. "end of #4".
menu-snapped-to = — snapped to { $target }
menu-generating = Generating…
# $snapped: empty, or "— snapped to …" with a space before it.
menu-dragging = Dragging to ({ $x }, { $y }){ $snapped } (Alt: no snapping; on a segment Ctrl inserts a point, Shift bends it)
menu-status-hint = Click a contour to select it · Ctrl+click adds or removes · right-click selects its layer · Esc clears
menu-status-simulate-hint = Simulate: drag to pan (3D: orbit) · click an operation's number to open it · F6 back to Edit
menu-join-ends = Join ends…
menu-join-ends-hover = Join open drawn polylines whose ends almost meet into one: the selected ones, or all when none is selected
menu-join-ends-disabled-hover = There are no drawn shapes. A DXF's shapes can be joined once it is exploded (right-click it in the Browser).
menu-draw-arc = Arc
menu-draw-slot = Slot
menu-draw-polygon = Polygon
menu-polygon-sides = Sides
menu-modify-offset = Offset
menu-modify-fillet = Fillet
menu-modify-chamfer = Chamfer
menu-modify-trim = Trim
menu-modify-extend = Extend
menu-modify-break = Break
menu-modify-union = Union
menu-modify-subtract = Subtract
menu-modify-intersect = Intersect
menu-modify-array = Array
menu-modify-polar-array = Polar array
