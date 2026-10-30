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
job-operation-label = Operation { $number }
job-operation-label-named = Operation { $number } ({ $name })
# A rest pass of a pocket: $number is the pocket's and the pass's, as "3.1".
job-operation-label-rest = Operation { $number } (restbearbetning)
job-operation-label-named-rest = Operation { $number } ({ $name }, restbearbetning)
job-operation-label-floor = Operation { $number } (botten)
job-operation-label-named-floor = Operation { $number } ({ $name }, botten)
# The kinds of tools, alone and with their article, inside other messages.
job-tool-type = { $type ->
    [drill] borr
    [vbit] V-fräs
   *[endmill] pinnfräs
}
job-tool-type-article = { $type ->
    [drill] ett borr
    [vbit] en V-fräs
   *[endmill] en pinnfräs
}

## Rules for any value

job-must-be-positive = måste vara större än 0, fick { $got }
job-must-be-positive-number = måste vara ett positivt tal, fick { $got }
job-must-be-more-than-0 = måste vara mer än 0, fick { $got }
job-must-be-0-or-more = måste vara 0 eller mer, fick { $got }
job-through-stale = skär igenom materialet och slutar därför på { $depth }
job-stock-fit-needs-centred = ett ämne som anpassas efter ritningen måste vara centrerat på den (stock.origin: drawing)
job-must-be-1-or-more = måste vara 1 eller mer
job-must-be-at-least-1 = måste vara minst 1
job-must-not-be-empty = får inte vara tom
job-must-be-finite = måste vara ett ändligt tal
# $got is an angle in radians, without a unit.
job-must-be-finite-got = måste vara ett ändligt tal, fick { $got }
# $got is a value that isn't a number (NaN, inf).
job-must-be-number = måste vara ett tal, fick { $got }
# Two lengths without their unit.
job-must-be-numbers = måste vara tal, fick ({ $x }, { $y })
job-xy-must-be-numbers = x och y måste vara tal, fick ({ $x }, { $y })
# $id is a number given to two things of the same kind.
job-id-twice = { $id } används två gånger
# Unitless numbers.
job-more-than-0-at-most = måste vara mer än 0 och högst { $most }, fick { $got }
job-rotation-range = måste vara minst 0 och mindre än 360 grader, fick { $got }
job-angle-below-180 = måste vara mer än 0 och mindre än 180 grader, fick { $got } grader
job-ramp-angle-range = måste vara mer än 0 och högst 30 grader, fick { $got } grader
job-flank-angle-range = måste vara mer än 0 och högst 90 grader, fick { $got } grader

## Copies in a grid

job-copies-per-side = måste vara från 1 till { $most }, fick { $got }
job-copies-too-many = högst { $most } kopior totalt, fick { $got }
job-copies-pitch-zero = får inte vara 0 med mer än en kopia i den riktningen

## Pictures and texts

# A picture setting's range in pixels; numbers without unit.
job-image-range-px = måste vara { $low } till { $high } px, fick { $got } px
# A picture setting's range as an area in square millimetres; numbers without unit.
job-image-range-area = måste vara { $low } till { $high } mm², fick { $got } mm²
# A length's range: $low is a number alone, $high and $got have their unit.
job-image-range-length = måste vara { $low } till { $high }, fick { $got }
# "auto" is written in the project file as it is.
job-image-threshold = måste vara 1 till 254 eller "auto", fick { $got }
job-image-missing = bilden saknas
job-text-empty = är tom: skriv något att fräsa
job-text-height = måste vara mer än 0 och högst { $most }, fick { $got }
# $heights is a number of text heights, $most that as a length.
job-text-spacing = måste vara högst { $heights } teckenhöjder åt båda hållen ({ $most }), fick { $got }

## The drawing

# next_shape_id is a field of the project file.
job-shape-id-range = måste vara mellan 1 och next_shape_id - 1 ({ $most }), fick { $got }
# drawing.images, drawing.dxfs and drawing.texts are places in the project file; $id is
# a number.
job-no-image = ingen bild { $id } i drawing.images
job-no-dxf = ingen DXF { $id } i drawing.dxfs
job-no-text = ingen text { $id } i drawing.texts
job-shape-one-source = en form kommer från en bild, DXF eller text, inte fler
job-path-no-segments = en form behöver minst ett segment
job-segment-not-finite = koordinater måste vara ändliga och radier positiva
job-corner-radius-tiny = 0 för skarpa hörn, eller minst { $least }, fick { $got }
job-corner-radius-too-large = högst { $most }, halva den kortare sidan, fick { $got }
# $closed is "closed" or "open".
job-polyline-points = { $closed ->
    [closed] en sluten
   *[open] en öppen
} polylinje behöver minst { $least } punkter, fick { $got }
# $tolerance is a length.
job-point-on-previous = ligger ovanpå föregående punkt (närmare än { $tolerance })
job-bulges-count = en per segment, { $wanted } för denna polylinje, fick { $got }
job-bulge-range = måste vara mellan -1 och 1 (högst en halvcirkel), fick { $got }

## Machine and tools

# Two lengths without their unit.
job-work-zero-not-finite = x och y måste vara ändliga, fick { $x }, { $y }
job-tip-diameter = måste vara 0 eller mer och mindre än diametern ({ $diameter }), fick { $got }

## Parts

job-no-parts = ett projekt behöver minst en detalj
job-part-twice = detalj { $id } finns med två gånger
job-part-name-taken = en annan detalj heter "{ $name }"
# "parts" is a place in the project file.
job-no-part = ingen detalj { $id } i parts
job-part-order = alla operationer i detalj { $part } måste komma före de i detalj { $before }

## Placement and depths

job-scale-range = måste vara { $low } till { $high }, fick { $got }
# end_depth is a field of the project file.
job-start-below-end = måste vara mindre än end_depth ({ $end }), fick { $got }
job-on-line-clearance = en konturfräsning på linjen har ingen sida att fräsa mer på; den måste vara 0, fick { $got }

## Surfacing

# Fractions of the tool diameter.
job-surface-stepover = måste vara från { $low } till { $high } av verktygsdiametern, fick { $got }
job-surface-too-deep = en planfräsning kan inte gå under ämnets underkant ({ $thickness } tjockt), fick { $got }

## Which shapes an operation cuts

job-selector-empty = urvalet behöver en DXF, en bild, en text, ett lager eller former
job-exclude-no-source = former kan bara uteslutas ur en hel DXF, bild eller text
# $source is "dxf", "image" or "text"; $id its number.
job-exclude-missing = form { $shape } finns inte längre i { $source ->
    [dxf] DXF
    [image] bild
   *[text] text
} { $id }: välj formerna som ska uteslutas igen
job-selector-one-source = ett urval anger en DXF, bild eller text, inte fler
# The part in braces is written in the project file as it is; $text is a number.
job-select-letter = form { $shape } är en bokstav i en text och ändras varje gång texten skapas på nytt: välj texten i stället ({"{"}"text": { $text }{"}"})
job-select-traced = form { $shape } är spårad från en bild och ändras varje gång bilden spåras på nytt: välj bilden i stället ({"{"}"image": { $image }{"}"})
job-select-centre-lines = bild { $image } är spårad som öppna mittlinjer: fräs dem med Följ bana, eller spåra bilden som konturer

## Operations

job-on-line-corners = en konturfräsning på linjen har inga innerhörn att fräsa ur
job-tbone-wall = en T-bone behöver väggen den går längs: den längre eller den kortare
job-wall-only-tbone = bara en T-bone har en vägg att gå längs
job-corner-chosen-twice = två val för hörnet vid { $at }
job-starts-not-finite = varje startpunkt behöver ändliga x och y
job-stepover-at-most-1 = måste vara högst 1 (en andel av verktygsdiametern), fick { $got }
job-on-line-finish = en konturfräsning på linjen har ingen vägg att finbearbeta; använd inuti eller utanför
# Lengths without their unit.
job-skin-above-step-down = måste vara högst skärdjupet ({ $step_down }), fick { $got }
job-skin-above-cut = måste vara mindre än fräsdjupet ({ $cut }, slutdjup minus startdjup), fick { $got }
job-skin-above-tabs = måste vara mindre än bryggornas höjd ({ $height }), fick { $got }

## Lists

# Messages joined: "a; b; c" and "a, b, c".
job-semicolons = { $list }; { $last }
job-commas = { $list }, { $last }

## Project files that can't be read

# $found is the schema_version in the file, as written there ("missing" when there is none).
job-error-version = Jobbets schema_version { $found } stöds inte: den här versionen av Chipload läser { $oldest } till { $current }. Uppdatera Chipload eller kontrollera filen.
job-error-too-old = Jobbets schema_version { $found } är från tiden före projekt och läses inte längre. Öppna det med Chipload 0.3.0 och spara det, eller importera dess DXF igen.
# $reason is why the old file can't be carried forward, in English: it names the file's fields.
job-error-cannot-migrate = Det här jobbet (schema_version { $from }) kan inte öppnas: { $reason }
# $reason is the operating system's or JSON reader's own words, or one of the job-error texts below.
job-error-unreadable = Kan inte läsa jobbfilen { $path }: { $reason }
# $reason is the JSON reader's own words, or the project's problems joined by semicolons.
job-error-invalid = Ogiltig jobbfil { $path }: { $reason }
job-error-not-object = måste innehålla ett JSON-objekt
# $error is the operating system's words.
job-error-cannot-write = kan inte skriva: { $error }
job-error-no-backup = kan inte behålla den tidigare versionen som .bak: { $error }
# On the command line, after `set`: $paths are the values set, as paths in the project file.
job-set-would-not-load = projektet skulle inte gå att läsa in efter att { $paths } satts: { $reason }

## The tool library and machine profiles

# $path is the library, settings or recovery file; $found its schema_version as written there.
job-library-error-version = { $path }: schema_version { $found } stöds inte: den här versionen av Chipload läser { $current }. Uppdatera Chipload eller kontrollera filen.
# $reason is the operating system's or JSON reader's own words, or a job- text.
job-library-error-unreadable = Kan inte läsa { $path }: { $reason }
# $reason is the JSON reader's own words, or the file's rule violations (in English).
job-library-error-invalid = Ogiltig fil { $path }: { $reason }
job-library-error-unwritable = Kan inte skriva { $path }: { $reason }
# A change the library refuses: $reason is one of the texts below, or the app's own words.
job-library-error-refused = { $reason }
job-library-no-tool = det finns inget verktyg som heter "{ $name }"
job-library-no-machine = det finns ingen maskin som heter "{ $name }"
job-library-no-material = det finns inget material som heter "{ $name }"
job-library-material-exists = det finns redan ett material som heter "{ $name }"
# $entry is "tool" or "machine".
job-library-no-entry = det finns { $entry ->
    [machine] ingen maskin
   *[tool] inget verktyg
} som heter "{ $name }"
# A file to import that holds the other library: $path is the file, $hint one of the two texts after it.
job-library-other-file = { $path }: { $hint }
job-library-holds-machines = den här filen innehåller maskinprofiler, inte verktyg: importera den som maskiner
job-library-holds-tools = den här filen innehåller verktyg, inte maskinprofiler: importera den som verktyg
# A value that differs between a project and the library, or a library and a file:
# "diameter 6 mm -> 8 mm". $what is job-library-field; $from and $to its values. Values
# that are words of the project file (endmill, vbit, cw, ccw, files, program, mm, inch,
# bottom-left, …) and names are written as they are.
job-library-change = { $what } { $from } -> { $to }
job-library-field = { $field ->
    [type] typ
    [diameter] diameter
    [flutes] skär
    [centre-cutting] centrumskärande
    [rpm] varvtal
    [angle] vinkel
    [tip] spets
    [id] nummer
    [cutting-values] skärdata
    [post] postprocessor
    [G-code-units] G-kodens enheter
    [tool-changes] verktygsbyten
    [safe-Z] säker Z
    [spindle] spindel
    [max-feed] max matning
    [travel] slaglängd
    [work-zero] arbetsnollpunkt
    [max-overcut] max djup under ämnet
    [tool-values] ett verktygs värden
   *[other] { $field }
}
# An angle in degrees, $degrees without its unit.
job-library-degrees = { $degrees }°
# No value.
job-library-none = ingen
job-library-yes-no = { $value ->
    [yes] ja
    [no] nej
   *[unknown] okänt
}
# The travel of a machine: $x and $y without their unit, $z with it.
job-library-travel = X{ $x } Y{ $y } Z{ $z }
job-library-work-zero-origin = ritningens origo
# $corner is a word of the project file: bottom-left, top-right, …
job-library-work-zero-stock = ämnet { $corner }
# $x and $y are lengths without their unit.
job-library-work-zero-point = punkten X{ $x } Y{ $y }
# A tool's cutting values: feeds and the step down with their units, the stepover a fraction.
job-library-cutting = matning { $feed }, nedmatning { $plunge }, skärdjup { $step_down }, sidosteg { $stepover }
job-library-other-values = andra värden
# A library tool put into a project: $id is its number there.
job-library-tool-there = verktyg { $id } (finns redan i projektet)
job-library-tool-added = verktyg { $id } (tillagt)
# $taken is the number the tool has in the library, taken in the project by another tool.
job-library-tool-added-as = verktyg { $id } (tillagt som { $id }; verktyg { $taken } var upptaget)
# How a project's tool or machine stands to the library entry it was copied from. $entry
# is "tool" (a library tool) or "profile" (a machine profile); $name the entry's name.
job-link-none = inte från ditt bibliotek
job-link-missing-named = { $entry ->
    [profile] profilen
   *[tool] biblioteksverktyget
} "{ $name }" finns inte i ditt bibliotek
job-link-missing = { $entry ->
    [profile] dess profil
   *[tool] dess biblioteksverktyg
} finns inte i ditt bibliotek
job-link-same = från { $entry ->
    [profile] profilen
   *[tool] biblioteksverktyget
} "{ $name }"
# $changes lists what updating would change (job-library-change texts).
job-link-differs = skiljer sig från { $entry ->
    [profile] profilen
   *[tool] biblioteksverktyget
} "{ $name }": { $changes }

## Importing a library file

# $list is "tools" or "machines", the list in the file.
job-import-empty = Filen innehåller inga { $list ->
    [machines] maskiner
   *[tools] verktyg
}.
# $names are the entries' names, separated by commas.
job-import-added = Tillagda: { $names }
job-import-new = Nya: { $names }
job-import-same = Finns redan i biblioteket: { $names }
# $differences are job-library-change texts joined by semicolons.
job-import-replaced = Ersatt: { $name } (filen: { $differences })
job-import-kept = Behöll din: { $name } (filen: { $differences })
job-import-differs = Skiljer sig från din: { $name } (filen: { $differences })
job-import-default = Standard: { $name }
job-import-becomes-default = Blir standard: { $name }
job-polygon-sides = en polygon har { $least } till { $most } sidor, fick { $got }
