# The operations being edited: names given to new operations, and the short description
# of what an operation cuts (crates/chipload-gui/src/editor.rs).

# The name of a duplicated operation.
editor-name-copy = { $name } (copia)

# What an operation cuts, shown in the operation list. The stock is the material block.
editor-summary-whole-stock = todo el material en bruto
# Where the shapes come from: a DXF file, a traced picture or a text, by number.
editor-summary-dxf = dibujo { $id }
editor-summary-image = imagen { $id }
editor-summary-text = texto { $id }
editor-summary-group = grupo { $id }
# A drawing layer (user's name); the source is one of the three texts above.
editor-summary-layer = capa { $layer }
editor-summary-layer-of = capa { $layer } de { $source }
editor-summary-all-of = todo { $source }
# Some of the source's shapes were left out of the operation.
editor-summary-left-out = { $source } salvo { $count } excluidos
editor-summary-not-in-drawing = formas que no están en el dibujo
editor-summary-no-geometry = sin geometría

# What an operation still lacks, under its field and in the list of problems. "Use
# selection" is the button that gives the operation the selected shapes.
editor-choose-side = elige el lado del contorneado (interior, exterior o sobre la línea)
editor-no-geometry = sin geometría: selecciona contornos y luego Usar selección
