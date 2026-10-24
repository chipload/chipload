# The user's library in the Browser and in Properties (crates/chipload-gui/src/app/library_view.rs).

# Buttons in a library folder's menu and properties.
library-view-new-tool = New tool
library-view-new-machine = New machine
library-view-import = Import…
library-view-export = Export…

# The Browser's root for what is kept on this computer, not in the project.
library-view-this-computer = This computer
library-view-this-computer-hover = Your preferences, tools, machines and post-processors, kept on this computer for every project. A project keeps copies of what it uses, so it cuts the same in a year and on another computer.
library-view-preferences = Preferences
library-view-preferences-hover = The setup sheet, updating the toolpath automatically, the colour theme
library-view-no-settings-folder = No settings folder
# $var is the name of an environment variable.
library-view-no-settings-folder-hover = Set { $var } to keep a library
library-view-tools = Tools
library-view-tools-hover = Your tool library: drag a tool onto the project's Tools to use it
library-view-machines = Machines
library-view-machines-hover = Your machine profiles: the values new projects start with
library-view-tool-hover = Click to edit it; drag it onto the project's Tools to use it there

# Commands on a library tool or machine profile.
library-view-add-to-project = Add to project
library-view-add-to-project-hover = Copy it into the project as a tool of its own
library-view-use-in-project = Use in this project
library-view-use-in-project-hover = Copy its values into the project's machine (undo with Ctrl+Z)
library-view-entry-disabled-hover = Open a project, and save or revert the changes first
# A button: make this entry the one new projects start with.
library-view-make-default = Make default
library-view-make-default-hover = New projects start with it
library-view-delete-hover = Take it out of your library; projects keep their copies
library-view-save-hover = Write the changes into your library
library-view-revert = Revert
library-view-revert-hover = Drop the changes
library-view-not-saved = not saved

library-view-tool-library = Tool library
library-view-tool-library-about = A project keeps a copy of each tool it uses. Click a tool under This computer to edit it; drag it onto the project's Tools, or right-click it, to use it.
library-view-machines-about = A project keeps a copy of its machine's values. Right-click a profile to use it in this project; the default is what new projects start with.
library-view-tool-count = { $count ->
    [one] { $count } tool
   *[other] { $count } tools
} in your library.
library-view-machine-count = { $count ->
    [one] { $count } machine
   *[other] { $count } machines
} in your library.

library-view-library-tool = Library tool
library-view-tool-gone = This tool is no longer in your library.
library-view-machine-profile = Machine profile
library-view-profile-gone = This profile is no longer in your library.
library-view-entry-about = In your library, for every project. Changes are kept when you press Save.
# $tools lists the project's tools, like "T1, T3".
library-view-project-copies = This project has a copy: { $tools }
library-view-project-machine = This project's machine came from it.

library-view-deleted = Deleted "{ $name }" from your library; projects keep their copies
library-view-saved = Saved to your library

# Notes at the right end of a library entry's row in the Browser.
library-view-note-default = default
library-view-note-unsaved = unsaved
