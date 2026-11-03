# The operations being edited: names given to new operations, and the short description
# of what an operation cuts (crates/chipload-gui/src/editor.rs).

# The name of a duplicated operation.
editor-name-copy = { $name } (kopia)

# What an operation cuts, shown in the operation list. The stock is the material block.
editor-summary-whole-stock = hela ämnet
# Where the shapes come from: a DXF file, a traced picture or a text, by number.
editor-summary-dxf = ritning { $id }
editor-summary-image = bild { $id }
editor-summary-text = text { $id }
editor-summary-group = grupp { $id }
# A drawing layer (user's name); the source is one of the three texts above.
editor-summary-layer = lager { $layer }
editor-summary-layer-of = lager { $layer } i { $source }
editor-summary-all-of = hela { $source }
# Some of the source's shapes were left out of the operation.
editor-summary-left-out = { $source } utom { $count } utelämnade
editor-summary-not-in-drawing = former som inte finns i ritningen
editor-summary-no-geometry = ingen geometri

# What an operation still lacks, under its field and in the list of problems. "Use
# selection" is the button that gives the operation the selected shapes.
editor-choose-side = välj konturfräsningens sida (inuti, utanför eller på linjen)
editor-no-geometry = ingen geometri: markera konturer och välj sedan Använd markering
