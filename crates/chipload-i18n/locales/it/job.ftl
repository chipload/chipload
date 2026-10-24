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
job-operation-label = Lavorazione { $number }
job-operation-label-named = Lavorazione { $number } ({ $name })
# A rest pass of a pocket: $number is the pocket's and the pass's, as "3.1".
job-operation-label-rest = Lavorazione { $number } (ripresa)
job-operation-label-named-rest = Lavorazione { $number } ({ $name }, ripresa)
# The kinds of tools, alone and with their article, inside other messages.
job-tool-type = { $type ->
    [drill] punta
    [vbit] fresa a V
   *[endmill] fresa
}
job-tool-type-article = { $type ->
    [drill] una punta
    [vbit] una fresa a V
   *[endmill] una fresa
}

## Rules for any value

job-must-be-positive = deve essere maggiore di 0, trovato { $got }
job-must-be-positive-number = deve essere un numero positivo, trovato { $got }
job-must-be-more-than-0 = deve essere più di 0, trovato { $got }
job-must-be-0-or-more = deve essere 0 o più, trovato { $got }
job-must-be-1-or-more = deve essere 1 o più
job-must-be-at-least-1 = deve essere almeno 1
job-must-not-be-empty = non deve essere vuoto
job-must-be-finite = deve essere finito
# $got is an angle in radians, without a unit.
job-must-be-finite-got = deve essere finito, trovato { $got }
# $got is a value that isn't a number (NaN, inf).
job-must-be-number = deve essere un numero, trovato { $got }
# Two lengths without their unit.
job-must-be-numbers = devono essere numeri, trovato ({ $x }, { $y })
job-xy-must-be-numbers = x e y devono essere numeri, trovato ({ $x }, { $y })
# $id is a number given to two things of the same kind.
job-id-twice = { $id } è usato due volte
# Unitless numbers.
job-more-than-0-at-most = deve essere più di 0 e al massimo { $most }, trovato { $got }
job-rotation-range = deve essere almeno 0 e meno di 360 gradi, trovato { $got }
job-angle-below-180 = deve essere più di 0 e meno di 180 gradi, trovato { $got } gradi
job-ramp-angle-range = deve essere più di 0 e al massimo 30 gradi, trovato { $got } gradi
job-flank-angle-range = deve essere più di 0 e al massimo 90 gradi, trovato { $got } gradi

## Copies in a grid

job-copies-per-side = deve essere da 1 a { $most }, trovato { $got }
job-copies-too-many = al massimo { $most } copie in tutto, trovato { $got }
job-copies-pitch-zero = non deve essere 0 con più di una copia in quella direzione

## Pictures and texts

# A picture setting's range in pixels; numbers without unit.
job-image-range-px = deve essere da { $low } a { $high } px, trovato { $got } px
# A picture setting's range as an area in square millimetres; numbers without unit.
job-image-range-area = deve essere da { $low } a { $high } mm², trovato { $got } mm²
# A length's range: $low is a number alone, $high and $got have their unit.
job-image-range-length = deve essere da { $low } a { $high }, trovato { $got }
# "auto" is written in the project file as it is.
job-image-threshold = deve essere da 1 a 254 o "auto", trovato { $got }
job-image-missing = l'immagine manca
job-text-empty = è vuoto: scrivi qualcosa da tagliare
job-text-height = deve essere più di 0 e al massimo { $most }, trovato { $got }
# $heights is a number of text heights, $most that as a length.
job-text-spacing = deve essere al massimo { $heights } altezze in entrambi i sensi ({ $most }), trovato { $got }

## The drawing

# next_shape_id is a field of the project file.
job-shape-id-range = deve essere tra 1 e next_shape_id - 1 ({ $most }), trovato { $got }
# drawing.images, drawing.dxfs and drawing.texts are places in the project file; $id is
# a number.
job-no-image = nessuna immagine { $id } in drawing.images
job-no-dxf = nessun DXF { $id } in drawing.dxfs
job-no-text = nessun testo { $id } in drawing.texts
job-shape-one-source = una forma viene da una sola immagine, un solo DXF o un solo testo, non di più
job-path-no-segments = una forma richiede almeno un segmento
job-segment-not-finite = le coordinate devono essere finite e i raggi positivi
job-corner-radius-tiny = 0 per angoli vivi, o almeno { $least }, trovato { $got }
job-corner-radius-too-large = al massimo { $most }, metà del lato più corto, trovato { $got }
# $closed is "closed" or "open".
job-polyline-points = { $closed ->
    [closed] una polilinea chiusa
   *[open] una polilinea aperta
} richiede almeno { $least } punti, trovato { $got }
# $tolerance is a length.
job-point-on-previous = coincide con il punto precedente (più vicino di { $tolerance })
job-bulges-count = uno per segmento, { $wanted } per questa polilinea, trovato { $got }
job-bulge-range = deve essere tra -1 e 1 (al massimo mezzo cerchio), trovato { $got }

## Machine and tools

# Two lengths without their unit.
job-work-zero-not-finite = x e y devono essere finiti, trovato { $x }, { $y }
job-tip-diameter = deve essere 0 o più e minore del diametro ({ $diameter }), trovato { $got }

## Parts

job-no-parts = un progetto richiede almeno un pezzo
job-part-twice = il pezzo { $id } è elencato due volte
job-part-name-taken = un altro pezzo si chiama "{ $name }"
# "parts" is a place in the project file.
job-no-part = nessun pezzo { $id } in parts
job-part-order = tutte le lavorazioni del pezzo { $part } devono venire prima di quelle del pezzo { $before }

## Placement and depths

job-scale-range = deve essere da { $low } a { $high }, trovato { $got }
# end_depth is a field of the project file.
job-start-below-end = deve essere minore di end_depth ({ $end }), trovato { $got }
job-on-line-clearance = una contornatura sulla linea non ha un lato su cui tagliare di più; deve essere 0, trovato { $got }

## Surfacing

# Fractions of the tool diameter.
job-surface-stepover = deve essere da { $low } a { $high } del diametro dell'utensile, trovato { $got }
job-surface-too-deep = una spianatura non può scendere sotto il fondo del grezzo (spessore { $thickness }), trovato { $got }

## Which shapes an operation cuts

job-selector-empty = il selettore richiede un DXF, un'immagine, un testo, un layer o delle forme
job-exclude-no-source = si possono escludere forme solo da un intero DXF, immagine o testo
# $source is "dxf", "image" or "text"; $id its number.
job-exclude-missing = la forma { $shape } non è più { $source ->
    [dxf] nel DXF
    [image] nell'immagine
   *[text] nel testo
} { $id }: scegli di nuovo le forme da escludere
job-selector-one-source = un selettore indica un solo DXF, immagine o testo, non di più
# The part in braces is written in the project file as it is; $text is a number.
job-select-letter = la forma { $shape } è una lettera di un testo e cambia ogni volta che il testo viene rigenerato: seleziona invece il testo ({"{"}"text": { $text }{"}"})
job-select-traced = la forma { $shape } è vettorizzata da un'immagine e cambia ogni volta che l'immagine viene vettorizzata di nuovo: seleziona invece l'immagine ({"{"}"image": { $image }{"}"})
job-select-centre-lines = l'immagine { $image } è vettorizzata come linee mediane aperte: tagliale con un segui percorso, o vettorizza l'immagine come contorni

## Operations

job-on-line-corners = una contornatura sulla linea non ha angoli interni da liberare
job-starts-not-finite = ogni punto di partenza richiede x e y finiti
job-stepover-at-most-1 = deve essere al massimo 1 (una frazione del diametro dell'utensile), trovato { $got }
job-on-line-finish = una contornatura sulla linea non ha una parete da rifinire; usa interno o esterno
# Lengths without their unit.
job-skin-above-step-down = deve essere al massimo la profondità di passata ({ $step_down }), trovato { $got }
job-skin-above-cut = deve essere minore del taglio ({ $cut }, profondità finale meno profondità iniziale), trovato { $got }
job-skin-above-tabs = deve essere minore dell'altezza dei ponticelli ({ $height }), trovato { $got }

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
