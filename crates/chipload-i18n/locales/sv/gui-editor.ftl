# The operations being edited: names given to new operations, and the short description
# of what an operation cuts (crates/chipload-gui/src/editor.rs).

# Names of new operations, numbered per kind; the user can rename them. A profile cuts
# along a contour, a pocket clears the area inside it, a V-carve cuts with a V-shaped bit,
# a follow path runs the tool along the line itself, holes are drilled or bored, a
# surface faces the whole stock flat.
editor-name-profile = Konturfräsning { $number }
editor-name-pocket = Ficka { $number }
editor-name-vcarve = V-gravyr { $number }
editor-name-path = Följ bana { $number }
editor-name-holes = Hål { $number }
editor-name-surface = Planfräsning { $number }
# The name of a duplicated operation.
editor-name-copy = { $name } (kopia)

# What an operation cuts, shown in the operation list. The stock is the material block.
editor-summary-whole-stock = hela ämnet
# Where the shapes come from: a DXF file, a traced picture or a text, by number.
editor-summary-dxf = DXF { $id }
editor-summary-image = bild { $id }
editor-summary-text = text { $id }
# A drawing layer (user's name); the source is one of the three texts above.
editor-summary-layer = lager { $layer }
editor-summary-layer-of = lager { $layer } i { $source }
editor-summary-all-of = hela { $source }
# Some of the source's shapes were left out of the operation.
editor-summary-left-out = { $source } utom { $count } utelämnade
editor-summary-not-in-drawing = former som inte finns i ritningen
editor-summary-no-geometry = ingen geometri
