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
dialogs-path-open = A .chipload project or a .dxf:
dialogs-path-save-title = Save project
dialogs-path-save = Save the project to:
dialogs-path-export-title = Export G-code
dialogs-path-export = Export the checked G-code to:
dialogs-path-update-title = Update drawing
dialogs-path-update = The DXF:
dialogs-path-image-title = Import image
dialogs-path-image = A picture (PNG, JPEG or BMP):
dialogs-path-add-dxf-title = Add DXF
dialogs-path-add-dxf = The DXF or SVG to add to the project:
dialogs-path-update-dxf-title = Update DXF
dialogs-path-update-dxf = The new version of the DXF or SVG:

# In the window's title bar, for a project never saved.
dialogs-title-new-project = new project

## Explode a DXF, join close ends (docs/specs/explode-and-join.md §5)

# $name is the DXF's name in the Browser.
dialogs-explode-title = Explode { $name } into shapes
dialogs-explode-body = Its shapes become drawn shapes you can edit, move and delete. Operations keep cutting the same shapes. The project no longer knows the DXF file: updating it from the DXF won't be possible.
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
