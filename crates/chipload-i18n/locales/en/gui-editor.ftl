# The operations being edited: names given to new operations, and the short description
# of what an operation cuts (crates/chipload-gui/src/editor.rs).

# The name of a duplicated operation.
editor-name-copy = { $name } (copy)

# What an operation cuts, shown in the operation list. The stock is the material block.
editor-summary-whole-stock = the whole stock
# Where the shapes come from: a DXF file, a traced picture or a text, by number.
editor-summary-dxf = drawing { $id }
editor-summary-image = image { $id }
editor-summary-text = text { $id }
editor-summary-group = group { $id }
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
