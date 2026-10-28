# Messages of the app services (crates/chipload-pipeline): problems found with the
# post-processor a project uses, planning notes, the time estimate, chip loads and the
# drawing's facts. A length or feed argument brings its own unit ("12.5 mm",
# "1200 mm/min").

# $post is the post-processor's name, $units the G-code units the machine is set to, as
# written in the project file ("inch"). "units" is a key of the post's config file.
pipeline-post-units = el postprocesador { $post } solo escribe código G en mm (su configuración no incluye "{ $units }" en units)

## Planning notes

pipeline-drawing-outside-stock = El dibujo se sale del material en bruto
# $count is the number of copies. $x0 and $y0 are numbers without their unit; $x1, $y1
# and $stock_y bring it; $stock_x is a number without it.
pipeline-copies-beyond-stock = Las { $count } copias cortan de X{ $x0 } a { $x1 }, de Y{ $y0 } a { $y1 }, fuera del material en bruto ({ $stock_x } × { $stock_y }); revisa el material en bruto o las copias de la pieza.
# As pipeline-copies-beyond-stock, for one part of several; $part is its name.
pipeline-part-copies-beyond-stock = Pieza { $part }: las { $count } copias cortan de X{ $x0 } a { $x1 }, de Y{ $y0 } a { $y1 }, fuera del material en bruto ({ $stock_x } × { $stock_y }); revisa el material en bruto o las copias de la pieza.
# $label is an operation's label, $layer a layer's name, $sources where the layer's shapes
# come from (DXF and picture names, joined with pipeline-and).
pipeline-layer-clash = { $label }: la capa "{ $layer }" está en { $sources }; se cortan todas. Limítala a una (selecciona la capa bajo ese DXF en el Navegador)
# Names joined: "a.dxf and b.dxf".
pipeline-and = { $list } y { $last }
# Where shapes come from: a DXF's or picture's name, or one without a name.
pipeline-source-named = { $name }
pipeline-source-dxf = DXF { $id }
pipeline-source-image = imagen { $id }
pipeline-source-drawn = formas dibujadas

## The time estimate (after planning and generating)

# A duration: "45 s", "12 min 30 s", "1 h 05 min". $minutes after hours has two digits.
pipeline-duration-seconds = { $seconds } s
pipeline-duration-minutes = { $minutes } min { $seconds } s
pipeline-duration-hours = { $hours } h { $minutes } min
# The durations are pipeline-duration-… texts; $changes is the number of tool changes.
pipeline-estimate = Tiempo estimado: { $total } (corte { $cutting }, rápidos { $rapids }, husillo { $spindle }){ $changes ->
    [0] {""}
    [one] , más { $changes } cambio de herramienta
   *[other] , más { $changes } cambios de herramienta
}
# The lines under it start with spaces, written {"  "}: keep them, they indent the line.
# $name is the part's name.
pipeline-estimate-part = {"  "}Pieza { $number } ({ $name }): { $time }
# $label is an operation's label.
pipeline-estimate-operation = {"  "}{ $label }: { $time }
# An operation under its part: indented deeper.
pipeline-estimate-part-operation = {"    "}{ $label }: { $time }
# One G-code file of a project written one file per tool.
pipeline-estimate-file = {"  "}Archivo { $number }: { $time }
# $rate is a feed ("5000 mm/min").
pipeline-estimate-rapids-assumed = {"  "}A los avances programados (rápidos a { $rate }, supuesto: fija el avance máximo de la máquina para una mejor estimación); la aceleración y las esquinas alargan el tiempo real.
pipeline-estimate-rapids = {"  "}A los avances programados (rápidos al avance máximo de la máquina, { $rate }); la aceleración y las esquinas alargan el tiempo real.

## Chip load

pipeline-chip-load-title = Avance por diente (al avance de corte):
# $label is an operation's label, $load a pipeline-chip-load… text.
pipeline-chip-load-line = {"  "}{ $label }: { $load }
# $load brings its unit ("0.042 mm/tooth").
pipeline-chip-load = { $load }
pipeline-chip-load-no-flutes = fija los filos de la herramienta para ver el avance por diente
pipeline-chip-load-no-rpm = fija la velocidad del husillo de la herramienta para ver el avance por diente
pipeline-chip-load-no-tool = sin herramienta
# The hover text next to an operation's chip load. Tool diameters $small_tool (a number)
# and $large_tool (with its unit); chip loads $wood_low, $aluminium_low and
# $aluminium_high (numbers) and $wood_high (with its unit).
pipeline-chip-load-hint = Avance por diente: el grosor de viruta que arranca cada filo, avance / (velocidad del husillo × filos). Demasiado fino y la herramienta roza y se calienta; demasiado grueso y puede romperse. Depende del material, la herramienta y la máquina; como guía aproximada, la madera y los plásticos con herramientas de { $small_tool }–{ $large_tool } suelen admitir { $wood_low }–{ $wood_high }, el aluminio { $aluminium_low }–{ $aluminium_high }. Consulta los datos del fabricante de la herramienta.

## The drawing's facts (after importing)

# $units is "mm" or "inch", as the DXF gives them or the user chose them.
pipeline-report-units = Unidades: { $units } ({ $from ->
    [dxf] del DXF
    [svg] del SVG
   *[user] elegidas por ti
})
# Numbers without their unit, except $height, which brings it.
pipeline-report-extents = Extensión: X { $x0 } .. { $x1 }, Y { $y0 } .. { $y1 } ({ $width } x { $height })
# $items lists DXF entity types with their counts ("LINE 4, ARC 2").
pipeline-report-imported = Importado: { $items }
pipeline-report-flattened = Aplanado: { $items }
pipeline-report-ignored = Ignorado: { $items }
pipeline-report-no-issues = Incidencias: ninguna
pipeline-report-issues = Incidencias ({ $count }):
# One issue of the import (a dxf-… text), indented.
pipeline-report-issue = {"  "}[{ $severity ->
    [warning] aviso
   *[error] error
}] { $issue }
pipeline-report-contours = Contornos ({ $count }):
# A row of the contour table, in a fixed-width font: $id, $state and $layers are padded
# with spaces to line up. The coordinates are numbers without their unit.
pipeline-report-contour = {"  "}#{ $id } { $state } capa { $layers } X { $x0 } .. { $x1 }  Y { $y0 } .. { $y1 }
# A contour's state in the table: keep it short.
pipeline-report-contour-state = { $state ->
    [open] abierto
    [invalid] NO VÁLIDO
    [hole] agujero
   *[outer] exterior
}

## The G-code checker's findings (check-… texts), placed in the program

# A finding on a line of the program; $message is the finding.
pipeline-check-line = línea { $line }: { $message }
# A finding in one of several G-code files: $file is the file's number.
pipeline-check-file = archivo { $file }: { $message }
# A finding on a line of one of several G-code files.
pipeline-check-file-line = archivo { $file }, línea { $line }: { $message }

## Errors: a change or run the project refuses

# The independent checker refused the program; each finding follows on its own line.
pipeline-error-check-failed = La comprobación del código G falló:
# One more finding (a pipeline-check-… text) on a line of its own, indented, after $list.
pipeline-error-check-finding =
    { $list }
    {"  "}{ $finding }
# Words of the command line or another library, kept as they are.
pipeline-text = { $text }

## Reports of changes to the drawing

# $x0 and $y0 are lengths without their unit, $x1 and $y1 with it; $width and $length the
# stock's size, the first without its unit.
pipeline-stock-fit = El dibujo (X{ $x0 } a { $x1 }, Y{ $y0 } a { $y1 }) se sale del material en bruto ({ $width } × { $length }); revisa el material en bruto en Ajustes del trabajo.

## Placing a DXF: moving, turning and scaling it

# $count is how many DXFs the project has, $dxfs their numbers and names ("1 part-a, 2 part-b").
pipeline-place-which-dxf = el proyecto tiene { $count } DXF ({ $dxfs }): indica cuál colocar
pipeline-no-dxf = no hay DXF { $id }
pipeline-place-nothing-imported = el proyecto no tiene ningún dibujo importado que mover
# What is placed, when the project has a single DXF (otherwise it is the DXF's name).
pipeline-place-the-drawing = el dibujo importado
# $dx and $dy are numbers as typed, without a unit.
pipeline-place-cannot-move = no se puede mover el dibujo ({ $dx }, { $dy })
pipeline-place-already-there = El dibujo ya está ahí; no se movió nada.
# $what is pipeline-place-the-drawing or a DXF's name; $degrees without its unit.
pipeline-place-turned = Se giró { $what } { $degrees }° en sentido { $direction ->
    [ccw] antihorario
   *[cw] horario
}
pipeline-place-reset = Se devolvió { $what } a donde lo tiene el DXF
# $factor is a scale factor ("2", "0.5").
pipeline-place-scaled = Se escaló { $what } ×{ $factor }
# $scaled is pipeline-place-scaled; $width is a length without its unit, $height with it.
pipeline-place-scaled-size = { $scaled } (ahora { $width } × { $height })
# $dx is a length without its unit, $dy with it.
pipeline-place-moved = Se movió { $what } X{ $dx } Y{ $dy }
# $done is one of the texts above; $placement a pipeline-placement text.
pipeline-place-done = { $done }; ahora está { $placement }.
pipeline-place-moved-starts = { $count ->
    [one] Se movió { $count } punto de inicio con él.
   *[other] Se movieron { $count } puntos de inicio con él.
}
pipeline-place-moved-corners = { $count ->
    [one] Se movió { $count } elección de esquina con él.
   *[other] Se movieron { $count } elecciones de esquina con él.
}
pipeline-place-moved-tabs = { $count ->
    [one] Se movió { $count } puente colocado a mano con él.
   *[other] Se movieron { $count } puentes colocados a mano con él.
}
pipeline-place-drawn-stay = Las formas dibujadas en Chipload se quedan donde están.
pipeline-place-other-dxfs-stay = Los demás DXF se quedan donde están.
pipeline-place-work-zero-stays = El cero pieza se queda en el punto X{ $x }, Y{ $y }; comprueba que sigue donde lo quieres.
pipeline-place-margin = el margen debe ser 0 o más, es { $got }
# $got is a number as typed, in degrees, without a unit.
pipeline-place-rotation = el giro debe ser un número de grados, es { $got }
pipeline-place-scale = la escala debe ser un número mayor que 0, es { $got }
# Scale factors: the result, and the smallest and largest allowed.
pipeline-place-scale-range = eso escalaría el dibujo ×{ $factor } respecto al DXF; debe quedar entre ×{ $min } y ×{ $max }
# $what is "width" or "height"; $got a length without its unit.
pipeline-place-size = { $what ->
    [height] la altura
   *[width] el ancho
} debe ser mayor que 0, es { $got }
pipeline-place-no-size = el dibujo importado no tiene { $what ->
    [height] altura
   *[width] ancho
} que escalar
# How a DXF is placed: "scaled ×2, turned 90°, moved X12.000 Y-3.500 from the DXF".
pipeline-placement-as-dxf = como en el DXF
pipeline-placement-from-dxf = { $parts } respecto al DXF
pipeline-placement-and = { $list }, { $last }
pipeline-placement-scaled = escalado ×{ $factor }
pipeline-placement-turned = girado { $degrees }°
# $dx and $dy are lengths without their unit.
pipeline-placement-moved = movido X{ $dx } Y{ $dy }
# After scaling: $within and $off are lengths, $factor the scale factor.
pipeline-scale-curves = { $count ->
    [one] { $count } forma de splines o elipses se aproximó
   *[other] { $count } formas de splines o elipses se aproximaron
} con una tolerancia de { $within } al tamaño anterior; escaladas ×{ $factor }, pueden desviarse hasta { $off } de sus curvas. Actualiza desde el DXF para volver a aproximarlas a este tamaño.
pipeline-scale-contours = { $was ->
    [one] { $was } contorno cerrado
   *[other] { $was } contornos cerrados
} antes, { $now } ahora: los huecos de hasta { $within } se unen al nuevo tamaño. Revisa el informe de importación y las selecciones de las operaciones.
# --units is an option of the command line.
pipeline-scale-units = Si las unidades del DXF están mal, fíjalas al actualizar (--units), para que el proyecto las registre.

## Adding, updating and removing a DXF

# $file is the DXF's file, $id its number, $name its name in the project.
pipeline-dxf-added = Se añadió { $file } como { $kind } { $id } "{ $name }": { $count } { $count ->
    [one] forma
   *[other] formas
}.
# $operations are operation numbers ("2, 5"); $name a DXF's name.
pipeline-dxf-pinned = { $count ->
    [one] La operación { $operations } sigue cortando solo { $name }: el DXF nuevo también tiene sus capas.
   *[other] Las operaciones { $operations } siguen cortando solo { $name }: el DXF nuevo también tiene sus capas.
}
pipeline-dxf-removed = Se eliminó el { $kind } { $id } "{ $name }" y { $count ->
    [one] su { $count } forma
   *[other] sus { $count } formas
}.
# $label is an operation's label; $shapes shape numbers ("S3, S4").
pipeline-dxf-shapes-gone = { $label }: las formas { $shapes } ya no existen; vuelve a elegir su geometría
pipeline-dxf-gone = { $label }: su DXF ya no está; vuelve a elegir su geometría
pipeline-dxf-updated = Actualizado desde { $file }: { $unchanged } sin cambios, { $changed } cambiadas, { $added } añadidas, { $removed } eliminadas.
# $placement is a pipeline-placement text.
pipeline-dxf-placed-as-before = Colocado como antes: { $placement }.
pipeline-dxf-geometry-changed = { $label }: la geometría cambió (formas { $shapes }); revisa su trayectoria
pipeline-dxf-layer-updated = { $label }: la capa { $layer } usa ahora el dibujo actualizado; revisa su trayectoria

## Pictures and texts

# $number is an operation's number.
pipeline-left-out-found = Operación { $number }: { $count ->
    [one] se volvió a encontrar su { $count } forma excluida
   *[other] se volvieron a encontrar sus { $count } formas excluidas
}.
pipeline-left-out-lost = Operación { $number }: { $count ->
    [one] no se volvió a encontrar { $count } forma excluida
   *[other] no se volvieron a encontrar { $count } formas excluidas
}: vuelve a elegir las formas que excluir.
pipeline-no-image = no hay imagen { $id }
# $error is the PNG writer's own words.
pipeline-image-png-failed = al escribir la imagen: { $error }
# drawing.images is a place in the project file; $error the decoder's own words.
pipeline-image-unreadable = drawing.images: no se puede leer el archivo guardado de la imagen { $id }: { $error }
# $file is the picture's file; $width a length without its unit, $height with it;
# $threshold a grey level (0–255); $picked "yes" when the threshold was picked from the
# picture; $lines "yes" for centre lines, "no" for outlines.
pipeline-image-traced = Se vectorizó { $file } ({ $width } × { $height }, umbral { $threshold }{ $picked ->
    [yes] , elegido
   *[no] {""}
}): { $count } { $lines ->
    [yes] { $count ->
        [one] línea central
       *[other] líneas centrales
    }
   *[no] { $count ->
        [one] forma
       *[other] formas
    }
} en la capa { $layer }.
pipeline-image-nothing-black = Nada es negro: cambia el umbral, o invierte si la imagen es clara sobre oscuro.
# $narrow is a length without its unit, $wide with it.
pipeline-image-strokes = Trazos de { $narrow } a { $wide } de ancho: córtalos con un seguir trayectoria.
pipeline-image-closed-lines = { $count ->
    [one] { $count } línea cerrada: un contorneado o una cajera puede cortar alrededor de ella, como de cualquier contorno.
   *[other] { $count } líneas cerradas: un contorneado o una cajera puede cortar alrededor de ellas, como de cualquier contorno.
}
pipeline-image-dots = { $count ->
    [one] Se omitió { $count } punto demasiado pequeño para una línea central.
   *[other] Se omitieron { $count } puntos demasiado pequeños para una línea central.
}
# $area is the smallest piece's area in mm², without its unit.
pipeline-image-specks = { $count ->
    [one] Se descartó { $count } mota menor que { $area } mm².
   *[other] Se descartaron { $count } motas menores que { $area } mm².
}
pipeline-image-black-at-border = El negro toca el borde de la imagen y se cierra a lo largo de él; si es el fondo, invierte.
pipeline-image-without-arcs = { $count ->
    [one] { $count } forma se conserva como líneas: sus arcos quedaban demasiado cerca de otra forma.
   *[other] { $count } formas se conservan como líneas: sus arcos quedaban demasiado cerca de otra forma.
}
pipeline-image-unsimplified = { $count ->
    [one] { $count } forma se conserva tal como se vectorizó: el suavizado la hacía cruzarse.
   *[other] { $count } formas se conservan tal como se vectorizaron: el suavizado las hacía cruzarse.
}
pipeline-no-text = no hay texto { $id }
# $text is the text's first line in quotes; $error why (a toolpath-error text).
pipeline-text-join-failed = { $text }: no se pudieron unir los contornos superpuestos de las letras: { $error }
# $text is the text's first line in quotes; $height its height; $width the box's width
# without its unit, $tall its height with it.
pipeline-text-wrote = Se escribió { $text } en { $font }, de { $height } de alto ({ $width } × { $tall }): { $count } { $count ->
    [one] forma
   *[other] formas
} en la capa { $layer }.
# $missing are the characters the font has no letter for.
pipeline-text-missing = { $font } no tiene letra para "{ $missing }": se omite.
pipeline-text-dropped = { $count ->
    [one] Se omitió { $count } contorno demasiado pequeño para cortarlo.
   *[other] Se omitieron { $count } contornos demasiado pequeños para cortarlos.
}

## Errors: operations and tools

# `tool use` is a command of the command line: keep it as written.
pipeline-no-tool-use = el proyecto no tiene la herramienta { $tool }; copia una de tu biblioteca con `tool use`
pipeline-no-tool = el proyecto no tiene la herramienta { $tool }
# $number is the operation's number as listed, $count how many the project has.
pipeline-no-operation = no hay operación { $number }; el proyecto tiene { $count }
pipeline-tab-no-ring = la operación { $number } no tiene ningún contorno a menos de un diámetro de herramienta de { $at }; `chipload op tabs` lista sus puentes
pipeline-tab-none-there = la operación { $number } no tiene ningún puente a menos de un diámetro de herramienta de { $at }; `chipload op tabs` lista sus puentes
pipeline-tab-no-tabs = la operación { $number } no tiene puentes: solo un contorneado con puentes tiene puentes que colocar
pipeline-corner-none-there = la operación { $number } no tiene ninguna esquina en { $at }; `chipload op corners` lista sus esquinas
pipeline-corner-not-profile-or-pocket = la operación { $number } no tiene esquinas que elegir: solo los contorneados y las cajeras tienen dogbones
pipeline-not-a-pocket = la operación { $number } no es una cajera; solo las cajeras admiten mecanizado de restos
# $reason is one of the pipeline-rest-… texts.
pipeline-operation-refused = operación { $number }: { $reason }
pipeline-rest-dogbones = tiene dogbones, y los dogbones y el mecanizado de restos aún no se pueden combinar; las esquinas dogbone no necesitan restos
pipeline-rest-not-endmill = la herramienta { $tool } no es una fresa
pipeline-rest-before-not-endmill = la herramienta { $tool } anterior no es una fresa
pipeline-rest-not-smaller = la herramienta { $tool } ({ $diameter }) no es más pequeña que la herramienta { $before } ({ $before_diameter }) anterior
# $dxfs lists the project's DXFs by number and name ("1 part-a, 2 part-b").
pipeline-update-which-dxf = el proyecto tiene { $count } DXF ({ $dxfs }): indica cuál actualizar
# `tool use` is a command of the command line: keep it as written.
pipeline-surface-no-endmill = el proyecto no tiene ninguna fresa: un planeado necesita una (cópiala con `tool use`)
# $type is the tool's type as written in the project file (drill, vbit).
pipeline-surface-not-endmill = la herramienta { $tool } es { $type }; un planeado necesita una fresa

## Errors: drawn shapes

pipeline-shape-not-drawable = solo se pueden dibujar círculos, rectángulos y polilíneas
# $kind is the shape's kind as written in the project file (circle, rectangle, polyline).
pipeline-shape-kind = la forma { $id } es { $kind }; su tipo no puede cambiar
pipeline-no-shapes-given = no se indicó ninguna forma
pipeline-shape-twice = la forma { $id } se indicó dos veces
pipeline-no-shape = no hay forma { $id }
pipeline-shape-imported-copy = la forma { $id } se importó del DXF; solo las formas dibujadas se pueden copiar o reflejar
pipeline-shapes-no-extent = las formas no tienen extensión
pipeline-shape-imported-transform = la forma { $id } no es una forma dibujada: solo las formas dibujadas se pueden mover, girar o escalar (descompón primero su DXF)
pipeline-transform-not-numbers = el desplazamiento, el giro y el centro deben ser números
pipeline-shapes-copy-imported = solo se pueden copiar las formas dibujadas, no las importadas
# "Update drawing from DXF" is the command in the app's File menu.
pipeline-shape-imported-change = la forma { $id } se importó del DXF y no se puede cambiar aquí; cambia el dibujo y usa Actualizar dibujo desde DXF

## Errors: parts and copies

pipeline-no-part = No hay pieza { $id } en el proyecto
pipeline-part-name-empty = El nombre de una pieza no puede estar vacío
pipeline-part-name-taken = Otra pieza se llama "{ $name }"
pipeline-last-part = Un proyecto necesita al menos una pieza: esta es la última
pipeline-part-has-operations = La pieza { $name } tiene { $count ->
    [one] { $count } operación
   *[other] { $count } operaciones
}: muévelas antes a otra pieza, o elimínalas con ella
pipeline-part-no-operation = No hay operación { $number } en el proyecto
pipeline-part-cuts-nothing = La pieza no corta nada
pipeline-copies-gap = La separación debe ser 0 o más, es { $got }

## Errors: the simulation

# $min is a length without its unit, $max and $got with it.
pipeline-simulate-cell-size = el tamaño de celda debe estar entre { $min } y { $max }, es { $got }
# $cell is the size of one cell, a length.
pipeline-simulate-too-many-cells = una celda de { $cell } da { $cells } celdas para este material en bruto, más de { $most }; usa una celda mayor
pipeline-simulate-stock-too-large = el material en bruto es demasiado grande para simularlo con celdas de { $cell } (más de { $most } celdas)
# $error is the PNG encoder's own words.
pipeline-simulate-png = PNG: { $error }

## Post-processors

pipeline-post-no-settings = no hay carpeta de ajustes, así que no hay postprocesador "{ $name }"
pipeline-post-no-folder = No existe el postprocesador "{ $name }": no está incluido y no hay carpeta posts
# Why a project's copy of a user post can't be compared with its file.
pipeline-post-no-posts-folder = no hay carpeta posts
# What a post-processor is, one line each (Settings, This computer). $name, $extension,
# the paths and the units (mm, inch) and modes (files, program) are written as in its file.
pipeline-post-name = Nombre: { $name }
pipeline-post-builtin = Incluido
pipeline-post-yours = Tuyo
pipeline-post-settings = Ajustes: { $path }
pipeline-post-template = Plantilla: { $path }
pipeline-post-extension = Extensión de archivo: { $extension }
pipeline-post-decimals = Decimales: { $mm } (mm), { $inch } (inch)
pipeline-post-units-list = Unidades del código G: { $units }
pipeline-post-arcs = Arcos: { $arcs ->
    [arcs] G2/G3
   *[lines] como líneas cortas (G1)
}
pipeline-post-tool-changes = Varias herramientas: { $modes }
# $seconds is a time in seconds, without its unit.
pipeline-post-dwell = Espera del husillo: { $seconds } s
pipeline-post-no-dwell = Espera del husillo: ninguna

## The tool library and machine profiles

# $error is why the library file couldn't be read (a job-library-error text).
pipeline-library-defaults = { $error } (se usan los valores incluidos por defecto)
pipeline-library-no-profile = no hay ningún perfil de máquina llamado "{ $name }"
# What a new project started with. $how is "profile" (asked for) or "default".
pipeline-library-machine = máquina: { $name } ({ $how ->
    [default] perfil predeterminado
   *[profile] perfil
})
pipeline-library-tool = herramienta { $tool }: { $name } (herramienta predeterminada)
# $error is why the post can't be used (a post-error text).
pipeline-library-post-unusable = postprocesador "{ $name }": { $error }
pipeline-library-unknown-post = Máquina "{ $name }": este Chipload no tiene el postprocesador "{ $post }"; los proyectos que la usan no pueden generar código G hasta que lo tenga

## Recovery copies

# $reason is the operating system's words, or the text below.
pipeline-recovery-io = No se puede escribir la copia de recuperación { $path }: { $reason }
pipeline-recovery-locked = ya está bloqueada

## Dragging a picture's or text's box

pipeline-box-turn-centre = el tirador de giro está en el centro
pipeline-box-no-size = el recuadro no tiene tamaño
# $min is the smallest size, a length.
pipeline-box-too-small = no puede ser menor que { $min }
pipeline-box-no-corner = un recuadro no tiene esquina { $corner }

## G-code files

# One of several G-code files: $name is its file name, $operations the numbers of the
# operations it cuts ("1, 3").
pipeline-gcode-file = { $name } (herramienta { $tool }, operaciones { $operations })

## Explode a DXF, join close ends (docs/specs/explode-and-join.md)

pipeline-dxf-exploded = { $kind } { $id } "{ $name }" descompuesto en { $count } { $count ->
    [one] forma dibujada
   *[other] formas dibujadas
}. El proyecto ya no conoce el archivo { $kind }.
# $shapes shape numbers ("S3, S4"); $tolerance a length.
pipeline-explode-split = { $count ->
    [one] La forma { $shapes } estaba en trozos que no se tocan: ahora cada trozo es una forma.
   *[other] Las formas { $shapes } estaban en trozos que no se tocan: ahora cada trozo es una forma.
}
pipeline-explode-circle-start = { $count ->
    [one] El círculo { $shapes } ahora empieza en su punto derecho, en sentido antihorario, como los círculos dibujados: su corte puede empezar en otro punto.
   *[other] Los círculos { $shapes } ahora empiezan en sus puntos derechos, en sentido antihorario, como los círculos dibujados: sus cortes pueden empezar en otro punto.
}
pipeline-explode-dropped = { $count ->
    [one] La forma { $shapes } solo tenía segmentos de menos de { $tolerance } y se ha eliminado.
   *[other] Las formas { $shapes } solo tenían segmentos de menos de { $tolerance } y se han eliminado.
}
# $label is an operation's label.
pipeline-explode-selects = { $label } ahora selecciona las formas { $shapes }.
pipeline-explode-no-dxf = el proyecto no tiene ningún DXF que descomponer
pipeline-explode-which-dxf = el proyecto tiene { $count } DXF ({ $dxfs }): indica cuál descomponer
pipeline-join-gap-range = La separación debe estar entre { $least } y { $most }, es { $got }.
pipeline-join-imported = La forma S{ $id } viene de un DXF: descompón primero el DXF.
pipeline-join-none = No se unió ningún extremo (separación { $gap }).
pipeline-joined = { $pieces } formas unidas en { $count }, { $closed } de ellas cerradas; mayor separación cerrada { $widest }.
pipeline-joined-touching = { $pieces } formas unidas en { $count }, { $closed } de ellas cerradas; sus extremos se tocaban.
pipeline-join-open = { $count } { $count ->
    [one] extremo queda abierto
   *[other] extremos quedan abiertos
}.
# $at a point, $nearest a length.
pipeline-join-open-end = El extremo en { $at }: el otro extremo más cercano está a { $nearest }.
pipeline-join-open-alone = El extremo en { $at }: no hay otro extremo en su capa.
pipeline-join-branch = Tres o más extremos se encuentran en { $at }: ahí no se unen.
pipeline-edit-imported = S{ $id } viene de un DXF, una imagen o un texto: descomponga el DXF, o dibuje la forma, para cambiarla
pipeline-offset-distance = el desfase debe ser mayor que 0, se recibió { $got }
pipeline-offset-vanishes = no queda nada de S{ $id } a { $distance } hacia dentro
pipeline-offset-side = S{ $id } está { $closed ->
    [closed] cerrada: desfásela por fuera o por dentro
   *[open] abierta: desfásela a la izquierda o a la derecha
}
pipeline-offset-failed = S{ $id } no se pudo desfasar: { $why }
pipeline-new-shapes-cut = { $count ->
    [one] La forma nueva está
   *[other] Las { $count } formas nuevas están
} en la capa { $layer }, que { $ops ->
    [one] corta la operación { $operations }
   *[other] cortan las operaciones { $operations }
}.
pipeline-corner-none = S{ $id } no tiene esquinas que redondear o achaflanar
pipeline-corner-not = la esquina { $corner } de S{ $id } no está entre dos lados rectos
pipeline-corner-straight = S{ $id } sigue recta en { $corner }: no hay esquina
pipeline-corner-too-large = es demasiado grande para los lados de S{ $id }: como mucho { $most }
pipeline-corner-size = el radio o la distancia debe ser mayor que 0
pipeline-trim-nothing-crosses = nada cruza S{ $id } ahí (una forma cerrada necesita dos cruces): bórrela en su lugar
pipeline-trim-everything = recortar S{ $id } ahí no dejaría nada: bórrela en su lugar
pipeline-extend-not-straight = solo un extremo recto de S{ $id } se puede alargar
pipeline-extend-nothing-ahead = no hay nada delante de ese extremo de S{ $id }
pipeline-extend-closed = S{ $id } está cerrada: no tiene extremo que alargar
pipeline-break-at-end = ese es un extremo de S{ $id }: no hay nada que partir
pipeline-combine-two = para combinar hacen falta dos formas o más
pipeline-combine-open = S{ $id } está abierta: solo se pueden combinar formas cerradas
pipeline-combine-empty = { $op ->
    [subtract] no queda nada de S{ $id }: las otras formas la cubren por completo
   *[other] las formas no tienen ninguna parte en común: no queda nada
}
pipeline-combine-failed = no se pudieron combinar las formas: { $why }
pipeline-array-count = una matriz tiene de 2 a { $most } formas contando el original, es { $got }
pipeline-array-spacing = las copias quedarían unas sobre otras: da una separación distinta de 0 donde haya más de una columna o fila
pipeline-array-angle = el ángulo debe ser mayor que 0 y de 360 grados como mucho, es { $got }
pipeline-align-two = para alinear hacen falta dos formas o más, o el material en bruto como referencia
pipeline-distribute-three = para distribuir hacen falta tres formas o más
pipeline-shape-gone = S{ $id } ya no existe: { $ops ->
    [one] la operación { $operations } la usaba; vuelve a elegir su geometría
   *[other] las operaciones { $operations } la usaban; vuelve a elegir su geometría
}.

## DXF export (docs/specs/dxf-export.md)

pipeline-export-nothing = no hay nada que exportar: ahí no se muestra ninguna forma
pipeline-export-no-layer = el dibujo no tiene la capa { $layer }
pipeline-export-unverified = no se escribió el DXF: S{ $id } no se volvió a leer igual ({ $why })
pipeline-export-summary = { $count ->
    [one] 1 entidad
   *[other] { $count } entidades
} en { $layers ->
    [one] 1 capa
   *[other] { $layers } capas
}, en { $units ->
    [inch] pulgadas
   *[mm] mm
}.
pipeline-export-renamed = La capa { $from } se llama { $to } en el DXF: un nombre de capa DXF no admite algunos de sus caracteres.
