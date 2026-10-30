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
job-operation-label-rest = Operation { $number } (rest)
job-operation-label-named-rest = Operation { $number } ({ $name }, rest)
job-operation-label-floor = Operation { $number } (floor)
job-operation-label-named-floor = Operation { $number } ({ $name }, floor)
# The kinds of tools, alone and with their article, inside other messages.
job-tool-type = { $type ->
    [drill] drill
    [vbit] V-bit
   *[endmill] endmill
}
job-tool-type-article = { $type ->
    [drill] a drill
    [vbit] a V-bit
   *[endmill] an endmill
}

## Rules for any value

job-must-be-positive = must be greater than 0, got { $got }
job-must-be-positive-number = must be a positive number, got { $got }
job-vcarve-passes = must be a whole number from 1 to { $most }, got { $got }
job-must-be-more-than-0 = must be more than 0, got { $got }
job-must-be-0-or-more = must be 0 or more, got { $got }
job-through-stale = cuts through the stock, so it ends at { $depth }
job-stock-fit-needs-centred = a stock that fits the drawing must be centred on it (stock.origin: drawing)
job-must-be-1-or-more = must be 1 or more
job-must-be-at-least-1 = must be at least 1
job-must-not-be-empty = must not be empty
job-must-be-finite = must be finite
# $got is an angle in radians, without a unit.
job-must-be-finite-got = must be finite, got { $got }
# $got is a value that isn't a number (NaN, inf).
job-must-be-number = must be a number, got { $got }
# Two lengths without their unit.
job-must-be-numbers = must be numbers, got ({ $x }, { $y })
job-xy-must-be-numbers = x and y must be numbers, got ({ $x }, { $y })
# $id is a number given to two things of the same kind.
job-id-twice = { $id } is used twice
# Unitless numbers.
job-more-than-0-at-most = must be more than 0 and at most { $most }, got { $got }
job-rotation-range = must be at least 0 and less than 360 degrees, got { $got }
job-angle-below-180 = must be more than 0 and less than 180 degrees, got { $got } degrees
job-ramp-angle-range = must be more than 0 and at most 30 degrees, got { $got } degrees
job-flank-angle-range = must be more than 0 and at most 90 degrees, got { $got } degrees

## Copies in a grid

job-copies-per-side = must be from 1 to { $most }, got { $got }
job-copies-too-many = at most { $most } copies in all, got { $got }
job-copies-pitch-zero = must not be 0 with more than one copy that way

## Pictures and texts

# A picture setting's range in pixels; numbers without unit.
job-image-range-px = must be { $low } to { $high } px, got { $got } px
# A picture setting's range as an area in square millimetres; numbers without unit.
job-image-range-area = must be { $low } to { $high } mm², got { $got } mm²
# A length's range: $low is a number alone, $high and $got have their unit.
job-image-range-length = must be { $low } to { $high }, got { $got }
# "auto" is written in the project file as it is.
job-image-threshold = must be 1 to 254 or "auto", got { $got }
job-image-missing = the picture is missing
job-text-empty = is empty: write something to cut
job-text-height = must be more than 0 and at most { $most }, got { $got }
# $heights is a number of text heights, $most that as a length.
job-text-spacing = must be at most { $heights } heights either way ({ $most }), got { $got }

## The drawing

# next_shape_id is a field of the project file.
job-shape-id-range = must be between 1 and next_shape_id - 1 ({ $most }), got { $got }
# drawing.images, drawing.dxfs and drawing.texts are places in the project file; $id is
# a number.
job-no-image = no image { $id } in drawing.images
job-no-dxf = no DXF { $id } in drawing.dxfs
job-no-text = no text { $id } in drawing.texts
job-shape-one-source = a shape comes from one image, DXF or text, not more
job-path-no-segments = a shape needs at least one segment
job-segment-not-finite = coordinates must be finite and radii positive
job-corner-radius-tiny = 0 for sharp corners, or at least { $least }, got { $got }
job-corner-radius-too-large = at most { $most }, half the shorter side, got { $got }
# $closed is "closed" or "open".
job-polyline-points = { $closed ->
    [closed] a closed
   *[open] an open
} polyline needs at least { $least } points, got { $got }
# $tolerance is a length.
job-point-on-previous = is on top of the previous point (closer than { $tolerance })
job-bulges-count = one per segment, { $wanted } for this polyline, got { $got }
job-bulge-range = must be between -1 and 1 (at most a half circle), got { $got }

## Machine and tools

# Two lengths without their unit.
job-work-zero-not-finite = x and y must be finite, got { $x }, { $y }
job-tip-diameter = must be 0 or more and less than the diameter ({ $diameter }), got { $got }

## Parts

job-no-parts = a project needs at least one part
job-no-setups = the project has no setup; it needs at least one
job-setup-twice = setup id { $id } is used twice
job-no-such-setup = no setup { $id } in setups
job-part-twice = part { $id } is listed twice
job-part-name-taken = another part is named "{ $name }"
# "parts" is a place in the project file.
job-no-part = no part { $id } in parts
job-part-order = part { $part }'s operations must all come before part { $before }'s

## Placement and depths

job-scale-range = must be { $low } to { $high }, got { $got }
# end_depth is a field of the project file.
job-start-below-end = must be less than end_depth ({ $end }), got { $got }
job-on-line-clearance = a profile on the line has no side to clear towards; it must be 0, got { $got }

## Surfacing

# Fractions of the tool diameter.
job-surface-stepover = must be from { $low } to { $high } of the tool diameter, got { $got }
job-surface-too-deep = a surface can't go below the stock bottom ({ $thickness } thick), got { $got }

## Which shapes an operation cuts

job-selector-empty = selector needs a DXF, an image, a text, a layer or shapes
job-exclude-no-source = shapes can only be left out of a whole DXF, image or text
# $source is "dxf", "image" or "text"; $id its number.
job-exclude-missing = shape { $shape } is not in { $source ->
    [dxf] DXF
    [image] image
   *[text] text
} { $id } any more: pick the shapes to leave out again
job-selector-one-source = a selector names one DXF, image or text, not more
# The part in braces is written in the project file as it is; $text is a number.
job-select-letter = shape { $shape } is a letter of a text and changes each time the text is made again: select the text instead ({"{"}"text": { $text }{"}"})
job-select-traced = shape { $shape } is traced from an image and changes each time the image is traced again: select the image instead ({"{"}"image": { $image }{"}"})
job-select-centre-lines = image { $image } is traced as open centre lines: cut them with a follow path, or trace the image as outlines

## Operations

job-on-line-corners = a profile on the line has no inside corners to clear
job-tbone-wall = a T-bone needs the wall it runs along: longer or shorter
job-wall-only-tbone = only a T-bone has a wall to run along
job-corner-chosen-twice = two choices for the corner at { $at }
job-starts-not-finite = every start point needs finite x and y
job-stepover-at-most-1 = must be at most 1 (a fraction of the tool diameter), got { $got }
job-on-line-finish = a profile on the line has no wall to finish; use inside or outside
# Lengths without their unit.
job-skin-above-step-down = must be at most the step down ({ $step_down }), got { $got }
job-skin-above-cut = must be less than the cut ({ $cut }, end depth minus start depth), got { $got }
job-skin-above-tabs = must be less than the tab height ({ $height }), got { $got }

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
job-library-no-material = no material named "{ $name }"
job-library-material-exists = a material named "{ $name }" already exists
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
    [tool-values] a tool's values
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
job-polygon-sides = a polygon has { $least } to { $most } sides, got { $got }
