# The operations being edited: names given to new operations, and the short description
# of what an operation cuts (crates/chipload-gui/src/editor.rs).

# The name of a duplicated operation.
editor-name-copy = { $name } (Kopie)

# What an operation cuts, shown in the operation list. The stock is the material block.
editor-summary-whole-stock = das ganze Rohteil
# Where the shapes come from: a DXF file, a traced picture or a text, by number.
editor-summary-dxf = Zeichnung { $id }
editor-summary-image = Bild { $id }
editor-summary-text = Text { $id }
editor-summary-group = Gruppe { $id }
# A drawing layer (user's name); the source is one of the three texts above.
editor-summary-layer = Layer { $layer }
editor-summary-layer-of = Layer { $layer } von { $source }
editor-summary-all-of = alles von { $source }
# Some of the source's shapes were left out of the operation.
editor-summary-left-out = { $source }, aber { $count } ausgelassen
editor-summary-not-in-drawing = Formen nicht in der Zeichnung
editor-summary-no-geometry = keine Geometrie

# What an operation still lacks, under its field and in the list of problems. "Use
# selection" is the button that gives the operation the selected shapes.
editor-choose-side = Seite der Kontur wählen (innen, außen oder auf der Linie)
editor-no-geometry = keine Geometrie: Konturen auswählen, dann Auswahl verwenden
