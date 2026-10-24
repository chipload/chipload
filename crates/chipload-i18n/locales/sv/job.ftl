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
job-starts-not-finite = varje startpunkt behöver ändliga x och y
job-stepover-at-most-1 = måste vara högst 1 (en andel av verktygsdiametern), fick { $got }
job-on-line-finish = en konturfräsning på linjen har ingen vägg att finbearbeta; använd inuti eller utanför
# Lengths without their unit.
job-skin-above-step-down = måste vara högst skärdjupet ({ $step_down }), fick { $got }
job-skin-above-cut = måste vara mindre än fräsdjupet ({ $cut }, slutdjup minus startdjup), fick { $got }
job-skin-above-tabs = måste vara mindre än bryggornas höjd ({ $height }), fick { $got }
