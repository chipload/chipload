# File dialogs, typed-path prompts, the About window and the window title
# (crates/chipload-gui/src/app/dialogs.rs).

# Help > User guide, until the guide is published online. Keep the file names as they are.
dialogs-user-guide = The user guide comes with each release as chipload-guide-<version>.zip (open index.html). From the source: packaging/docs/build-book.sh.

# Names of file types in the system's file dialogs.
dialogs-filter-projects = Chipload projects
dialogs-filter-dxf = Drawings (DXF, SVG)

dialogs-selector-copied = Selector copied to the clipboard

dialogs-about-title = About Chipload
dialogs-about-appimage = The Linux AppImage also contains libxkbcommon and libxcb-xkb (MIT-style licences); their copyright files are in usr/share/doc/chipload/bundled inside the AppImage.
dialogs-third-party = Third-party licences

# Asked when a DXF doesn't say whether it was drawn in millimetres or inches.
dialogs-units-title = Drawing units
dialogs-units-unknown = { $file } does not say which units it uses.
dialogs-units-question = What units was it drawn in?
# A button: the drawing was drawn in inches.
dialogs-inch = inch
dialogs-units-cancelled = Drawing not imported: its units are unknown.
dialogs-px-title = Pixels per inch
dialogs-px-unknown = { $file } gives its size in pixels only.
dialogs-px-question = How many pixels make an inch?
dialogs-px-96 = 96 (Inkscape, browsers)
dialogs-px-72 = 72 (older Illustrator)
dialogs-px-other = Other:
dialogs-px-use = Use
dialogs-px-cancelled = Drawing not imported: its size is unknown.
dialogs-export-dxf-title = Export drawing as DXF
dialogs-export-dxf-inch = In inches (otherwise mm)
dialogs-export-dxf-hidden = Hidden layers and groups too
dialogs-export-dxf-construction = Construction geometry too (on layers of their own)
dialogs-export-dxf-selection = Only the selected shapes
dialogs-export-dxf-save = Export…
dialogs-export-dxf-path = Export to path…
dialogs-path-export-dxf = The DXF to write:
dialogs-filter-dxf-file = DXF drawing

dialogs-unsaved-title = Unsaved changes
dialogs-unsaved-body = The project has changes that aren't saved. Save them first?
dialogs-save-hover = Save, then go on
dialogs-dont-save = Don't save
dialogs-dont-save-hover = Throw the changes away and go on

# Another program saved the open project's file.
dialogs-changed-title = { $file } changed on disk
dialogs-changed-body = Another program saved the project. Load its version? Your unsaved changes stay one Ctrl+Z away.
dialogs-load-it = Load it
dialogs-load-it-hover = Show the file's version; Ctrl+Z brings yours back
dialogs-keep-mine = Keep mine
dialogs-keep-mine-hover = Keep your changes; the next Save overwrites the file

# Windows where a file's path is typed in: a title and what to type.
dialogs-path-open-title = Open path
dialogs-path-open = A .chipload project, or a .dxf or .svg:
dialogs-path-save-title = Save project
dialogs-path-save = Save the project to:
dialogs-path-export-title = Export G-code
dialogs-path-export = Export the checked G-code to:
dialogs-path-update-title = Update drawing
dialogs-path-update = The DXF or SVG:
dialogs-path-image-title = Import image
dialogs-path-image = A picture (PNG, JPEG or BMP):
dialogs-path-add-dxf-title = Add DXF or SVG
dialogs-path-add-dxf = The DXF or SVG to add to the project:
dialogs-path-update-dxf-title = Update drawing file
dialogs-path-update-dxf = The new version of the DXF or SVG:

# In the window's title bar, for a project never saved.
dialogs-title-new-project = new project

## Explode a DXF, join close ends (docs/specs/explode-and-join.md §5)

# $name is the DXF's name in the Browser.
dialogs-explode-title = Explode { $name } into shapes
dialogs-explode-body = Its shapes become drawn shapes you can edit, move and delete. Operations keep cutting the same shapes. The project no longer knows the DXF file: updating it from the DXF won't be possible.
dialogs-explode-image-body = Its shapes become drawn shapes you can edit, move and delete. Operations keep cutting the same shapes. The picture itself goes from the project: it can't be traced again.
dialogs-explode-text-body = Its letters become drawn shapes you can edit, move and delete. Operations keep cutting the same shapes. The words can't be edited again.
dialogs-explode-join = Join ends closer than
dialogs-explode = Explode
dialogs-join-title = Join ends
dialogs-join-body = Open polylines among the { $count } selected { $count ->
    [one] shape
   *[other] shapes
} whose ends are closer than the gap become one, on the same layer. A chain that comes back to its start is closed.
dialogs-join-body-all = Open drawn polylines whose ends are closer than the gap become one, on the same layer. A chain that comes back to its start is closed.
dialogs-join-gap = Join ends closer than
dialogs-join = Join

# V-carve inlays (docs/specs/vcarve-inlay.md §4)
inlay-ribbon = Inlay…
inlay-ribbon-hover = A V-carve inlay of the selected shapes: a pocket here, and a mirrored plug in a board of its own that is glued into it
inlay-menu = New inlay…
inlay-title = V-carve inlay
inlay-body = The selected shapes are carved as a pocket in this board, and as a plug, mirrored, in a new setup: a board of its own. The plug is glued face down into the pocket, and its back planed off.
inlay-vbit = V-bit
inlay-none = None
inlay-depth = Pocket depth
inlay-depth-hover = How deep the pocket is cut in the base
inlay-plug-start = Plug start depth
inlay-plug-start-hover = How deep in its board the plug's outline is taken: its face lies this deep in the pocket, and the rest of the pocket's depth is left for glue
inlay-gap = Glue gap
inlay-gap-hover = How much deeper the plug board is carved, so its back stands this far above the base and squeezed-out glue has room
inlay-floor = Floor end mill
inlay-floor-hover = An end mill for the floor passes of both. The plug's channel is mostly floor
inlay-thickness = Plug board thickness
inlay-thickness-hover = The board the plug is cut from. What is left under its channel holds it together until it is planed off
inlay-make = Make inlay
inlay-no-vbit = Choose a V-bit: the project has none, and neither has your tool library
inlay-made = Made the inlay: operation { $pocket } is the pocket, operation { $plug } the plug, in setup { $setup }, its part mirrored left to right.
inlay-refused = Couldn't make the inlay.
# New sign… (docs/specs/sign-maker.md §3).
sign-title = New sign
sign-body = The words, a frame round them that follows them, and every operation that cuts the sign, in a part of its own. Change any of it afterwards.
sign-words = Words
sign-words-hint = What the sign says; Enter starts a new line
sign-around = Round the selection
sign-around-body = { $count ->
    [one] The sign goes round the selected shape, which is carved with the V-bit. The frame follows it, and a picture traced again.
   *[other] The sign goes round the { $count } selected shapes, which are carved with the V-bit. The frame follows them, and a picture traced again.
}
sign-around-words = Write words instead
sign-around-words-hover = Make the sign round new words, as with nothing selected
sign-font = Font
sign-height = Height
sign-style = Style
sign-style-hover = Engraved: the letters V-carved. Raised: the background inside the innermost border cut down round the letters. Inlay: the letters inlaid from a second board.
sign-style-engraved = Engraved
sign-style-raised = Raised
sign-style-inlay = Inlay
sign-relief = Background depth
sign-relief-hover = How deep the background round the raised letters is cut, with the end mill
sign-edge = Edge
sign-edge-chamfer = Chamfer
sign-edge-hover = A chamfer this wide round the outline, cut with the V-bit before the sign is cut out
sign-vbit = V-bit
sign-end-mill = End mill
wizard-library-tool-hover = From your tool library: copied into the project when you confirm, not before
wizard-library-tool-gone = A tool chosen from your library isn't in it any more: choose another
sign-board = Board
sign-fit-board = Fit round the sign
sign-fit-board-hover = The stock fitted round the drawing with this margin, this thick. Off: the stock stays as it is.
sign-thickness = Thickness
sign-board-margin = Margin
sign-adds = Adds: { $operations }
sign-make = Make sign
sign-no-tools = Choose a V-bit and an end mill, the project's or your tool library's.
sign-made = Made the sign: { $count } operations in a part of its own. Change the words and the frame follows.
sign-refused = Couldn't make the sign.
inlay-mark-gap = back, { $gap } above
inlay-mark-surface = surface
inlay-mark-face = plug face, { $depth }
inlay-mark-pocket = pocket, { $depth }
