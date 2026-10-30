# The Browser: the project as one tree of folders and rows (crates/chipload-gui/src/app/browser.rs).

# The panel's caption, shown in capitals.
browser-caption = Browser
browser-no-project = No project open.
browser-new-project = New project

# A row's eye (shown on the canvas or not) and tick (cut or not).
browser-eye-hide-hover = Hide it on the canvas; it is still cut
browser-eye-show-hover = Hidden on the canvas (still cut): show it
browser-tick-on-hover = Cut: untick to leave it out of the program
browser-tick-off-hover = Not cut: tick to cut it

# A project tool's link to the user's tool library, at its row's right end ($name: the
# library tool's name).
browser-link-changed = { $name } (changed)
browser-link-missing = (not in library)

# The kinds of operation, and what a new one of each does.
browser-profile = Profile
browser-profile-hover = A profile operation for the selected contours
browser-pocket = Pocket
browser-pocket-hover = A pocket operation for the selected contours
browser-vcarve = V-carve
browser-vcarve-hover = A V-bit along the centre line of the selected closed contours, as deep as makes the cut as wide as the shape
browser-follow-path = Follow path
browser-follow-path-hover = The tool on the line of the selected shapes, open or closed: engraving, grooves and score lines
browser-holes = Holes
browser-holes-hover = The selected circles: drilled with a drill, or bored with a helix with an endmill
browser-surface = Surface
browser-surface-hover = The whole stock taken down flat with a wide endmill: flatten the spoilboard or face the stock. Needs no selection

# Job settings and Tools.
browser-job-settings = Machine
browser-job-settings-hover = The machine: its post-processor, G-code units, safe Z, travel and tool changes, shared by every setup
browser-work-zero = Work zero
browser-work-zero-hover = Where the G-code's X0 Y0 Z0 is. Click to set it, or select it and drag it in the view.
browser-job-settings-problems-hover = Machine, post-processor, units and travel. { $count ->
        [one] { $count } problem
       *[other] { $count } problems
    }
browser-tools = Tools
browser-tools-hover = The project's tools: click to see them all and add one

# Parts: groups of operations, each cut with its copies before the next.
browser-setups = Setups
browser-setups-hover = The boards the project is cut from, one after the other: each its own program, with its stock and work zero. Click one to make it the active one
browser-add-setup = Add a setup
browser-add-setup-hover = Another board: its own stock, work zero and parts, cut as a program of its own
browser-setup-parts = { $count ->
    [one] { $count } part
   *[other] { $count } parts
}
browser-setup-active-hover = The active setup: shown in full colour, simulated and generated. Click it for its board; its work zero is beneath it
browser-setup-hover = Click to make it the active setup: shown, simulated and generated
browser-move-to-setup = Move to setup
browser-new-part = New part
# At a part row's right end: the part new operations go into.
browser-part-active = active
browser-part-active-hover = The active part: new operations go here. Click for its copies; right-click for more
browser-part-hover = Click for its copies; double-click to make it the active part; right-click for more
# A verb: make this the active part.
browser-activate = Activate
browser-activate-hover = New operations go into the active part
browser-export-part = Export G-code of this part…
browser-export-part-hover = Write the G-code of this part only, as if every other part were switched off; its setup sheet too when that setting is on.
browser-new-operation = New operation
browser-select-shapes-first = Select shapes first
browser-move-up = Move up
browser-move-down = Move down
# $count: the part's operations.
browser-delete-part = { $count ->
        [0] Delete part
        [one] Delete part and its operation
       *[other] Delete part and its { $count } operations
    }
browser-delete-part-disabled-hover = A project needs at least one part
browser-part-empty-active = Select shapes, then add an operation
browser-part-empty = No operations: drag one here

# An operation's row and its right-click menu.
browser-no-name = (no name)
# $count: contours the operation cuts that are hidden on the canvas.
browser-op-hidden = { $count } hidden
browser-duplicate = Duplicate
browser-add-rest-pass = Add rest pass
# A tool for a rest pass: $tool the project tool's number, $size its diameter.
browser-rest-project-tool = T{ $tool } · { $size } endmill
browser-rest-library-tool = { $name } · { $size } endmill
browser-from-library = From your tool library
browser-copied-into-project-hover = Copied into the project's Tools
browser-add-rest-pass-hover = A smaller tool that clears only what this pocket's tool leaves, in its shapes and depths. It follows the pocket, and is cut after the pockets next to it with the same tool
browser-make-rest-pass = Make it a rest pass of operation { $number }
browser-make-rest-pass-hover = It copies that pocket: as its rest pass it follows it, and is cut after the pockets next to it with the same tool
browser-delete = Delete

# A rest pass's row. "Rest": a rest pass, a smaller tool after the pocket's own.
browser-rest = Rest
browser-rest-pass-hover = A rest pass: it clears what the tool before it leaves, in its pocket's shapes and depths, and is cut after the pockets next to its pocket with the same tool
browser-add-floor-pass = Add floor pass
browser-add-floor-pass-hover = An end mill that clears the floor between the walls of strokes too wide to carve, in this V-carve's shapes and depths. It is cut before the V-carve
browser-floor = Floor
browser-floor-pass-hover = A floor pass: an end mill clears the floor between its V-carve's walls, before the V-carve; the V-bit cleans up where it can't reach
browser-remove = Remove

# An operation row's hover text: its lines.
browser-op-rest-after = rest after T{ $tool }
browser-op-not-cut = Not cut (ticked off)
browser-op-hidden-hover = Cuts { $count } hidden { $count ->
        [one] contour
       *[other] contours
    }: show them in Drawing to see them
browser-op-drag-hover = Drag to reorder; right-click for more

# Properties: what is selected on the canvas.
browser-selection = Selection
browser-copy-as-selector = Copy as selector
browser-copy-as-selector-hover = The selection as JSON, for an operation in a project file

# A thin line between operation rows where the tool changes; $tool the new tool's number.
browser-tool-change = tool change → T{ $tool }
