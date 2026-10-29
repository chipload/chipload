# The setup sheet for the operator (crates/chipload-pipeline/src/sheet.rs,
# docs/specs/setup-sheet.md): an HTML page printed next to the machine. A length argument
# brings its own unit ("6 mm", "0.25 in"); $start, $end, $x, $y, $width, $step, $radius and
# $feed are numbers alone, in the sheet's units, as are $z and $peck where said. Names
# (project, tools, parts, operations, layers, files) are never translated.

sheet-title = Hoja de preparación: { $project }
# Only some of the project's parts were exported.
sheet-title-parts = Hoja de preparación: { $project }, { $count ->
    [one] pieza
   *[other] piezas
} { $parts }
# The line under the title, its parts joined with " · ".
sheet-post = postprocesador { $post }
sheet-written-by = escrita por { $by }
# $length is "mm" or "in", $feed "mm/min" or "in/min".
sheet-units = longitudes en { $length }, avances en { $feed }
sheet-gcode-inch = Código G en pulgadas (G20)

## Stock and zero
sheet-stock-and-zero = Material en bruto y cero
sheet-stock = Material en bruto
# This text is HTML: keep &times; (the × sign). $width is a number alone.
sheet-stock-size = { $width } &times; { $length }, { $thickness } de espesor
sheet-copies = Copias
sheet-part = Pieza
sheet-dxf = DXF { $id }
# $placement says how the drawing was moved, turned and scaled.
sheet-dxf-placed = { $name } ({ $file }), { $placement }
sheet-image = Imagen { $id }
sheet-image-placed = { $file } en la capa { $layer }, { $width } de ancho en X{ $x } Y{ $y }
sheet-text = Texto { $id }
# $text is the text's first words in quotes, $font its font.
sheet-text-placed = { $text } con la fuente { $font } en la capa { $layer }, { $height } de alto en X{ $x } Y{ $y }

## Where X0 Y0 is, said on the stock
sheet-zero-centre = el centro del material en bruto
sheet-zero-bottom-left = la esquina inferior izquierda del material en bruto
sheet-zero-bottom-right = la esquina inferior derecha del material en bruto
sheet-zero-top-left = la esquina superior izquierda del material en bruto
sheet-zero-top-right = la esquina superior derecha del material en bruto
# A zero away from the stock's bottom left corner: "25 mm right of and 10 mm above the
# stock's bottom left corner". $offset is one or two of the four below, joined with -and.
sheet-zero-from-corner = { $offset } de la esquina inferior izquierda del material en bruto
sheet-zero-right-of = { $distance } a la derecha
sheet-zero-left-of = { $distance } a la izquierda
sheet-zero-above = { $distance } por encima
sheet-zero-below = { $distance } por debajo
sheet-zero-and = { $first } y { $second }
sheet-zero-outside = { $zero } (fuera del material en bruto)
# Where Z0 is.
sheet-z-top = la cara superior del material en bruto
sheet-z-bottom = la cara inferior del material en bruto (el tablero de sacrificio)

## A file's tools. $what is the tool: "6 mm end mill, 2 flutes".
# $xy is where X0 Y0 is, $z where Z0 is (texts above).
sheet-load-tool = Carga la herramienta T{ $tool } ({ $what }). Fija X0 Y0 en { $xy } y Z0 en { $z }.
sheet-tool-change-m6 = Cambio de herramienta (M6) a T{ $tool } ({ $what }), según lo gestione tu controlador.
sheet-change-tool = Cambia a la herramienta T{ $tool } ({ $what }). Vuelve a fijar Z0 con esta herramienta en { $z }; mantén X0 Y0.
# These two texts are HTML: keep &ldquo; and &rdquo; (the quotes round the tool's name).
# $rpm is the spindle speed, $spindle one of the two -spindle- texts.
sheet-tool-named = Herramienta T{ $tool }: &ldquo;{ $name }&rdquo;, { $what }, { $rpm } rpm en sentido { $spindle }.
sheet-tool = Herramienta T{ $tool }: { $what }, { $rpm } rpm en sentido { $spindle }.
sheet-spindle-cw = horario (M3)
sheet-spindle-ccw = antihorario (M4)
# The parts of $what, joined with ", ".
sheet-end-mill = fresa de { $diameter }
sheet-drill = broca de { $diameter }
# $angle in degrees.
sheet-vbit-angle = fresa en V de { $angle }°, { $diameter }
sheet-vbit = fresa en V, { $diameter }
sheet-tip = punta { $diameter }
sheet-flutes = { $count ->
    [one] { $count } filo
   *[other] { $count } filos
}

## The table of operations
sheet-column-operation = Operación
sheet-column-depth = Profundidad
sheet-column-feeds = Avance / penetración
sheet-column-entry = Entrada
sheet-column-tabs = Puentes
sheet-column-time = Tiempo
# A part's row over its operations, numbered from 1.
sheet-part-row = Pieza { $number }: { $name }
sheet-part-row-copies = Pieza { $number }: { $name }, { $count ->
    [one] { $count } copia
   *[other] { $count } copias
}
# An operation without a name.
sheet-operation = Operación { $number }
# A drill's one feed: $feed is a number alone.
sheet-feed-down = { $feed } bajando
# The kind column, its parts joined with ", ": "Profile, outside, climb, leaves 0.2 mm".
sheet-kind-profile = Contorneado
sheet-side-outside = exterior
sheet-side-inside = interior
sheet-side-on = sobre la línea
sheet-kind-pocket = Cajera, anillos
sheet-climb = en concordancia
sheet-conventional = en oposición
# $percent: the stepover in percent of the tool's diameter.
sheet-stepover = paso lateral { $percent } %
sheet-kind-vcarve = Tallado en V
sheet-kind-path = Seguir trayectoria, sobre la línea
sheet-kind-holes-drilled = Taladros, taladrados
sheet-kind-holes-bored = Taladros, taladrado helicoidal
sheet-kind-surface = Planeado, todo el material en bruto, en vaivén (en concordancia y en oposición), paso lateral { $percent } %
# $surface is the text above; $distance half the tool's diameter.
sheet-kind-surface-past = { $surface }; sobrepasa { $distance } los extremos del material en bruto: mantén las sujeciones fuera del recorrido
sheet-cuts-past = corta { $distance } más allá de la línea
sheet-leaves = deja { $distance }
sheet-rest-after = restos tras T{ $tool }
sheet-rest-after-size = restos tras T{ $tool } ({ $diameter })
# $degrees: the sharpest corner angle that gets one.
sheet-dogbones = dogbones en esquinas de hasta { $degrees }°
sheet-tbones = T-bones en esquinas de hasta { $degrees }°
sheet-corner-choices = { $count ->
    [one] { $count } esquina elegida aparte
   *[other] { $count } esquinas elegidas una a una
}
# The depth column, its parts joined with ", ": "0 to 6, 2 per pass".
sheet-depths = de { $start } a { $end }
sheet-depths-drill = de { $start } a { $end } (la punta de la broca)
sheet-pecks = picoteo { $peck }
sheet-one-go = de una vez
sheet-per-turn = { $step } por vuelta
sheet-per-pass = { $step } por pasada
sheet-one-pass = una pasada
sheet-finish = acabado { $stock }
sheet-skin = piel { $thickness }
# The entry column.
sheet-entry-beside = baja junto al material en bruto
sheet-entry-straight = recta hacia abajo
sheet-entry-plunge = penetración
sheet-entry-ramp = rampa { $degrees }°
sheet-entry-helix = hélice { $degrees }°
sheet-entry-helix-radius = hélice r { $radius }, { $degrees }°
sheet-lead = aproximación r { $radius }
# The tabs column: $count tabs, $width and $height numbers alone.
sheet-tabs = { $count } × { $width } de ancho, { $height } de alto
sheet-flanks = flancos { $degrees }°
sheet-tabs-none = ninguno
# Under the table: how deep the tool goes. $z is a number alone; the parts are joined with ", ".
sheet-cuts-down = Corta hasta Z{ $z }: { $depth } por debajo de la cara superior del material en bruto
sheet-into-spoilboard = { $depth } dentro del tablero de sacrificio
sheet-through = atraviesa el material en bruto
sheet-sticks-out = { $cut }. Comprueba que la herramienta sobresale lo suficiente.

## Watch out: the notes and the checker's findings
sheet-watch-out = Atención
sheet-nothing-reported = No se ha señalado nada.
# $note is the post-processor's own note, as its file writes it.
sheet-post-note = Postprocesador { $post }: { $note }
sheet-check = Comprobación: { $finding }

## The picture's legend
sheet-legend-file = Archivo { $number } (T{ $tool })
sheet-legend-tabs = puentes
sheet-legend-grey = Gris: las piezas. Solo movimientos de corte, sin rápidos.

## The time
sheet-time = Tiempo
