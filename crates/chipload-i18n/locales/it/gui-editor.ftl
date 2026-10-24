# The operations being edited: names given to new operations, and the short description
# of what an operation cuts (crates/chipload-gui/src/editor.rs).

# Names of new operations, numbered per kind; the user can rename them. A profile cuts
# along a contour, a pocket clears the area inside it, a V-carve cuts with a V-shaped bit,
# a follow path runs the tool along the line itself, holes are drilled or bored, a
# surface faces the whole stock flat.
editor-name-profile = Contornatura { $number }
editor-name-pocket = Tasca { $number }
editor-name-vcarve = Incisione a V { $number }
editor-name-path = Segui percorso { $number }
editor-name-holes = Fori { $number }
editor-name-surface = Spianatura { $number }
# The name of a duplicated operation.
editor-name-copy = { $name } (copia)

# What an operation cuts, shown in the operation list. The stock is the material block.
editor-summary-whole-stock = tutto il grezzo
# Where the shapes come from: a DXF file, a traced picture or a text, by number.
editor-summary-dxf = DXF { $id }
editor-summary-image = immagine { $id }
editor-summary-text = testo { $id }
# A drawing layer (user's name); the source is one of the three texts above.
editor-summary-layer = layer { $layer }
editor-summary-layer-of = layer { $layer } di { $source }
editor-summary-all-of = tutto { $source }
# Some of the source's shapes were left out of the operation.
editor-summary-left-out = { $source } tranne { $count } esclusi
editor-summary-not-in-drawing = forme non nel disegno
editor-summary-no-geometry = nessuna geometria

# What an operation still lacks, under its field and in the list of problems. "Use
# selection" is the button that gives the operation the selected shapes.
editor-choose-side = scegli il lato della contornatura (interno, esterno o sulla linea)
editor-no-geometry = nessuna geometria: seleziona i contorni, poi Usa selezione
