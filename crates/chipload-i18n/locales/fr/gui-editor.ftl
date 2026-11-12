# The operations being edited: names given to new operations, and the short description
# of what an operation cuts (crates/chipload-gui/src/editor.rs).

# The name of a duplicated operation.
editor-name-copy = { $name } (copie)

# What an operation cuts, shown in the operation list. The stock is the material block.
editor-summary-whole-stock = tout le brut
# Where the shapes come from: a DXF file, a traced picture or a text, by number.
editor-summary-dxf = dessin { $id }
editor-summary-image = image { $id }
editor-summary-model = modèle { $id }
editor-summary-text = texte { $id }
editor-summary-group = groupe { $id }
# A drawing layer (user's name); the source is one of the three texts above.
editor-summary-layer = calque { $layer }
editor-summary-layer-of = calque { $layer } de { $source }
editor-summary-all-of = { $source } (tout)
# Some of the source's shapes were left out of the operation.
editor-summary-left-out = { $source }, { $count } exclus
editor-summary-not-in-drawing = formes absentes du dessin
editor-summary-no-geometry = aucune géométrie

# What an operation still lacks, under its field and in the list of problems. "Use
# selection" is the button that gives the operation the selected shapes.
editor-choose-side = choisissez le côté du contour (intérieur, extérieur ou sur la ligne)
editor-no-geometry = aucune géométrie : sélectionnez des contours, puis Prendre la sélection
