# Messages of the project model (crates/chipload-job): what is wrong with a project's
# values. Each problem is shown under its field in the app, and on the command line
# after its place in the project file, as "operations.1.feed: must be greater than 0".
# Places ($path, $at) are written as in the project file and never translated. So are
# words in the texts that name fields of the project file (end_depth, next_shape_id,
# drawing.images, …): they are written in the file that way.
# Every $got is the value found; a length, feed or angle argument brings its own unit
# ("12.5 mm", "1200 mm/min") unless its comment says otherwise.

## Places and names

# A problem after its place.
job-problem-at = { $path }: { $problem }
# A problem of an operation after its label ("Operation 3 (Outline): …").
job-operation-problem = { $label }: { $problem }
# A shape's problem: $at is its place (drawing.shapes.4), $id its number (S7), $problem
# the problem with its field ("radius: must be …").
job-shape-problem = { $at } (S{ $id }).{ $problem }
# Operations by their number, and with the name the user gave them.
job-operation-label = Operación { $number }
job-operation-label-named = Operación { $number } ({ $name })
# A rest pass of a pocket: $number is the pocket's and the pass's, as "3.1".
job-operation-label-rest = Operación { $number } (restos)
job-operation-label-named-rest = Operación { $number } ({ $name }, restos)
# The kinds of tools, alone and with their article, inside other messages.
job-tool-type = { $type ->
    [drill] broca
    [vbit] fresa en V
   *[endmill] fresa
}
job-tool-type-article = { $type ->
    [drill] una broca
    [vbit] una fresa en V
   *[endmill] una fresa
}

## Rules for any value

job-must-be-positive = debe ser mayor que 0, es { $got }
job-must-be-positive-number = debe ser un número positivo, es { $got }
job-must-be-more-than-0 = debe ser más de 0, es { $got }
job-must-be-0-or-more = debe ser 0 o más, es { $got }
job-must-be-1-or-more = debe ser 1 o más
job-must-be-at-least-1 = debe ser al menos 1
job-must-not-be-empty = no debe estar vacío
job-must-be-finite = debe ser finito
# $got is an angle in radians, without a unit.
job-must-be-finite-got = debe ser finito, es { $got }
# $got is a value that isn't a number (NaN, inf).
job-must-be-number = debe ser un número, es { $got }
# Two lengths without their unit.
job-must-be-numbers = deben ser números, son ({ $x }, { $y })
job-xy-must-be-numbers = x e y deben ser números, son ({ $x }, { $y })
# $id is a number given to two things of the same kind.
job-id-twice = { $id } se usa dos veces
# Unitless numbers.
job-more-than-0-at-most = debe ser más de 0 y como máximo { $most }, es { $got }
job-rotation-range = debe ser al menos 0 y menos de 360 grados, es { $got }
job-angle-below-180 = debe ser más de 0 y menos de 180 grados, es { $got } grados
job-ramp-angle-range = debe ser más de 0 y como máximo 30 grados, es { $got } grados
job-flank-angle-range = debe ser más de 0 y como máximo 90 grados, es { $got } grados

## Copies in a grid

job-copies-per-side = debe ser de 1 a { $most }, es { $got }
job-copies-too-many = como máximo { $most } copias en total, son { $got }
job-copies-pitch-zero = no debe ser 0 con más de una copia en esa dirección

## Pictures and texts

# A picture setting's range in pixels; numbers without unit.
job-image-range-px = debe ser de { $low } a { $high } px, es { $got } px
# A picture setting's range as an area in square millimetres; numbers without unit.
job-image-range-area = debe ser de { $low } a { $high } mm², es { $got } mm²
# A length's range: $low is a number alone, $high and $got have their unit.
job-image-range-length = debe ser de { $low } a { $high }, es { $got }
# "auto" is written in the project file as it is.
job-image-threshold = debe ser de 1 a 254 o "auto", es { $got }
job-image-missing = falta la imagen
job-text-empty = está vacío: escribe algo que cortar
job-text-height = debe ser más de 0 y como máximo { $most }, es { $got }
# $heights is a number of text heights, $most that as a length.
job-text-spacing = debe ser como máximo { $heights } alturas en cualquier sentido ({ $most }), es { $got }

## The drawing

# next_shape_id is a field of the project file.
job-shape-id-range = debe estar entre 1 y next_shape_id - 1 ({ $most }), es { $got }
# drawing.images, drawing.dxfs and drawing.texts are places in the project file; $id is
# a number.
job-no-image = no hay imagen { $id } en drawing.images
job-no-dxf = no hay DXF { $id } en drawing.dxfs
job-no-text = no hay texto { $id } en drawing.texts
job-shape-one-source = una forma viene de una imagen, un DXF o un texto, no de varios
job-path-no-segments = una forma necesita al menos un segmento
job-segment-not-finite = las coordenadas deben ser finitas y los radios positivos
job-corner-radius-tiny = 0 para esquinas vivas, o al menos { $least }, es { $got }
job-corner-radius-too-large = como máximo { $most }, la mitad del lado más corto, es { $got }
# $closed is "closed" or "open".
job-polyline-points = { $closed ->
    [closed] una polilínea cerrada
   *[open] una polilínea abierta
} necesita al menos { $least } puntos, tiene { $got }
# $tolerance is a length.
job-point-on-previous = está encima del punto anterior (a menos de { $tolerance })
job-bulges-count = uno por segmento, { $wanted } para esta polilínea, hay { $got }
job-bulge-range = debe estar entre -1 y 1 (como máximo media circunferencia), es { $got }

## Machine and tools

# Two lengths without their unit.
job-work-zero-not-finite = x e y deben ser finitos, son { $x }, { $y }
job-tip-diameter = debe ser 0 o más y menor que el diámetro ({ $diameter }), es { $got }

## Parts

job-no-parts = un proyecto necesita al menos una pieza
job-part-twice = la pieza { $id } aparece dos veces
job-part-name-taken = otra pieza se llama "{ $name }"
# "parts" is a place in the project file.
job-no-part = no hay pieza { $id } en parts
job-part-order = todas las operaciones de la pieza { $part } deben ir antes que las de la pieza { $before }

## Placement and depths

job-scale-range = debe ser de { $low } a { $high }, es { $got }
# end_depth is a field of the project file.
job-start-below-end = debe ser menor que end_depth ({ $end }), es { $got }
job-on-line-clearance = un contorneado sobre la línea no tiene lado hacia el que cortar más; debe ser 0, es { $got }

## Surfacing

# Fractions of the tool diameter.
job-surface-stepover = debe ser de { $low } a { $high } del diámetro de la herramienta, es { $got }
job-surface-too-deep = un planeado no puede bajar más allá de la cara inferior del material en bruto ({ $thickness } de espesor), es { $got }

## Which shapes an operation cuts

job-selector-empty = el selector necesita un DXF, una imagen, un texto, una capa o formas
job-exclude-no-source = solo se pueden excluir formas de un DXF, una imagen o un texto completos
# $source is "dxf", "image" or "text"; $id its number.
job-exclude-missing = la forma { $shape } ya no está en { $source ->
    [dxf] el DXF
    [image] la imagen
   *[text] el texto
} { $id }: vuelve a elegir las formas que excluir
job-selector-one-source = un selector nombra un DXF, una imagen o un texto, no varios
# The part in braces is written in the project file as it is; $text is a number.
job-select-letter = la forma { $shape } es una letra de un texto y cambia cada vez que el texto se vuelve a generar: selecciona el texto en su lugar ({"{"}"text": { $text }{"}"})
job-select-traced = la forma { $shape } está vectorizada de una imagen y cambia cada vez que la imagen se vuelve a vectorizar: selecciona la imagen en su lugar ({"{"}"image": { $image }{"}"})
job-select-centre-lines = la imagen { $image } está vectorizada como líneas centrales abiertas: córtalas con un seguir trayectoria, o vectoriza la imagen como contornos

## Operations

job-on-line-corners = un contorneado sobre la línea no tiene esquinas interiores que despejar
job-tbone-wall = un T-bone necesita la pared a lo largo de la que va: la más larga o la más corta
job-wall-only-tbone = solo un T-bone tiene una pared a lo largo de la que ir
job-corner-chosen-twice = dos elecciones para la esquina en { $at }
job-starts-not-finite = cada punto de inicio necesita x e y finitos
job-stepover-at-most-1 = debe ser como máximo 1 (una fracción del diámetro de la herramienta), es { $got }
job-on-line-finish = un contorneado sobre la línea no tiene pared que acabar; usa interior o exterior
# Lengths without their unit.
job-skin-above-step-down = debe ser como máximo la profundidad de pasada ({ $step_down }), es { $got }
job-skin-above-cut = debe ser menor que el corte ({ $cut }, prof. final menos prof. inicial), es { $got }
job-skin-above-tabs = debe ser menor que la altura de los puentes ({ $height }), es { $got }

## Lists

# Messages joined: "a; b; c" and "a, b, c".
job-semicolons = { $list }; { $last }
job-commas = { $list }, { $last }


## Project files that can't be read

# $found is the schema_version in the file, as written there ("missing" when there is none).
job-error-version = El schema_version { $found } del proyecto no es compatible: esta versión de Chipload lee de { $oldest } a { $current }. Actualiza Chipload o revisa el archivo.
job-error-too-old = El schema_version { $found } del proyecto es de antes de que existieran los proyectos y ya no se lee. Ábrelo con Chipload 0.3.0 y guárdalo, o vuelve a importar su DXF.
# $reason is why the old file can't be carried forward, in English: it names the file's fields.
job-error-cannot-migrate = Este proyecto (schema_version { $from }) no se puede abrir: { $reason }
# $reason is the operating system's or JSON reader's own words, or one of the job-error texts below.
job-error-unreadable = No se puede leer el archivo de proyecto { $path }: { $reason }
# $reason is the JSON reader's own words, or the project's problems joined by semicolons.
job-error-invalid = Archivo de proyecto no válido { $path }: { $reason }
job-error-not-object = debe contener un objeto JSON
# $error is the operating system's words.
job-error-cannot-write = no se puede escribir: { $error }
job-error-no-backup = no se puede conservar la versión anterior como .bak: { $error }
# On the command line, after `set`: $paths are the values set, as paths in the project file.
job-set-would-not-load = el proyecto no se cargaría tras asignar { $paths }: { $reason }

## The tool library and machine profiles

# $path is the library, settings or recovery file; $found its schema_version as written there.
job-library-error-version = { $path }: el schema_version { $found } no es compatible: esta versión de Chipload lee el { $current }. Actualiza Chipload o revisa el archivo.
# $reason is the operating system's or JSON reader's own words, or a job- text.
job-library-error-unreadable = No se puede leer { $path }: { $reason }
# $reason is the JSON reader's own words, or the file's rule violations (in English).
job-library-error-invalid = { $path } no es válido: { $reason }
job-library-error-unwritable = No se puede escribir { $path }: { $reason }
# A change the library refuses: $reason is one of the texts below, or the app's own words.
job-library-error-refused = { $reason }
job-library-no-tool = no hay ninguna herramienta llamada "{ $name }"
job-library-no-machine = no hay ninguna máquina llamada "{ $name }"
# $entry is "tool" or "machine".
job-library-no-entry = no hay ninguna { $entry ->
    [machine] máquina
   *[tool] herramienta
} llamada "{ $name }"
# A file to import that holds the other library: $path is the file, $hint one of the two texts after it.
job-library-other-file = { $path }: { $hint }
job-library-holds-machines = este archivo contiene perfiles de máquina, no herramientas: impórtalo como máquinas
job-library-holds-tools = este archivo contiene herramientas, no perfiles de máquina: impórtalo como herramientas
# A value that differs between a project and the library, or a library and a file:
# "diameter 6 mm -> 8 mm". $what is job-library-field; $from and $to its values. Values
# that are words of the project file (endmill, vbit, cw, ccw, files, program, mm, inch,
# bottom-left, …) and names are written as they are.
job-library-change = { $what } { $from } -> { $to }
job-library-field = { $field ->
    [type] tipo
    [diameter] diámetro
    [flutes] filos
    [centre-cutting] corte al centro
    [rpm] rpm
    [angle] ángulo
    [tip] punta
    [id] id
    [cutting-values] valores de corte
    [post] postprocesador
    [G-code-units] unidades del código G
    [tool-changes] cambios de herramienta
    [safe-Z] Z segura
    [spindle] husillo
    [max-feed] avance máx.
    [travel] recorrido
    [work-zero] cero pieza
    [max-overcut] prof. máx. bajo el material
   *[other] { $field }
}
# An angle in degrees, $degrees without its unit.
job-library-degrees = { $degrees }°
# No value.
job-library-none = ninguno
job-library-yes-no = { $value ->
    [yes] sí
    [no] no
   *[unknown] desconocido
}
# The travel of a machine: $x and $y without their unit, $z with it.
job-library-travel = X{ $x } Y{ $y } Z{ $z }
job-library-work-zero-origin = origen del dibujo
# $corner is a word of the project file: bottom-left, top-right, …
job-library-work-zero-stock = material en bruto { $corner }
# $x and $y are lengths without their unit.
job-library-work-zero-point = punto X{ $x } Y{ $y }
# A tool's cutting values: feeds and the step down with their units, the stepover a fraction.
job-library-cutting = avance { $feed }, penetración { $plunge }, prof. de pasada { $step_down }, paso lateral { $stepover }
job-library-other-values = otros valores
# A library tool put into a project: $id is its number there.
job-library-tool-there = herramienta { $id } (ya está en el proyecto)
job-library-tool-added = herramienta { $id } (añadida)
# $taken is the number the tool has in the library, taken in the project by another tool.
job-library-tool-added-as = herramienta { $id } (añadida como { $id }; el número { $taken } ya lo tenía otra herramienta)
# How a project's tool or machine stands to the library entry it was copied from. $entry
# is "tool" (a library tool) or "profile" (a machine profile); $name the entry's name.
job-link-none = no viene de tu biblioteca
job-link-missing-named = { $entry ->
    [profile] el perfil
   *[tool] la herramienta de biblioteca
} "{ $name }" no está en tu biblioteca
job-link-missing = { $entry ->
    [profile] su perfil
   *[tool] su herramienta de biblioteca
} no está en tu biblioteca
job-link-same = { $entry ->
    [profile] del perfil
   *[tool] de la herramienta de biblioteca
} "{ $name }"
# $changes lists what updating would change (job-library-change texts).
job-link-differs = difiere { $entry ->
    [profile] del perfil
   *[tool] de la herramienta de biblioteca
} "{ $name }": { $changes }

## Importing a library file

# $list is "tools" or "machines", the list in the file.
job-import-empty = El archivo no contiene { $list ->
    [machines] máquinas
   *[tools] herramientas
}.
# $names are the entries' names, separated by commas.
job-import-added = Añadidas: { $names }
job-import-new = Nuevas: { $names }
job-import-same = Ya en la biblioteca: { $names }
# $differences are job-library-change texts joined by semicolons.
job-import-replaced = Reemplazada: { $name } (archivo: { $differences })
job-import-kept = Se conserva la tuya: { $name } (archivo: { $differences })
job-import-differs = Difiere de la tuya: { $name } (archivo: { $differences })
job-import-default = Predeterminada: { $name }
job-import-becomes-default = Pasa a ser la predeterminada: { $name }
