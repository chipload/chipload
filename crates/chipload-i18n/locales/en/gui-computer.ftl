# "This computer" in the Browser: Preferences and post-processors (crates/chipload-gui/src/app/computer.rs).

computer-posts = Post-processors
computer-posts-hover = What turns toolpaths into G-code for your machine: built in and yours
# Notes at the right end of a post's row in the Browser.
computer-post-built-in = built in
computer-post-yours = yours
computer-post-this-project = this project

computer-preferences = Preferences
computer-preferences-about = For every project on this computer.
computer-settings-folder = Settings folder
# $var is the name of an environment variable.
computer-settings-folder-none = none: set { $var } to keep settings and a library
computer-setup-sheet = Setup sheet
computer-setup-sheet-hover = An HTML page for the operator next to the exported files: stock, zero, tools in order, operations, times and warnings
computer-setup-sheet-write = write one with the G-code
computer-toolpath = Toolpath
computer-toolpath-auto = update automatically (View menu too)
computer-toolpath-auto-hover = Generate by itself half a second after each change
computer-units = Units
computer-units-hover = What lengths and feeds are shown and typed in. Projects stay in mm; the G-code's units are the machine's (Job settings > Machine)
computer-theme = Colour theme
computer-theme-system = System
computer-theme-light = Light
computer-theme-dark = Dark

computer-posts-count = { $builtin } built in, { $yours } of yours.
computer-posts-folder = Yours go in { $folder }
computer-posts-no-folder = No settings folder, so only the built-in posts.
# NAME.toml is a file name: keep it as it is.
computer-posts-about = A post of your own is two files: NAME.toml with its settings, and the template it names. Start from a copy of a built-in one; the guide's Post-processors page explains both.
computer-posts-reload = Read the folder again
computer-posts-reload-hover = After adding or changing a post's files

computer-post = Post-processor
computer-post-gone = This post is no longer there.
computer-post-unusable = Can't be used: { $error }
computer-post-use = Use in this project
computer-post-use-hover = Make it the project's post-processor
computer-post-use-copy-hover = Make it the project's post-processor: a copy goes into the project
computer-post-in-use = The project uses it
computer-post-no-project = Open a project first
