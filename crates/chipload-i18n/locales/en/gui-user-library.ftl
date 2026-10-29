# The library rows of a project tool and the machine, and copying values into and out of the project (crates/chipload-gui/src/app/user_library.rs).

# The user's library of tools and machine profiles, kept on this computer.
user-library-library = Library
user-library-use-profile = Use profile…
user-library-use-tool = Use library tool…
user-library-save-to-library = Save to library
user-library-save-to-library-named = Save to library…
user-library-save-to-library-hover = Copies these values into your library, for other projects
user-library-save-to-linked-profile-hover = Write these values into the profile it is linked to, for other projects
user-library-save-to-linked-tool-hover = Write these values into the library tool it is linked to, for other projects
user-library-save-as-new = Save as new…
user-library-save-as-new-hover = Keep these values in your library under another name
user-library-update-from-library = Update from library
user-library-update-from-library-hover = Copy the library's values into the project (undo with Ctrl+Z)
user-library-show-in-library = Show in library
user-library-show-in-library-hover = Open it under This computer in the Browser
# $var is the name of an environment variable.
user-library-no-settings-folder-set = No settings folder found; set { $var }
user-library-no-settings-folder-use = No settings folder found; set { $var } to use a library.
user-library-no-settings-folder = No settings folder
user-library-kept-in = Kept in { $path }

user-library-name = Name
# The tool's number (T1, T2, …) in a project.
user-library-tool-number = Tool number
user-library-tool-number-hover = The id it gets in a project where that id is free
user-library-cutting-values = Cutting values
user-library-cutting-values-hover = Filled into a new operation with this tool when there is no operation above to copy from. Never changes an existing operation.
user-library-feed = Feed
user-library-plunge-feed = Plunge feed
user-library-step-down = Step down
user-library-stepover = Stepover
user-library-safe-z = Safe Z
user-library-max-overcut = Max depth below stock
user-library-max-overcut-hover = How far a cut may go below the stock (into the spoilboard)

user-library-settings-not-saved = Settings not saved: { $error }
user-library-recent-gone = { $path } no longer exists; it was taken off the recent list.
# Followed by the reason on the next line.
user-library-post-named = Post-processor "{ $name }":
user-library-post-chosen = Post-processor: { $name }
user-library-no-machine-profile = There is no machine profile "{ $name }".
user-library-no-library-tool = There is no library tool "{ $name }".
user-library-no-project-tool = The project has no tool { $id }.
user-library-machine-profile-named = Machine profile "{ $name }"
user-library-library-tool-named = Library tool "{ $name }"
# $what is user-library-machine-profile-named or user-library-library-tool-named.
user-library-no-values-changed-status = { $what }: no values changed
user-library-values-changed-status = { $what }: { $count } value(s) changed
user-library-changes-heading = { $what }:
user-library-no-values-changed = no values changed

user-library-profile-gone = The profile it came from is not in your library.
user-library-library-tool-gone = The library tool it came from is not in your library.
user-library-no-tool-to-save = The project has no tool to save.
# Followed by the reason on the next line.
user-library-not-saved = Not saved to the library:
user-library-saved = Saved "{ $name }" to your library

# Dialog titles.
user-library-save-machine-title = Save machine profile
user-library-save-tool-title = Save tool to library
user-library-name-in-library = Name in your library:
user-library-replaces = Replaces "{ $name }" in your library.
# Buttons.
user-library-replace = Replace

user-library-tool-exists = There is already a tool "{ $name }".
user-library-machine-exists = There is already a machine "{ $name }".
# The names a new library entry gets ("New tool 2" if taken).
user-library-new-tool = New tool
user-library-new-machine = New machine
