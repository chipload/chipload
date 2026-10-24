# The operations being edited: names given to new operations, and the short description
# of what an operation cuts (crates/chipload-gui/src/editor.rs).

# Names of new operations, numbered per kind; the user can rename them. A profile cuts
# along a contour, a pocket clears the area inside it, a V-carve cuts with a V-shaped bit,
# a follow path runs the tool along the line itself, holes are drilled or bored, a
# surface faces the whole stock flat.
editor-name-profile = Contorneado { $number }
editor-name-pocket = Cajera { $number }
editor-name-vcarve = Tallado en V { $number }
editor-name-path = Seguir trayectoria { $number }
editor-name-holes = Taladros { $number }
editor-name-surface = Planeado { $number }
# The name of a duplicated operation.
editor-name-copy = { $name } (copia)

# What an operation cuts, shown in the operation list. The stock is the material block.
editor-summary-whole-stock = todo el material en bruto
# Where the shapes come from: a DXF file, a traced picture or a text, by number.
editor-summary-dxf = DXF { $id }
editor-summary-image = imagen { $id }
editor-summary-text = texto { $id }
# A drawing layer (user's name); the source is one of the three texts above.
editor-summary-layer = capa { $layer }
editor-summary-layer-of = capa { $layer } de { $source }
editor-summary-all-of = todo { $source }
# Some of the source's shapes were left out of the operation.
editor-summary-left-out = { $source } salvo { $count } excluidos
editor-summary-not-in-drawing = formas que no están en el dibujo
editor-summary-no-geometry = sin geometría
