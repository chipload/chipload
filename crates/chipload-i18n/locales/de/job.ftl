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
job-operation-label = Bearbeitung { $number }
job-operation-label-named = Bearbeitung { $number } ({ $name })
# A rest pass of a pocket: $number is the pocket's and the pass's, as "3.1".
job-operation-label-rest = Bearbeitung { $number } (Rest)
job-operation-label-named-rest = Bearbeitung { $number } ({ $name }, Rest)
# The kinds of tools, alone and with their article, inside other messages.
job-tool-type = { $type ->
    [drill] Bohrer
    [vbit] V-Fräser
   *[endmill] Schaftfräser
}
job-tool-type-article = { $type ->
    [drill] ein Bohrer
    [vbit] ein V-Fräser
   *[endmill] ein Schaftfräser
}

## Rules for any value

job-must-be-positive = muss größer als 0 sein, ist { $got }
job-must-be-positive-number = muss eine positive Zahl sein, ist { $got }
job-must-be-more-than-0 = muss mehr als 0 sein, ist { $got }
job-must-be-0-or-more = muss 0 oder mehr sein, ist { $got }
job-through-stale = schneidet durch das Material, endet also bei { $depth }
job-stock-fit-needs-centred = ein Rohteil, das sich der Zeichnung anpasst, muss mittig auf ihr liegen (stock.origin: drawing)
job-must-be-1-or-more = muss 1 oder mehr sein
job-must-be-at-least-1 = muss mindestens 1 sein
job-must-not-be-empty = darf nicht leer sein
job-must-be-finite = muss endlich sein
# $got is an angle in radians, without a unit.
job-must-be-finite-got = muss endlich sein, ist { $got }
# $got is a value that isn't a number (NaN, inf).
job-must-be-number = muss eine Zahl sein, ist { $got }
# Two lengths without their unit.
job-must-be-numbers = müssen Zahlen sein, sind ({ $x }, { $y })
job-xy-must-be-numbers = x und y müssen Zahlen sein, sind ({ $x }, { $y })
# $id is a number given to two things of the same kind.
job-id-twice = { $id } wird zweimal verwendet
# Unitless numbers.
job-more-than-0-at-most = muss mehr als 0 und höchstens { $most } sein, ist { $got }
job-rotation-range = muss mindestens 0 und kleiner als 360 Grad sein, ist { $got }
job-angle-below-180 = muss mehr als 0 und weniger als 180 Grad sein, ist { $got } Grad
job-ramp-angle-range = muss mehr als 0 und höchstens 30 Grad sein, ist { $got } Grad
job-flank-angle-range = muss mehr als 0 und höchstens 90 Grad sein, ist { $got } Grad

## Copies in a grid

job-copies-per-side = muss von 1 bis { $most } sein, ist { $got }
job-copies-too-many = höchstens { $most } Kopien insgesamt, sind { $got }
job-copies-pitch-zero = darf nicht 0 sein bei mehr als einer Kopie in diese Richtung

## Pictures and texts

# A picture setting's range in pixels; numbers without unit.
job-image-range-px = muss { $low } bis { $high } px sein, ist { $got } px
# A picture setting's range as an area in square millimetres; numbers without unit.
job-image-range-area = muss { $low } bis { $high } mm² sein, ist { $got } mm²
# A length's range: $low is a number alone, $high and $got have their unit.
job-image-range-length = muss { $low } bis { $high } sein, ist { $got }
# "auto" is written in the project file as it is.
job-image-threshold = muss 1 bis 254 oder "auto" sein, ist { $got }
job-image-missing = das Bild fehlt
job-text-empty = ist leer: schreiben Sie etwas zum Fräsen
job-text-height = muss mehr als 0 und höchstens { $most } sein, ist { $got }
# $heights is a number of text heights, $most that as a length.
job-text-spacing = muss in beide Richtungen höchstens { $heights } Höhen sein ({ $most }), ist { $got }

## The drawing

# next_shape_id is a field of the project file.
job-shape-id-range = muss zwischen 1 und next_shape_id - 1 ({ $most }) liegen, ist { $got }
# drawing.images, drawing.dxfs and drawing.texts are places in the project file; $id is
# a number.
job-no-image = kein Bild { $id } in drawing.images
job-no-dxf = keine DXF { $id } in drawing.dxfs
job-no-text = kein Text { $id } in drawing.texts
job-shape-one-source = eine Form stammt aus einem Bild, einer DXF oder einem Text, nicht aus mehreren
job-path-no-segments = eine Form braucht mindestens ein Segment
job-segment-not-finite = Koordinaten müssen endlich und Radien positiv sein
job-corner-radius-tiny = 0 für scharfe Ecken, oder mindestens { $least }, ist { $got }
job-corner-radius-too-large = höchstens { $most }, die Hälfte der kürzeren Seite, ist { $got }
# $closed is "closed" or "open".
job-polyline-points = { $closed ->
    [closed] eine geschlossene
   *[open] eine offene
} Polylinie braucht mindestens { $least } Punkte, hat { $got }
# $tolerance is a length.
job-point-on-previous = liegt auf dem vorigen Punkt (näher als { $tolerance })
job-bulges-count = eine je Segment, { $wanted } für diese Polylinie, sind { $got }
job-bulge-range = muss zwischen -1 und 1 liegen (höchstens ein Halbkreis), ist { $got }

## Machine and tools

# Two lengths without their unit.
job-work-zero-not-finite = x und y müssen endlich sein, sind { $x }, { $y }
job-tip-diameter = muss 0 oder mehr und kleiner als der Durchmesser ({ $diameter }) sein, ist { $got }

## Parts

job-no-parts = ein Projekt braucht mindestens ein Teil
job-part-twice = Teil { $id } ist zweimal aufgeführt
job-part-name-taken = ein anderes Teil heißt bereits "{ $name }"
# "parts" is a place in the project file.
job-no-part = kein Teil { $id } in parts
job-part-order = die Bearbeitungen von Teil { $part } müssen alle vor denen von Teil { $before } kommen

## Placement and depths

job-scale-range = muss { $low } bis { $high } sein, ist { $got }
# end_depth is a field of the project file.
job-start-below-end = muss kleiner als end_depth ({ $end }) sein, ist { $got }
job-on-line-clearance = eine Kontur auf der Linie hat keine Seite für ein Übermaß; es muss 0 sein, ist { $got }

## Surfacing

# Fractions of the tool diameter.
job-surface-stepover = muss { $low } bis { $high } des Werkzeugdurchmessers sein, ist { $got }
job-surface-too-deep = Planfräsen darf nicht unter die Rohteilunterkante gehen ({ $thickness } dick), ist { $got }

## Which shapes an operation cuts

job-selector-empty = die Auswahl braucht eine DXF, ein Bild, einen Text, einen Layer oder Formen
job-exclude-no-source = Formen können nur aus einer ganzen DXF, einem ganzen Bild oder Text ausgelassen werden
# $source is "dxf", "image" or "text"; $id its number.
job-exclude-missing = Form { $shape } ist nicht mehr in { $source ->
    [dxf] DXF
    [image] Bild
   *[text] Text
} { $id }: wählen Sie die auszulassenden Formen neu
job-selector-one-source = eine Auswahl nennt eine DXF, ein Bild oder einen Text, nicht mehrere
# The part in braces is written in the project file as it is; $text is a number.
job-select-letter = Form { $shape } ist ein Buchstabe eines Textes und ändert sich jedes Mal, wenn der Text neu erzeugt wird: wählen Sie stattdessen den Text ({"{"}"text": { $text }{"}"})
job-select-traced = Form { $shape } ist aus einem Bild nachgezeichnet und ändert sich jedes Mal, wenn das Bild neu nachgezeichnet wird: wählen Sie stattdessen das Bild ({"{"}"image": { $image }{"}"})
job-select-centre-lines = Bild { $image } ist als offene Mittellinien nachgezeichnet: fräsen Sie sie mit Pfad folgen, oder zeichnen Sie das Bild als Umrisse nach

## Operations

job-on-line-corners = eine Kontur auf der Linie hat keine Innenecken zum Freifräsen
job-tbone-wall = ein T-Bone braucht die Wand, an der er entlangläuft: längere oder kürzere
job-wall-only-tbone = nur ein T-Bone hat eine Wand, an der er entlangläuft
job-corner-chosen-twice = zwei Einstellungen für die Ecke bei { $at }
job-starts-not-finite = jeder Startpunkt braucht endliche x und y
job-stepover-at-most-1 = muss höchstens 1 sein (ein Anteil des Werkzeugdurchmessers), ist { $got }
job-on-line-finish = eine Kontur auf der Linie hat keine Wand zum Schlichten; verwenden Sie innen oder außen
# Lengths without their unit.
job-skin-above-step-down = muss höchstens die Zustellung ({ $step_down }) sein, ist { $got }
job-skin-above-cut = muss kleiner als die Schnitttiefe ({ $cut }, Endtiefe minus Starttiefe) sein, ist { $got }
job-skin-above-tabs = muss kleiner als die Höhe der Haltestege ({ $height }) sein, ist { $got }

## Lists

# Messages joined: "a; b; c" and "a, b, c".
job-semicolons = { $list }; { $last }
job-commas = { $list }, { $last }

## Project files that can't be read

# $found is the schema_version in the file, as written there ("missing" when there is none).
job-error-version = Job-Datei mit schema_version { $found } wird nicht unterstützt: diese Version von Chipload liest { $oldest } bis { $current }. Aktualisieren Sie Chipload oder prüfen Sie die Datei.
job-error-too-old = Job-Datei mit schema_version { $found } stammt aus der Zeit vor den Projekten und wird nicht mehr gelesen. Öffnen Sie sie mit Chipload 0.3.0 und speichern Sie sie, oder importieren Sie ihre DXF erneut.
# $reason is why the old file can't be carried forward, in English: it names the file's fields.
job-error-cannot-migrate = Dieser Job (schema_version { $from }) kann nicht geöffnet werden: { $reason }
# $reason is the operating system's or JSON reader's own words, or one of the job-error texts below.
job-error-unreadable = Job-Datei { $path } kann nicht gelesen werden: { $reason }
# $reason is the JSON reader's own words, or the project's problems joined by semicolons.
job-error-invalid = Ungültige Job-Datei { $path }: { $reason }
job-error-not-object = muss ein JSON-Objekt enthalten
# $error is the operating system's words.
job-error-cannot-write = Schreiben nicht möglich: { $error }
job-error-no-backup = die vorherige Version kann nicht als .bak behalten werden: { $error }
# On the command line, after `set`: $paths are the values set, as paths in the project file.
job-set-would-not-load = das Projekt ließe sich nach dem Setzen von { $paths } nicht mehr laden: { $reason }

## The tool library and machine profiles

# $path is the library, settings or recovery file; $found its schema_version as written there.
job-library-error-version = { $path }: schema_version { $found } wird nicht unterstützt: diese Version von Chipload liest { $current }. Aktualisieren Sie Chipload oder prüfen Sie die Datei.
# $reason is the operating system's or JSON reader's own words, or a job- text.
job-library-error-unreadable = { $path } kann nicht gelesen werden: { $reason }
# $reason is the JSON reader's own words, or the file's rule violations (in English).
job-library-error-invalid = { $path } ist ungültig: { $reason }
job-library-error-unwritable = { $path } kann nicht geschrieben werden: { $reason }
# A change the library refuses: $reason is one of the texts below, or the app's own words.
job-library-error-refused = { $reason }
job-library-no-tool = kein Werkzeug namens „{ $name }“
job-library-no-machine = keine Maschine namens „{ $name }“
job-library-no-material = kein Material namens „{ $name }“
job-library-material-exists = ein Material namens „{ $name }“ gibt es schon
# $entry is "tool" or "machine".
job-library-no-entry = { $entry ->
    [machine] keine Maschine
   *[tool] kein Werkzeug
} namens „{ $name }“
# A file to import that holds the other library: $path is the file, $hint one of the two texts after it.
job-library-other-file = { $path }: { $hint }
job-library-holds-machines = diese Datei enthält Maschinenprofile, keine Werkzeuge: importieren Sie sie als Maschinen
job-library-holds-tools = diese Datei enthält Werkzeuge, keine Maschinenprofile: importieren Sie sie als Werkzeuge
# A value that differs between a project and the library, or a library and a file:
# "diameter 6 mm -> 8 mm". $what is job-library-field; $from and $to its values. Values
# that are words of the project file (endmill, vbit, cw, ccw, files, program, mm, inch,
# bottom-left, …) and names are written as they are.
job-library-change = { $what } { $from } -> { $to }
job-library-field = { $field ->
    [type] Typ
    [diameter] Durchmesser
    [flutes] Schneiden
    [centre-cutting] über Mitte schneidend
    [rpm] Drehzahl
    [angle] Winkel
    [tip] Spitze
    [id] Nummer
    [cutting-values] Schnittwerte
    [post] Postprozessor
    [G-code-units] G-Code-Einheiten
    [tool-changes] Werkzeugwechsel
    [safe-Z] Sicherheitshöhe
    [spindle] Spindel
    [max-feed] max. Vorschub
    [travel] Verfahrweg
    [work-zero] Werkstücknullpunkt
    [max-overcut] max. Tiefe unter Rohteil
    [tool-values] Werte eines Werkzeugs
   *[other] { $field }
}
# An angle in degrees, $degrees without its unit.
job-library-degrees = { $degrees }°
# No value.
job-library-none = nicht gesetzt
job-library-yes-no = { $value ->
    [yes] ja
    [no] nein
   *[unknown] unbekannt
}
# The travel of a machine: $x and $y without their unit, $z with it.
job-library-travel = X{ $x } Y{ $y } Z{ $z }
job-library-work-zero-origin = Ursprung der Zeichnung
# $corner is a word of the project file: bottom-left, top-right, …
job-library-work-zero-stock = Rohteil { $corner }
# $x and $y are lengths without their unit.
job-library-work-zero-point = Punkt X{ $x } Y{ $y }
# A tool's cutting values: feeds and the step down with their units, the stepover a fraction.
job-library-cutting = Vorschub { $feed }, Eintauchen { $plunge }, Zustellung { $step_down }, seitl. Zustellung { $stepover }
job-library-other-values = andere Werte
# A library tool put into a project: $id is its number there.
job-library-tool-there = Werkzeug { $id } (schon im Projekt)
job-library-tool-added = Werkzeug { $id } (hinzugefügt)
# $taken is the number the tool has in the library, taken in the project by another tool.
job-library-tool-added-as = Werkzeug { $id } (als { $id } hinzugefügt; Werkzeug { $taken } war schon vergeben)
# How a project's tool or machine stands to the library entry it was copied from. $entry
# is "tool" (a library tool) or "profile" (a machine profile); $name the entry's name.
job-link-none = nicht aus Ihrer Bibliothek
job-link-missing-named = { $entry ->
    [profile] Profil
   *[tool] Bibliothekswerkzeug
} „{ $name }“ ist nicht in Ihrer Bibliothek
job-link-missing = das zugehörige { $entry ->
    [profile] Profil
   *[tool] Bibliothekswerkzeug
} ist nicht in Ihrer Bibliothek
job-link-same = aus { $entry ->
    [profile] Profil
   *[tool] Bibliothekswerkzeug
} „{ $name }“
# $changes lists what updating would change (job-library-change texts).
job-link-differs = weicht von { $entry ->
    [profile] Profil
   *[tool] Bibliothekswerkzeug
} „{ $name }“ ab: { $changes }

## Importing a library file

# $list is "tools" or "machines", the list in the file.
job-import-empty = Die Datei enthält keine { $list ->
    [machines] Maschinen
   *[tools] Werkzeuge
}.
# $names are the entries' names, separated by commas.
job-import-added = Hinzugefügt: { $names }
job-import-new = Neu: { $names }
job-import-same = Schon in der Bibliothek: { $names }
# $differences are job-library-change texts joined by semicolons.
job-import-replaced = Ersetzt: { $name } (Datei: { $differences })
job-import-kept = Ihres behalten: { $name } (Datei: { $differences })
job-import-differs = Weicht von Ihrem ab: { $name } (Datei: { $differences })
job-import-default = Standard: { $name }
job-import-becomes-default = Wird Standard: { $name }
job-polygon-sides = ein Vieleck hat { $least } bis { $most } Seiten, erhalten { $got }
