# File > Import image… and the window that traces a picture into shapes
# (crates/chipload-gui/src/app/images.rs).

# The name of picture files in the file dialog's filter.
images-pictures-filter = Imágenes
images-cannot-read = No se puede leer { $path }: { $error }
# Followed by the error on its own line.
images-cannot-import = No se puede importar { $path }:
# Followed by the error on its own line.
images-trace-at-size-failed = No se pudo vectorizar la imagen a ese tamaño:
images-tracing-failed = falló la vectorización de la imagen
# After a picture is traced into the project. "V-carve (ribbon)" is the ribbon's V-carve button.
images-traced-next = La imagen está seleccionada: Tallado en V (cinta) la talla. Fija su tamaño y posición en Propiedades.
images-traced = Vectorizado { $file }
images-removed = Imagen { $id } quitada

# Window titles. $file: the picture's file name.
images-import-title = Importar imagen: { $file }
# "Trace" is a verb here: trace this picture again.
images-trace-title = Vectorizar { $file }
# The window's button, a verb.
images-import = Importar

# Under the preview. $zoom: such as "400%".
images-zoom-note = Zoom { $zoom } · arrastra para desplazar · doble clic para ajustar
images-scroll-to-zoom = Usa la rueda para ampliar

# The settings.
images-threshold = Umbral
images-threshold-hover = Los grises por debajo son negros (0 negro, 255 blanco). Auto elige el valor que mejor separa las partes oscuras y las claras.
# Checkbox: pick the threshold automatically.
images-threshold-auto = auto
images-invert = Invertir
images-invert-check = tallar las partes claras
images-blur = Desenfoque
images-blur-hover = Suaviza bordes irregulares y ruido antes del umbral.
images-smallest-piece = Mancha mínima
images-smallest-piece-hover = Se descartan las motas negras o blancas menores que esto.
images-smoothing = Suavizado
images-smoothing-hover = Cuánto puede enderezarse un borde vectorizado respecto al borde de la imagen.
# What tracing makes, a noun.
images-trace = Vectorizado
images-trace-hover = Contornos alrededor de las partes negras, para un tallado en V; o las líneas centrales de los trazos negros (letra manuscrita, dibujo lineal), para un seguir trayectoria.
images-trace-outlines = Contornos (tallado en V)
images-trace-centre-lines = Líneas centrales (seguir trayectoria)
images-arcs = Arcos
images-arcs-hover = Ajustar arcos además de líneas: programas más pequeños y movimiento más suave para contorneados, cajeras y seguir trayectoria.
images-arcs-check = ajustar arcos

# What the preview traced.
images-tracing = Vectorizando…
images-found-shapes = { $count ->
        [one] { $count } forma
       *[other] { $count } formas
    }
# $threshold: the grey value the automatic threshold picked, 1 to 254.
images-found-shapes-auto = { $count ->
        [one] { $count } forma
       *[other] { $count } formas
    } (umbral elegido: { $threshold })
images-found-centre-lines = { $count ->
        [one] { $count } línea central
       *[other] { $count } líneas centrales
    }
images-found-centre-lines-auto = { $count ->
        [one] { $count } línea central
       *[other] { $count } líneas centrales
    } (umbral elegido: { $threshold })
# The narrowest and widest strokes; $wide carries the unit.
images-stroke-widths = Trazos de { $narrow } a { $wide } de ancho
images-dots-left-out = { $count } puntos excluidos
# Traced pieces where no arcs were fitted.
images-kept-as-lines = { $count } quedan como líneas
images-specks-dropped = { $count } motas descartadas
images-black-at-border = El negro toca el borde: invierte si es el fondo.
