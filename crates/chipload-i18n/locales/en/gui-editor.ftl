# The operations being edited: names given to new operations, and the short description
# of what an operation cuts (crates/chipload-gui/src/editor.rs).

# Names of new operations, numbered per kind; the user can rename them. A profile cuts
# along a contour, a pocket clears the area inside it, a V-carve cuts with a V-shaped bit,
# a follow path runs the tool along the line itself, holes are drilled or bored, a
# surface faces the whole stock flat.
editor-name-profile = Profile { $number }
editor-name-pocket = Pocket { $number }
editor-name-vcarve = V-carve { $number }
editor-name-path = Follow path { $number }
editor-name-holes = Holes { $number }
editor-name-surface = Surface { $number }
# The name of a duplicated operation.
editor-name-copy = { $name } (copy)

# What an operation cuts, shown in the operation list. The stock is the material block.
editor-summary-whole-stock = the whole stock
# Where the shapes come from: a DXF file, a traced picture or a text, by number.
editor-summary-dxf = DXF { $id }
editor-summary-image = image { $id }
editor-summary-text = text { $id }
# A drawing layer (user's name); the source is one of the three texts above.
editor-summary-layer = layer { $layer }
editor-summary-layer-of = layer { $layer } of { $source }
editor-summary-all-of = all of { $source }
# Some of the source's shapes were left out of the operation.
editor-summary-left-out = { $source } but { $count } left out
editor-summary-not-in-drawing = shapes not in the drawing
editor-summary-no-geometry = no geometry

# What an operation still lacks, under its field and in the list of problems. "Use
# selection" is the button that gives the operation the selected shapes.
editor-choose-side = choose the profile side (inside, outside or on)
editor-no-geometry = no geometry: select contours, then Use selection
