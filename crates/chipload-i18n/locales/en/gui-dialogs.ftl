# File dialogs, typed-path prompts, the About window and the window title
# (crates/chipload-gui/src/app/dialogs.rs).

# Help > User guide, until the guide is published online. Keep the file names as they are.
dialogs-user-guide = The user guide comes with each release as chipload-guide-<version>.zip (open index.html). From the source: packaging/docs/build-book.sh.

# Names of file types in the system's file dialogs.
dialogs-filter-projects = Chipload projects
dialogs-filter-dxf = DXF drawings

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
dialogs-path-add-dxf = The DXF to add to the project:
dialogs-path-update-dxf-title = Update DXF
dialogs-path-update-dxf = The new version of the DXF:

# In the window's title bar, for a project never saved.
dialogs-title-new-project = new project
