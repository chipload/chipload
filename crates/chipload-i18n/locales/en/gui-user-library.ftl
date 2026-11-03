# The library rows of a project tool and the machine, and copying values into and out of the project (crates/chipload-gui/src/app/user_library.rs).

# The user's library of tools and machine profiles, kept on this computer.
user-library-profile = Profile
user-library-tool = Library tool
user-library-same = same as in your library
user-library-differs = differs from your library: { $changes }
user-library-more-hover = Save as a new entry, or show it in the library
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
user-library-plunge-feed = Plunge feed
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
user-library-cutting-data = Cutting data
user-library-cutting-data-hover = Your own values for this tool, one row per material. Any material is used without a material, or for a material the tool has no row for. Filled into an operation when you choose a material, change its tool or add it; never changes an operation by itself.
user-library-any-material = Any material
user-library-material-gone = (a material no longer in your library)
user-library-row-summary = slot { $slot_feed } / { $slot_step_down } · clearing { $feed } / { $step_down } × { $stepover }
user-library-row-speed = Spindle speed
user-library-row-speed-hover = The speed these values are for. Choosing the material for a whole setup sets the tool's speed to it
user-library-row-own-speed = The tool's
user-library-slot-feed = Slot feed
user-library-slot-step-down = Slot step down
user-library-slot-hover = Full-width cuts: profiles, paths, V-carves, bores, a pocket's first ring
user-library-clearing-feed = Clearing feed
user-library-clearing-step-down = Clearing step down
user-library-clearing-hover = Cuts at a stepover: a pocket's rings, a surface
user-library-add-material = Add a material…
user-library-remove-row = Remove
user-library-remove-row-hover = Remove this material's values from the tool
user-library-tools-updated = Updated from your library:
user-library-tool-updated = T{ $tool }: { $changes }
user-library-tool-differs = T{ $tool }: your library's tool "{ $name }" has other values: { $changes }
user-library-update-all = Update all from library
user-library-update-all-hover = Every project tool that differs from its library tool takes the library's values; a speed from the setup's material stays. One undo step
user-library-update-all-none = Every tool from your library has its values
