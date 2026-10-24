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
job-starts-not-finite = jeder Startpunkt braucht endliche x und y
job-stepover-at-most-1 = muss höchstens 1 sein (ein Anteil des Werkzeugdurchmessers), ist { $got }
job-on-line-finish = eine Kontur auf der Linie hat keine Wand zum Schlichten; verwenden Sie innen oder außen
# Lengths without their unit.
job-skin-above-step-down = muss höchstens die Zustellung ({ $step_down }) sein, ist { $got }
job-skin-above-cut = muss kleiner als die Schnitttiefe ({ $cut }, Endtiefe minus Starttiefe) sein, ist { $got }
job-skin-above-tabs = muss kleiner als die Höhe der Haltestege ({ $height }) sein, ist { $got }
