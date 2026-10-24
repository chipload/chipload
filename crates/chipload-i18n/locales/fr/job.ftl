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
job-problem-at = { $path } : { $problem }
# A problem of an operation after its label ("Operation 3 (Outline): …").
job-operation-problem = { $label } : { $problem }
# A shape's problem: $at is its place (drawing.shapes.4), $id its number (S7), $problem
# the problem with its field ("radius: must be …").
job-shape-problem = { $at } (S{ $id }).{ $problem }
# Operations by their number, and with the name the user gave them.
job-operation-label = Opération { $number }
job-operation-label-named = Opération { $number } ({ $name })
# A rest pass of a pocket: $number is the pocket's and the pass's, as "3.1".
job-operation-label-rest = Opération { $number } (reprise)
job-operation-label-named-rest = Opération { $number } ({ $name }, reprise)
# The kinds of tools, alone and with their article, inside other messages.
job-tool-type = { $type ->
    [drill] foret
    [vbit] fraise en V
   *[endmill] fraise
}
job-tool-type-article = { $type ->
    [drill] un foret
    [vbit] une fraise en V
   *[endmill] une fraise
}

## Rules for any value

job-must-be-positive = doit être supérieur à 0, reçu { $got }
job-must-be-positive-number = doit être un nombre positif, reçu { $got }
job-must-be-more-than-0 = doit être plus grand que 0, reçu { $got }
job-must-be-0-or-more = doit être 0 ou plus, reçu { $got }
job-must-be-1-or-more = doit être 1 ou plus
job-must-be-at-least-1 = doit être au moins 1
job-must-not-be-empty = ne doit pas être vide
job-must-be-finite = doit être fini
# $got is an angle in radians, without a unit.
job-must-be-finite-got = doit être fini, reçu { $got }
# $got is a value that isn't a number (NaN, inf).
job-must-be-number = doit être un nombre, reçu { $got }
# Two lengths without their unit.
job-must-be-numbers = doivent être des nombres, reçu ({ $x }, { $y })
job-xy-must-be-numbers = x et y doivent être des nombres, reçu ({ $x }, { $y })
# $id is a number given to two things of the same kind.
job-id-twice = { $id } est utilisé deux fois
# Unitless numbers.
job-more-than-0-at-most = doit être plus grand que 0 et au plus { $most }, reçu { $got }
job-rotation-range = doit être au moins 0 et moins de 360 degrés, reçu { $got }
job-angle-below-180 = doit être plus grand que 0 et moins de 180 degrés, reçu { $got } degrés
job-ramp-angle-range = doit être plus grand que 0 et au plus 30 degrés, reçu { $got } degrés
job-flank-angle-range = doit être plus grand que 0 et au plus 90 degrés, reçu { $got } degrés

## Copies in a grid

job-copies-per-side = doit aller de 1 à { $most }, reçu { $got }
job-copies-too-many = au plus { $most } copies en tout, reçu { $got }
job-copies-pitch-zero = ne doit pas être 0 avec plus d'une copie dans ce sens

## Pictures and texts

# A picture setting's range in pixels; numbers without unit.
job-image-range-px = doit aller de { $low } à { $high } px, reçu { $got } px
# A picture setting's range as an area in square millimetres; numbers without unit.
job-image-range-area = doit aller de { $low } à { $high } mm², reçu { $got } mm²
# A length's range: $low is a number alone, $high and $got have their unit.
job-image-range-length = doit aller de { $low } à { $high }, reçu { $got }
# "auto" is written in the project file as it is.
job-image-threshold = doit aller de 1 à 254 ou être "auto", reçu { $got }
job-image-missing = l'image est manquante
job-text-empty = est vide : écrivez quelque chose à usiner
job-text-height = doit être plus grande que 0 et au plus { $most }, reçu { $got }
# $heights is a number of text heights, $most that as a length.
job-text-spacing = doit être au plus { $heights } hauteurs dans un sens ou l'autre ({ $most }), reçu { $got }

## The drawing

# next_shape_id is a field of the project file.
job-shape-id-range = doit être entre 1 et next_shape_id - 1 ({ $most }), reçu { $got }
# drawing.images, drawing.dxfs and drawing.texts are places in the project file; $id is
# a number.
job-no-image = aucune image { $id } dans drawing.images
job-no-dxf = aucun DXF { $id } dans drawing.dxfs
job-no-text = aucun texte { $id } dans drawing.texts
job-shape-one-source = une forme vient d'une seule image, d'un seul DXF ou d'un seul texte, pas de plusieurs
job-path-no-segments = une forme a besoin d'au moins un segment
job-segment-not-finite = les coordonnées doivent être finies et les rayons positifs
job-corner-radius-tiny = 0 pour des coins vifs, ou au moins { $least }, reçu { $got }
job-corner-radius-too-large = au plus { $most }, la moitié du plus petit côté, reçu { $got }
# $closed is "closed" or "open".
job-polyline-points = { $closed ->
    [closed] une polyligne fermée
   *[open] une polyligne ouverte
} a besoin d'au moins { $least } points, reçu { $got }
# $tolerance is a length.
job-point-on-previous = est sur le point précédent (à moins de { $tolerance })
job-bulges-count = un par segment, { $wanted } pour cette polyligne, reçu { $got }
job-bulge-range = doit être entre -1 et 1 (au plus un demi-cercle), reçu { $got }

## Machine and tools

# Two lengths without their unit.
job-work-zero-not-finite = x et y doivent être finis, reçu { $x }, { $y }
job-tip-diameter = doit être 0 ou plus et inférieur au diamètre ({ $diameter }), reçu { $got }

## Parts

job-no-parts = un projet a besoin d'au moins une pièce
job-part-twice = la pièce { $id } est listée deux fois
job-part-name-taken = une autre pièce s'appelle « { $name } »
# "parts" is a place in the project file.
job-no-part = aucune pièce { $id } dans parts
job-part-order = les opérations de la pièce { $part } doivent toutes venir avant celles de la pièce { $before }

## Placement and depths

job-scale-range = doit aller de { $low } à { $high }, reçu { $got }
# end_depth is a field of the project file.
job-start-below-end = doit être inférieure à end_depth ({ $end }), reçu { $got }
job-on-line-clearance = un contournage sur la ligne n'a pas de côté vers lequel usiner plus ; doit être 0, reçu { $got }

## Surfacing

# Fractions of the tool diameter.
job-surface-stepover = doit aller de { $low } à { $high } du diamètre de l'outil, reçu { $got }
job-surface-too-deep = un surfaçage ne peut pas descendre sous le dessous du brut ({ $thickness } d'épaisseur), reçu { $got }

## Which shapes an operation cuts

job-selector-empty = la sélection a besoin d'un DXF, d'une image, d'un texte, d'un calque ou de formes
job-exclude-no-source = on ne peut exclure des formes que d'un DXF, d'une image ou d'un texte entier
# $source is "dxf", "image" or "text"; $id its number.
job-exclude-missing = la forme { $shape } n'est plus dans { $source ->
    [dxf] le DXF
    [image] l'image
   *[text] le texte
} { $id } : choisissez à nouveau les formes à exclure
job-selector-one-source = une sélection désigne un seul DXF, une seule image ou un seul texte, pas plusieurs
# The part in braces is written in the project file as it is; $text is a number.
job-select-letter = la forme { $shape } est une lettre d'un texte et change chaque fois que le texte est refait : sélectionnez plutôt le texte ({"{"}"text": { $text }{"}"})
job-select-traced = la forme { $shape } est vectorisée depuis une image et change chaque fois que l'image est revectorisée : sélectionnez plutôt l'image ({"{"}"image": { $image }{"}"})
job-select-centre-lines = l'image { $image } est vectorisée en lignes médianes ouvertes : usinez-les avec un suivi de tracé, ou vectorisez l'image en contours

## Operations

job-on-line-corners = un contournage sur la ligne n'a pas de coins intérieurs à dégager
job-starts-not-finite = chaque point de départ a besoin de x et y finis
job-stepover-at-most-1 = doit être au plus 1 (une fraction du diamètre de l'outil), reçu { $got }
job-on-line-finish = un contournage sur la ligne n'a pas de paroi à finir ; utilisez intérieur ou extérieur
# Lengths without their unit.
job-skin-above-step-down = doit être au plus la profondeur de passe ({ $step_down }), reçu { $got }
job-skin-above-cut = doit être inférieure à la coupe ({ $cut }, profondeur finale moins profondeur de départ), reçu { $got }
job-skin-above-tabs = doit être inférieure à la hauteur des attaches ({ $height }), reçu { $got }

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
