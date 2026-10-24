# The Browser's Drawing folder: the project's DXFs with their layers, its pictures, texts
# and drawn shapes (crates/chipload-gui/src/app/sources.rs).

# The folder's name: everything that is drawn, a noun.
sources-drawing = Dibujo
sources-drawing-hover = DXF, imágenes, textos y formas dibujadas; clic derecho para añadir uno
sources-add-dxf = Añadir DXF…
sources-add-dxf-hover = Otra pieza: un DXF colocado y actualizado por separado
# "File > Import DXF" names a menu item.
sources-add-dxf-disabled-hover = Archivo > Importar DXF empieza un proyecto a partir de uno
sources-import-image = Importar imagen…
sources-import-image-hover = Vectorizar una imagen en formas, para tallado en V
sources-add-text = Añadir texto
sources-add-text-hover = Escribe un texto y luego haz clic donde empieza
sources-no-shapes = Aún no hay formas.
# The name of DXF files in the file dialog's filter.
sources-dxf-filter = Dibujos DXF

# Rows of the folder. $file: the picture's file name; $text: what the text says.
sources-image-title = { $file } (imagen { $id })
sources-text-title = { $text } (texto { $id })
# Rows for the shapes drawn in the app, by kind.
sources-rectangles = Rectángulos
sources-circles = Círculos
sources-polylines = Polilíneas
sources-other-paths = Otros trayectos

sources-dxf-hover = Un clic selecciona todo el DXF: una operación creada a partir de él sigue sus actualizaciones. Su posición y escala se muestran en Propiedades; clic derecho para más
sources-image-hover = Un clic selecciona toda la imagen: una operación creada a partir de ella sigue su revectorizado. Su tamaño y posición se muestran en Propiedades; doble clic para sus ajustes de vectorizado
sources-text-hover = Un clic selecciona todo el texto: una operación creada a partir de él sigue sus cambios. Lo que dice, su fuente, tamaño y posición están en Propiedades
sources-drawn-hover = Un clic selecciona estas formas; Ctrl+clic las añade
sources-layer-hover = Un clic selecciona esta capa de este DXF: una operación creada a partir de ella solo corta esta capa de este DXF

# Right-click menu items, verbs.
sources-select = Seleccionar
sources-show = Mostrar
sources-hide = Ocultar
sources-place = Colocar…
sources-place-hover = Mover, girar o escalar este DXF, en sus propiedades
sources-update-from-dxf = Actualizar desde un DXF…
sources-update-from-path = Actualizar desde una ruta…
sources-remove-dxf-hover = Quitar este DXF y sus formas
sources-trace-settings = Ajustes de vectorizado…
sources-trace-settings-hover = Umbral, suavizado, contornos o líneas centrales; su tamaño y posición están en sus propiedades
sources-remove-image-hover = Quitar la imagen y sus formas
sources-remove-text-hover = Quitar el texto y sus letras

# Followed by the project's problems, one per line.
sources-project-incomplete = El proyecto no está completo:
# Followed by the error on its own line.
sources-remove-dxf-failed = No se pudo quitar el DXF:
