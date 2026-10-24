# The operations being edited: names given to new operations, and the short description
# of what an operation cuts (crates/chipload-gui/src/editor.rs).

# Names of new operations, numbered per kind; the user can rename them. A profile cuts
# along a contour, a pocket clears the area inside it, a V-carve cuts with a V-shaped bit,
# a follow path runs the tool along the line itself, holes are drilled or bored, a
# surface faces the whole stock flat.
editor-name-profile = Konturfräsen { $number }
editor-name-pocket = Tasche { $number }
editor-name-vcarve = V-Gravur { $number }
editor-name-path = Pfad folgen { $number }
editor-name-holes = Bohrungen { $number }
editor-name-surface = Planfräsen { $number }
# The name of a duplicated operation.
editor-name-copy = { $name } (Kopie)

# What an operation cuts, shown in the operation list. The stock is the material block.
editor-summary-whole-stock = das ganze Rohteil
# Where the shapes come from: a DXF file, a traced picture or a text, by number.
editor-summary-dxf = DXF { $id }
editor-summary-image = Bild { $id }
editor-summary-text = Text { $id }
# A drawing layer (user's name); the source is one of the three texts above.
editor-summary-layer = Layer { $layer }
editor-summary-layer-of = Layer { $layer } von { $source }
editor-summary-all-of = alles von { $source }
# Some of the source's shapes were left out of the operation.
editor-summary-left-out = { $source }, aber { $count } ausgelassen
editor-summary-not-in-drawing = Formen nicht in der Zeichnung
editor-summary-no-geometry = keine Geometrie
