# The new project dialog (crates/chipload-gui/src/app/new_project.rs, docs/specs/new-project-dialog.md).

new-project-title = New project
new-project-material = Material
new-project-material-none = None
new-project-thickness = Thickness
# The board: fitted round the drawing with a margin, or a size.
new-project-board = Board
new-project-fit = Fit the drawing, margin
new-project-size = Size
new-project-z-zero = Z zero
new-project-z-zero-hover = Where the machine's Z0 is set: on the top of the board, or on the spoilboard under it
new-project-z-top = Top of the board
new-project-z-spoilboard = Spoilboard
new-project-machine = Machine
new-project-machine-none = Chipload's own settings (no machine in your library)
# Under the machine: its post-processor, G-code units (mm or inch) and how tools are changed.
new-project-machine-summary = { $post } · { $units } · { $change }
new-project-tools = Tools
new-project-search = Search
# Under the tool list: how many are ticked and their numbers ("T1, T2").
new-project-tools-count = { $count ->
    [0] No tools: add one before generating
    [one] 1 tool: { $list }
   *[other] { $count } tools: { $list }
}
new-project-no-library-tools = Your tool library is empty: the project starts with a 6 mm end mill. Add your tools under This computer > Tools.
new-project-create = Create
