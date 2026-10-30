# The property forms: an operation, the job settings, tools, DXFs, pictures, texts, parts (crates/chipload-gui/src/app/forms.rs).

# Depths, shared by the operations. "Through": a button that sets the end depth through the stock.
forms-start-depth = Prof. inicial
forms-start-depth-hover = Dónde empieza el corte, medido hacia abajo desde la cara superior del material
forms-end-depth = Prof. final
forms-end-depth-hover = Dónde termina el corte, medido hacia abajo desde la cara superior del material
forms-through = Pasante
forms-through-hover = Cortar atravesando el material: la profundidad final es su espesor más la profundidad por debajo, y sigue al espesor cuando cambia
forms-through-below = Bajo el material
forms-through-below-hover = Cuánto baja el corte por debajo de la cara inferior del material, en el tablero de sacrificio. Ajustes del trabajo > Máquina fija el máximo ({ $depth })
forms-oversize = Holgura
forms-oversize-disabled = Un contorneado sobre la línea no tiene lado en el que cortar más
forms-step-down = Prof. de pasada
# Indented under "Passes": the depth of each pass.
forms-step-down-sub = prof. de pasada

# Cutting direction (milling).
forms-direction = Sentido
forms-climb = en concordancia
forms-conventional = en oposición

# Tabs' sloped sides.
forms-flank-angle = ángulo del flanco
forms-flank-angle-hover = Cuánta pendiente tienen los lados del puente desde su cara superior plana. 90° es un puente recto: la herramienta sube y baja en vertical. Por debajo de eso la herramienta sigue la pendiente, así que nunca penetra en vertical tras un puente. Bajar por un flanco es una rampa: con una herramienta que no corta al centro, no más inclinada que el ángulo de rampa.
forms-flank-straight = recto
forms-flank-feet = base { $length } a cada lado

# How the tool goes down into the material.
forms-entry = Entrada
forms-entry-hover = Cómo baja la herramienta al material. Una rampa baja a lo largo de la trayectoria (un contorneado baja en espiral por su anillo, un seguir trayectoria baja yendo y viniendo, una cajera sale en rampa y vuelve), una hélice (cajeras) alrededor de un pequeño círculo, así que la herramienta no necesita cortar al centro; ambas usan el avance de corte.
forms-entry-plunge = penetración recta
forms-entry-ramp = rampa
forms-entry-helix = hélice
forms-entry-ramp-summary = rampa { $angle }
forms-entry-helix-summary = hélice r { $radius }, { $angle }
forms-entry-with-lead = { $entry }, aproximación { $radius }
forms-helix-radius = radio de hélice
forms-helix-radius-hover = Del círculo que sigue el centro de la herramienta: menor que el radio de la herramienta, para que no quede un núcleo en pie
forms-ramp-angle-sub = ángulo de rampa
forms-ramp-angle = Ángulo de rampa
forms-ramp-angle-hover = La herramienta baja en espiral a lo largo del contorno, nunca con más pendiente que esta y nunca más de una profundidad de pasada por vuelta: sin penetración vertical en la pieza. Cada pasada baja con este ángulo y luego sigue a nivel; un ángulo mayor acorta la rampa.

# The operation kinds, in the form's title.
forms-kind-profile = Contorneado
forms-kind-pocket = Cajera
forms-kind-vcarve = Tallado en V
forms-kind-path = Seguir trayectoria
forms-kind-holes = Taladros
forms-kind-surface = Planeado
forms-operation-number = Operación { $number }

forms-name = Nombre
forms-geometry = Geometría
forms-whole-stock = Todo el material en bruto
forms-whole-stock-hover = Un planeado cubre todo el material en bruto, una sola vez, haya las copias que haya
forms-use-selection = Usar selección
forms-use-selection-count = Usar selección ({ $count })
forms-use-selection-hover = Sustituir por los contornos seleccionados en el lienzo.
forms-use-selection-disabled = Selecciona contornos en el lienzo primero

# A profile's side of the line.
forms-side = Lado
forms-side-inside = interior
forms-side-outside = exterior
forms-side-on = sobre la línea
forms-side-choose = — elegir —

# Section titles and their folded summaries. "Cut": how the operation cuts (noun).
forms-section-cut = Corte
forms-profile-cut-summary = { $start }–{ $end } { $unit }, pasadas de { $step } { $unit }, { $direction }, rampa en espiral { $ramp }°
forms-pocket-cut-summary = { $start }–{ $end } { $unit }, pasadas de { $step } { $unit }, { $direction }
forms-path-cut-summary = { $start }–{ $end } { $unit }, pasadas de { $step } { $unit }, sobre la línea
forms-surface-cut-summary = { $start }–{ $end } { $unit }, pasadas de { $step } { $unit }, paso lateral { $stepover } × D
forms-vcarve-cut-one-pass = desde { $start }, como máximo { $end } de profundidad, una pasada
forms-vcarve-cut-steps = desde { $start }, como máximo { $end } de profundidad, pasadas de { $step }

# A pocket's clearing, in rings.
forms-section-clearing = Vaciado
forms-clearing-summary = anillos, paso lateral { $stepover } × D
forms-clearing-summary-rest = anillos, paso lateral { $stepover } × D, restos tras T{ $tool }
forms-stepover = Paso lateral
forms-stepover-hover = Cuánto avanza cada anillo, como fracción del diámetro de la herramienta
forms-surface-stepover-hover = Cuánto se desplaza cada pasada, como fracción del diámetro de la herramienta
forms-surface-end-depth-hover = Cuánto se rebaja de la parte superior, medido hacia abajo desde la cara superior del material
forms-section-entry-lead = Entrada y aproximación
forms-section-entry = Entrada

# V-carving.
forms-vcarve-start-depth-hover = La superficie tallada, medida hacia abajo desde la cara superior del material (0: la cara superior; más para tallar en el fondo de una cajera)
forms-vcarve-end-depth-hover = Lo más hondo que llega la punta, desde la cara superior del material (end_depth en el archivo). Los trazos más anchos se tallan hasta ahí y se informa de ellos
forms-passes = Pasadas
forms-in-steps = en pasadas

# The operation's tool. "T3" is tool number 3.
forms-tool = Herramienta
forms-tool-not-in-project = T{ $tool } (no está en el proyecto)
forms-from-library = De tu biblioteca:
forms-use-cutting-values = Usar los valores de corte de la herramienta
forms-use-cutting-values-hover = De la biblioteca: avance { $feed } / penetración { $plunge } { $feed_unit }, prof. de pasada { $step_down }, paso lateral { $stepover }

forms-section-feeds = Avances
forms-feed = Avance
forms-chip-load = Avance por diente
forms-chip-load-no-flutes = filos sin fijar
forms-chip-load-no-rpm = velocidad del husillo sin fijar
forms-chip-load-missing-hover = Fíjalos en las propiedades de la herramienta: Herramientas en el navegador, luego la herramienta.
forms-flutes = Filos
forms-flutes-hover = Los filos de la herramienta, para el avance por diente. Sin marcar: desconocido.
forms-plunge-feed = Avance de penetración

# Rest machining: clear only what a larger tool left.
forms-rest-after = Restos tras
forms-rest-after-hover = Quitar solo lo que deja una herramienta mayor en estas formas: las esquinas que deja redondeadas y las ranuras en las que no cabe. Corta primero la cajera con esa herramienta
forms-rest-none = ninguna
forms-rest-endmill = fresa de { $diameter }

# Square inside corners.
forms-corners = Esquinas
forms-corners-hover = Esquinas interiores que una herramienta redonda deja redondeadas: un dogbone entra en la esquina por su bisectriz, un T-bone a lo largo de la pared más larga (la muesca queda en la más corta), para que una pieza cuadrada encaje hasta el fondo
forms-corners-round = redondeadas
forms-corners-dogbones = dogbones
forms-corners-tbones = T-bones
forms-corners-disabled = Un contorneado sobre la línea no tiene esquinas interiores
forms-corners-up-to = hasta
forms-corners-up-to-hover = Solo las esquinas así de agudas o más (las esquinas de 90° dibujadas con una ligera desviación también cuentan)
forms-corners-chosen = { $count ->
    [one] { $count } esquina elegida aparte
   *[other] { $count } esquinas elegidas una a una
}
forms-corners-chosen-hover = Esquinas pulsadas en el lienzo y elegidas aparte, diga lo que diga la fila de arriba
forms-corners-reset = Restablecer
forms-corners-reset-hover = Cada esquina vuelve a tener lo que dice la fila de arriba

# Holes: drilled, or bored with an endmill round a helix.
forms-holes-drilled-pecks = taladrados, picoteo { $peck }
forms-holes-drilled-once = taladrados de una vez
forms-holes-bore = taladrado helicoidal, { $step } por vuelta, { $direction }
forms-holes-cut-summary = { $start }–{ $end } { $unit }, { $method }
forms-holes-drill-end-depth-hover = Dónde termina la punta de la broca, medido hacia abajo desde la cara superior del material: el diámetro completo se detiene unas 0.3 × el diámetro de la broca más arriba
forms-pecks = Picoteo
forms-pecks-hover = Taladrar esta profundidad cada vez, saliendo del agujero entre medias para evacuar la viruta
forms-in-pecks = con picoteo
forms-peck = picoteo
forms-step-per-turn = Bajada por vuelta
forms-step-per-turn-hover = Lo máximo que baja la hélice en una vuelta
forms-bore-ramp-angle-hover = La hélice nunca tiene más pendiente que esto
forms-drill-feeds-summary = { $feed } { $unit } bajando
forms-drill-feed = Avance de taladrado

# How a profile holds its part.
forms-holding-none = ninguna
forms-holding-tabs = puentes
forms-holding-skin = piel de cebolla
forms-holding-skin-tabs = piel de cebolla y puentes
forms-holding-summary-tabs = { $holding }: { $count } × { $width } { $unit }, { $height } { $unit } de alto
forms-holding-summary-skin = { $summary }, piel { $thickness }
forms-section-holding = Sujeción
forms-hold-the-part = Sujetar la pieza
forms-hold-the-part-hover =
    Cómo se mantiene en su sitio una pieza recortada.
    Puentes: material que se deja en el fondo del corte; la pieza queda unida hasta que la liberas.
    Piel de cebolla: una capa fina que se deja todo alrededor durante el desbaste, y luego una última vuelta, tras todos los anillos de la operación, la atraviesa. La pieza queda suelta durante esa vuelta: añade puentes si las piezas no deben moverse entonces.
forms-tab-count = n.º de puentes
forms-tab-width = ancho
forms-tab-height = alto
forms-tabs-placed = { $count ->
    [one] { $count } puente colocado a mano
   *[other] { $count } puentes colocados a mano
}
forms-tabs-placed-hover = Puentes arrastrados, añadidos o quitados en el lienzo; los contornos sin ninguno reciben el número de arriba, repartidos por igual
forms-tabs-reset = Restablecer
forms-tabs-reset-hover = Los puentes de cada contorno vuelven a repartirse por igual
forms-skin-thickness = espesor de piel
forms-skin-thickness-hover = Se deja por encima de la profundidad final hasta la última vuelta: como máximo una profundidad de pasada, y más fina que los puentes

# A profile's finishing pass.
forms-finish-none = ninguna
forms-finish-summary = { $stock } de sobrematerial, luego una vuelta
forms-finishing-pass = Pasada de acabado
forms-finishing-pass-hover = Desbastar dejando esto en la pared, y quitarlo después en una vuelta ligera a profundidad completa: una pared más limpia, sin marcas de pasadas. La vuelta baja en la ranura del desbaste, nunca sobre la pared acabada.
forms-finishing-lap = vuelta de acabado
forms-finishing-disabled = Un contorneado sobre la línea no tiene pared que acabar
# The material left on the wall for the finishing lap.
forms-finish-stock = sobrematerial

# A pocket's lead-in and lead-out.
forms-lead = Aproximación
forms-lead-hover = Entrar a la pared en arco desde la cajera ya vaciada y salir de ella igual, para que la herramienta nunca se detenga contra la pared. Una aproximación sin espacio se reduce o se omite (el informe lo indica).
forms-lead-in-out = aproximación y salida
forms-lead-radius = radio

# The G-code's X0 Y0.
forms-work-zero = Cero pieza X Y
forms-work-zero-hover = El punto que pasa a ser X0 Y0 en el código G. La geometría no se mueve.
forms-zero-drawing-origin = origen del dibujo
forms-zero-stock-centre = centro del material
forms-zero-stock-bottom-left = material, abajo a la izquierda
forms-zero-stock-bottom-right = material, abajo a la derecha
forms-zero-stock-top-left = material, arriba a la izquierda
forms-zero-stock-top-right = material, arriba a la derecha
forms-zero-point = punto
forms-zero-on-stock = En el material
forms-zero-on-stock-hover = Haga clic en una esquina o en el centro: el cero se queda ahí cuando cambia el tamaño del material
forms-zero-drawing-origin-button = Origen del dibujo
forms-zero-drawing-origin-hover = X0 Y0 donde el dibujo tiene su propio 0,0
forms-zero-at = Posición
forms-zero-at-hover = Dónde está el cero, en coordenadas del dibujo. Escribir un valor lo convierte en un punto propio.
forms-zero-note = El X0 Y0 Z0 del código G. Moverlo no mueve ningún corte, solo los números del programa. En la vista, selecciónelo y arrástrelo: se ajusta a las esquinas y al centro del material y al origen del dibujo.
forms-zero-link-hover = Mostrar las propiedades del cero de pieza

# An imported DXF.
forms-file = Archivo
forms-units = Unidades
forms-dxf-inch = pulgadas (convertidas a mm)
forms-shapes = Formas
forms-section-placement = Posición, rotación y escala
forms-update-dxf = Actualizar desde un DXF o SVG…
forms-update-dxf-hover = Una nueva versión del dibujo: las operaciones conservan sus capas
# A button (verb).
forms-remove = Quitar
forms-remove-dxf-hover = Quitar este archivo de dibujo y sus formas

# A picture traced into shapes.
forms-picture = Imagen
forms-picture-pixels = { $width } × { $height } píxeles
forms-traced-as = Vectorizada como
forms-traced-outlines = contornos
forms-traced-centre-lines = líneas centrales
forms-position = Posición
forms-picture-reset-position = La esquina del recuadro en X0 Y0
forms-rotation = Rotación
forms-picture-rotation-hover = Grados en sentido antihorario, cualquier ángulo, alrededor del centro de la imagen. La imagen se vectoriza tal cual y luego se giran sus formas.
forms-turn-ccw-hover = 90° en sentido antihorario
forms-turn-cw-hover = 90° en sentido horario
forms-picture-reset-rotation = Como está la imagen (0°)
forms-width = Ancho
forms-picture-width-hover = El ancho de la imagen antes de girarla; el alto se ajusta solo
forms-picture-height = alto { $height }
forms-layer = Capa
forms-picture-layer-hover = La capa en la que están sus formas, para las operaciones que la seleccionan
forms-trace-settings = Ajustes de vectorizado…
forms-trace-settings-hover = Umbral, suavizado, contornos o líneas centrales, con vista previa
forms-remove-picture-hover = Quitar la imagen y sus formas

# A text written in a built-in font.
forms-text = Texto
forms-text-hover = Lo que dice; Enter empieza una línea nueva
forms-font = Fuente
forms-font-hover = Una fuente incluida: normal, manuscrita o decorativa (Guía del usuario: Texto)
forms-height = Altura
forms-text-height-hover = La altura de una letra mayúscula
forms-anchor = Anclaje
forms-anchor-hover = Qué es la Posición: la esquina inferior izquierda del recuadro del texto, o su centro. Un texto centrado sigue centrado cuando cambian sus palabras, la fuente o el tamaño
forms-anchor-corner = esquina inferior izquierda
forms-anchor-centre = centro
forms-text-reset-position = En X0 Y0
forms-text-rotation-hover = Grados en sentido antihorario, cualquier ángulo, alrededor del centro del texto
forms-text-reset-rotation = Horizontal (0°)
forms-align = Alinear
forms-align-hover = Cómo se alinean las líneas entre sí
forms-align-left = izquierda
forms-align-centre = centro
forms-align-right = derecha
forms-letter-spacing = Espaciado de letras
forms-letter-spacing-hover = Espacio extra entre letras; 0: el propio de la fuente
forms-line-spacing = Interlineado
forms-line-spacing-hover = De la base de una línea a la siguiente, en alturas de mayúscula
# After the number: "1.2 × height".
forms-line-spacing-suffix = × altura
forms-text-layer-hover = La capa en la que están sus letras, para las operaciones que la seleccionan
forms-size = Tamaño
forms-remove-text-hover = Quitar el texto y sus letras

# The job settings. "Stock": the material the parts are cut from.
forms-job-settings = Ajustes del trabajo
forms-section-stock = Material en bruto
forms-stock-size = Tamaño
forms-thickness = Espesor
forms-material = Material
forms-material-hover = Tus propios avances por herramienta, de Este equipo > Materiales. Elegirlo da a cada operación, y a las que añadas después, este material y sus valores. El material en bruto queda como está
forms-material-none = Ninguno
forms-material-not-in-library = { $name } (no está en tu biblioteca)
forms-material-from = De { $name }
forms-material-from-hover = Los avances, la profundidad de pasada y el paso lateral son los valores del material para esta herramienta
forms-material-changed = Cambiado respecto a { $name }
forms-material-use = Valores de { $name }
forms-material-use-hover = Cortar con los valores del material para esta herramienta
forms-material-save = Guardar en { $name }
forms-material-save-hover = Guardar estos avances, profundidad de pasada y paso lateral como valores del material para esta herramienta, para los próximos trabajos
forms-material-not-in = { $name } no tiene valores para esta herramienta
forms-material-no-values = { $name } (sin valores para esta herramienta)
forms-op-material-hover = Tus propios avances por herramienta, de Este equipo > Materiales: la operación toma los valores de este material para su herramienta, y los sigue cuando cambias la herramienta
forms-max-overcut = Prof. máx. bajo el material
forms-max-overcut-hover = Cuánto puede bajar un corte por debajo del material (en el tablero de sacrificio). Los cortes dentro de ese margen generan una advertencia; los más profundos se rechazan.
forms-stock-position = Posición del material
forms-stock-corner-at-origin = esquina en el origen del dibujo
forms-stock-centred-on-origin = centrado en el origen del dibujo
forms-stock-centred-on-drawing = centrado en el dibujo
forms-stock-margin = margen
forms-stock-fits-hover = El contorno del dibujo más el margen a cada lado, centrado en el dibujo; sigue al dibujo
forms-stock-fit = Contorno + margen
forms-stock-fit-hover = Centrar el material en el contorno del dibujo y darle el tamaño del contorno más un margen, siguiéndolo
forms-stock-fixed-size = Tamaño fijo, centrado
forms-stock-fixed-size-hover = Un tamaño que escribes, para un tablero que quieres usar, centrado en el contorno del dibujo
forms-drawing-size-note = Dibujo { $size }
forms-zero-summary = X0 Y0 { $xy } · Z0 { $z }
forms-material-each-op = Cada operación puede elegir el suyo, bajo su herramienta.
forms-drawing-size-hover = El tamaño del contorno del dibujo (lo que se corta; sin la geometría auxiliar). El contorno del corte, con la herramienta, aparece en el informe tras Calcular
forms-z-zero = Cero Z
forms-z-zero-top = cara superior del material
forms-z-zero-bottom = cara inferior del material
forms-machine-summary = { $post }, Z segura { $safe_z }, { $spindle }
forms-section-machine = Máquina
forms-from-machine = De la máquina
forms-edit-here = Editar aquí
forms-edit-here-hover = Cambiar los valores de la máquina solo para este proyecto; sigue vinculada y muestra lo que difiere
forms-safe-z-job = Z segura (este trabajo)
forms-safe-z-hover = La altura para los movimientos rápidos laterales, por encima del material y de sus bridas

# The project's tools.
forms-tools = Herramientas
forms-add-from-library = Añadir de la biblioteca…
forms-new-tool = Herramienta nueva
forms-tool-not-used = sin usar
# $operations: a list of operation numbers, "1, 3".
forms-tool-used-by = operaciones { $operations }
forms-tool-not-used-long = No la usa ninguna operación
forms-tool-cuts = Corta las operaciones { $operations }
forms-remove-tool = Quitar herramienta
forms-remove-tool-disabled = La usan las operaciones { $operations }; primero asígnales otra herramienta
# An icon button that puts a value back.
forms-reset = Restablecer

# A DXF's position, rotation and scale.
forms-dxf-reset-position = Volver a donde lo tiene el archivo (giro y escala se mantienen)
forms-bottom-left = Abajo izquierda
forms-bottom-left-hover = La esquina del dibujo a { $margin } de la esquina inferior izquierda del material
# A button: centre the drawing on the stock.
forms-centre = Centrar
forms-centre-hover = El centro del dibujo sobre el centro del material
forms-dxf-rotation-hover = Grados en sentido antihorario desde el archivo tal como se dibujó, cualquier ángulo, alrededor del centro del dibujo
forms-dxf-reset-rotation = Volver al ángulo propio del archivo (0°)
forms-scale = Escala
forms-scale-hover = Desde el archivo tal como se dibujó, alrededor del centro del dibujo. Los tamaños de herramienta, las profundidades y los puentes no se escalan.
forms-dxf-reset-scale = Volver al tamaño propio del archivo (× 1)

# A part: a group of operations cut one after the other.
forms-part-number = Pieza { $number }
# Label of the on/off checkbox (verb): whether the part is cut.
forms-part-cut = Cortar
forms-part-cut-hover = Desactivado: la pieza y sus operaciones siguen en el proyecto, sin cortarse
forms-part-cut-check = esta pieza y sus operaciones
forms-active = Activa
forms-active-hover = Las operaciones nuevas van a la pieza activa. Haz clic derecho en una pieza para activarla
forms-active-yes = sí: aquí van las operaciones nuevas
forms-activate = Activar
forms-operations = Operaciones
forms-operations-none = ninguna todavía
forms-copies-one = una
forms-copies = Copias
forms-parts-note = Las piezas se cortan una tras otra, cada una con sus copias.
forms-copies-hover = Cortar la pieza varias veces en una cuadrícula. Las copias se dibujan atenuadas; solo se puede seleccionar el dibujo original.
forms-copies-grid = varias, en cuadrícula
forms-columns-rows = columnas × filas
# Copy to copy distance, in X and Y.
forms-pitch = paso X Y
forms-gap = separación
forms-gap-hover = Fija el paso una vez para que quede esta distancia entre los cortes de copias vecinas. El proyecto guarda el paso.
forms-set-pitch = Fijar paso
forms-order = orden

# The machine. "Post": the post-processor that writes the G-code.
forms-post = Postprocesador
forms-post-yours = { $name } (tuyo)
forms-post-folder-hover = Tus propios postprocesadores van en { $folder }
forms-post-builtin-hover = Los postprocesadores incluidos
forms-gcode-units = Unidades del código G
forms-gcode-units-hover = Las unidades en que se escribe el código G: mm (G21) o pulgadas (G20), según esté configurado tu controlador. Todo lo demás del proyecto sigue en mm
forms-gcode-mm = mm (G21)
forms-gcode-inch = pulgadas (G20)
forms-spindle = Husillo
forms-spindle-cw = horario (M3)
forms-spindle-ccw = antihorario (M4)
forms-max-feed = Avance máx.
forms-tool-changes = Cambios de herramienta
forms-tool-changes-hover = Con varias herramientas: un archivo de código G por herramienta (entre archivos cambia la herramienta y fija el cero Z), o un solo archivo con cambios de herramienta M6 que gestiona tu programa de envío (GRBL por sí solo rechaza M6)
forms-tool-change-files = un archivo por herramienta
forms-tool-change-program = un archivo con M6 (tu programa de envío gestiona el cambio)
forms-travel = Recorrido X Y Z
forms-max-feed-not-set = sin definir
forms-travel-not-checked = sin comprobar
forms-revert-hover = "{ $name }" tiene { $value }: haz clic para tomarlo
forms-post-yours-copied = Tuyo, copiado al proyecto
forms-post-changed = Tu archivo de postprocesador ha cambiado
forms-post-update = Actualizar
forms-post-update-hover = Copiar el postprocesador tal como está ahora al proyecto
forms-post-file-missing = Tuyo; el proyecto conserva su copia (el archivo no está)

# A tool's values.
forms-tool-type = Tipo
forms-diameter = Diámetro
forms-spindle-speed = Vel. del husillo
forms-centre-cutting = Corte al centro
forms-yes = sí
forms-no = no
forms-unknown = desconocido
forms-angle = Ángulo
forms-angle-hover = El ángulo incluido de la V; el tallado en V lo necesita
forms-tip = Punta
forms-tip-hover = El diámetro de la punta plana; sin definir se toma como puntiaguda
forms-pointed = puntiaguda

# A rest pass of a pocket: a smaller tool clears what the one before left.
forms-rest-pass-of = operación { $number }
forms-rest-pass-of-named = operación { $number } ({ $name })
forms-rest-pass-facts = { $start }–{ $end } { $unit }, holgura { $clearance } { $unit }, { $direction }
forms-rest-pass = Pasada de restos
forms-rest-of = Restos de
forms-rest-of-hover = Corta las formas y profundidades de su cajera, y solo quita lo que deja la herramienta anterior
# A button: show the pocket.
forms-open = Abrir
forms-from-pocket = De la cajera
forms-after-tool = Tras herramienta
forms-after-tool-hover = Quita lo que deja esta herramienta: la de la cajera, o la de la pasada anterior
forms-cut-it = Cortarla
forms-cut-it-hover = Desactivado: se conserva, pero no se corta
forms-rest-pass-note = Se corta después de las cajeras contiguas a su cajera que usan la misma herramienta, todas sus pasadas de restos juntas: la herramienta pequeña entra una sola vez.
forms-remove-rest-pass = Quitar pasada de restos
forms-floor-pass = Pasada de fondo
forms-floor-of = Fondo de
forms-floor-of-hover = Corta las formas de su tallado en V hasta su profundidad final, apartada de las paredes inclinadas
forms-from-vcarve = Del tallado en V
forms-floor-pass-facts = { $start }–{ $end } { $unit }, { $inset } { $unit } hacia dentro del contorno
forms-floor-pass-note = Se corta antes de los tallados en V contiguos que usan la misma fresa en V, todas sus pasadas de fondo juntas. Donde la fresa plana no llega, la fresa en V repasa a lo largo de la línea central.
forms-remove-floor-pass = Quitar pasada de fondo
forms-floor-hint = Deja parte del fondo: una de { $largest } como máximo lo vaciaría entero.
forms-floor-hint-library = En tu biblioteca: { $tools }

# Font styles, headings in the font list.
forms-font-plain = Normal
forms-font-script = Manuscrita
forms-font-display = Decorativa
forms-explode-dxf = Descomponer…
forms-explode-dxf-hover = Convertir las formas de este DXF en formas dibujadas que puedes editar; se pierde el vínculo con el archivo DXF
