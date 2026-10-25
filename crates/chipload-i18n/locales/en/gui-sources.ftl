# The Browser's Drawing folder: the project's DXFs with their layers, its pictures, texts
# and drawn shapes (crates/chipload-gui/src/app/sources.rs).

# The folder's name: everything that is drawn, a noun.
sources-drawing = Drawing
sources-drawing-hover = DXFs, pictures, texts and drawn shapes; right-click to add one
sources-add-dxf = Add DXF…
sources-add-dxf-hover = Another part: a DXF placed and updated on its own
# "File > Import DXF" names a menu item.
sources-add-dxf-disabled-hover = File > Import DXF starts a project from one
sources-import-image = Import image…
sources-import-image-hover = Trace a picture into shapes, for V-carving
sources-add-text = Add text
sources-add-text-hover = Write a text, then click where it starts
sources-no-shapes = No shapes yet.
# The name of DXF files in the file dialog's filter.
sources-dxf-filter = DXF drawings

# Rows of the folder. $file: the picture's file name; $text: what the text says.
sources-image-title = { $file } (image { $id })
sources-text-title = { $text } (text { $id })
# Rows for the shapes drawn in the app, by kind.
sources-rectangles = Rectangles
sources-circles = Circles
sources-polylines = Polylines
sources-other-paths = Other paths

sources-dxf-hover = Click selects the whole DXF: an operation made from it follows its updates. Its position and scale show in Properties; right-click for more
sources-image-hover = Click selects the whole picture: an operation made from it follows its retracing. Its size and place show in Properties; double-click for its trace settings
sources-text-hover = Click selects the whole text: an operation made from it follows its changes. What it says, its font, size and place are in Properties
sources-drawn-hover = Click selects these shapes; Ctrl+click adds them
sources-layer-hover = Click selects this layer of this DXF: an operation made from it cuts only this DXF's layer

# Right-click menu items, verbs.
sources-select = Select
sources-show = Show
sources-hide = Hide
sources-place = Place…
sources-place-hover = Move, turn or scale this DXF, in its properties
sources-update-from-dxf = Update from a DXF…
sources-update-from-path = Update from a path…
sources-remove-dxf-hover = Remove this DXF and its shapes
sources-trace-settings = Trace settings…
sources-trace-settings-hover = Threshold, smoothing, outlines or centre lines; its size and place are in its properties
sources-remove-image-hover = Remove the picture and its shapes
sources-remove-text-hover = Remove the text and its letters

# Followed by the project's problems, one per line.
sources-project-incomplete = The project is not complete:
# Followed by the error on its own line.
sources-remove-dxf-failed = Could not remove the DXF:
sources-explode = Explode into shapes…
sources-explode-hover = Turn this DXF's shapes into drawn shapes you can edit; the link to the DXF file goes
sources-explode-failed = Could not explode the DXF:
sources-join-failed = Could not join the shapes:
sources-drawn-layer-hover = Click to select these shapes; the eye hides them
sources-shape-hover = Click to select it; Ctrl+click adds or removes; right-click to rename, copy or delete
sources-rename = Rename…
sources-copy-shape = Copy
sources-delete-shape = Delete
