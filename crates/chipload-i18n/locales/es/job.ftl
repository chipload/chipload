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
job-error-version = Job schema_version { $found } is not supported: this version of Chipload reads { $oldest } to { $current }. Update Chipload or check the file.
job-error-too-old = Job schema_version { $found } is from before projects and is no longer read. Open it with Chipload 0.3.0 and save it, or import its DXF again.
# $reason is why the old file can't be carried forward, in English: it names the file's fields.
job-error-cannot-migrate = This job (schema_version { $from }) can't be opened: { $reason }
# $reason is the operating system's or JSON reader's own words, or one of the job-error texts below.
job-error-unreadable = Cannot read job file { $path }: { $reason }
# $reason is the JSON reader's own words, or the project's problems joined by semicolons.
job-error-invalid = Invalid job file { $path }: { $reason }
job-error-not-object = must contain a JSON object
# $error is the operating system's words.
job-error-cannot-write = cannot write: { $error }
job-error-no-backup = cannot keep the previous version as .bak: { $error }
# On the command line, after `set`: $paths are the values set, as paths in the project file.
job-set-would-not-load = the project wouldn't load after setting { $paths }: { $reason }

## The tool library and machine profiles

# $path is the library, settings or recovery file; $found its schema_version as written there.
job-library-error-version = { $path }: schema_version { $found } is not supported: this version of Chipload reads { $current }. Update Chipload or check the file.
# $reason is the operating system's or JSON reader's own words, or a job- text.
job-library-error-unreadable = Cannot read { $path }: { $reason }
# $reason is the JSON reader's own words, or the file's rule violations (in English).
job-library-error-invalid = Invalid { $path }: { $reason }
job-library-error-unwritable = Cannot write { $path }: { $reason }
# A change the library refuses: $reason is one of the texts below, or the app's own words.
job-library-error-refused = { $reason }
job-library-no-tool = no tool named "{ $name }"
job-library-no-machine = no machine named "{ $name }"
# $entry is "tool" or "machine".
job-library-no-entry = no { $entry ->
    [machine] machine
   *[tool] tool
} named "{ $name }"
# A file to import that holds the other library: $path is the file, $hint one of the two texts after it.
job-library-other-file = { $path }: { $hint }
job-library-holds-machines = this file holds machine profiles, not tools: import it as machines
job-library-holds-tools = this file holds tools, not machine profiles: import it as tools
# A value that differs between a project and the library, or a library and a file:
# "diameter 6 mm -> 8 mm". $what is job-library-field; $from and $to its values. Values
# that are words of the project file (endmill, vbit, cw, ccw, files, program, mm, inch,
# bottom-left, …) and names are written as they are.
job-library-change = { $what } { $from } -> { $to }
job-library-field = { $field ->
    [type] type
    [diameter] diameter
    [flutes] flutes
    [centre-cutting] centre cutting
    [rpm] rpm
    [angle] angle
    [tip] tip
    [id] id
    [cutting-values] cutting values
    [post] post
    [G-code-units] G-code units
    [tool-changes] tool changes
    [safe-Z] safe Z
    [spindle] spindle
    [max-feed] max feed
    [travel] travel
    [work-zero] work zero
    [max-overcut] max overcut
   *[other] { $field }
}
# An angle in degrees, $degrees without its unit.
job-library-degrees = { $degrees }°
# No value.
job-library-none = none
job-library-yes-no = { $value ->
    [yes] yes
    [no] no
   *[unknown] unknown
}
# The travel of a machine: $x and $y without their unit, $z with it.
job-library-travel = X{ $x } Y{ $y } Z{ $z }
job-library-work-zero-origin = drawing origin
# $corner is a word of the project file: bottom-left, top-right, …
job-library-work-zero-stock = stock { $corner }
# $x and $y are lengths without their unit.
job-library-work-zero-point = point X{ $x } Y{ $y }
# A tool's cutting values: feeds and the step down with their units, the stepover a fraction.
job-library-cutting = feed { $feed }, plunge { $plunge }, step down { $step_down }, stepover { $stepover }
job-library-other-values = other values
# A library tool put into a project: $id is its number there.
job-library-tool-there = tool { $id } (already in the project)
job-library-tool-added = tool { $id } (added)
# $taken is the number the tool has in the library, taken in the project by another tool.
job-library-tool-added-as = tool { $id } (added as { $id }; tool { $taken } was taken)
# How a project's tool or machine stands to the library entry it was copied from. $entry
# is "tool" (a library tool) or "profile" (a machine profile); $name the entry's name.
job-link-none = not from your library
job-link-missing-named = { $entry ->
    [profile] profile
   *[tool] library tool
} "{ $name }" is not in your library
job-link-missing = its { $entry ->
    [profile] profile
   *[tool] library tool
} is not in your library
job-link-same = from { $entry ->
    [profile] profile
   *[tool] library tool
} "{ $name }"
# $changes lists what updating would change (job-library-change texts).
job-link-differs = differs from { $entry ->
    [profile] profile
   *[tool] library tool
} "{ $name }": { $changes }

## Importing a library file

# $list is "tools" or "machines", the list in the file.
job-import-empty = The file holds no { $list ->
    [machines] machines
   *[tools] tools
}.
# $names are the entries' names, separated by commas.
job-import-added = Added: { $names }
job-import-new = New: { $names }
job-import-same = Already in the library: { $names }
# $differences are job-library-change texts joined by semicolons.
job-import-replaced = Replaced: { $name } (file: { $differences })
job-import-kept = Kept yours: { $name } (file: { $differences })
job-import-differs = Differs from yours: { $name } (file: { $differences })
job-import-default = Default: { $name }
job-import-becomes-default = Becomes the default: { $name }
