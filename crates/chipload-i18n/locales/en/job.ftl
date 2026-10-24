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
job-must-be-more-than-0 = must be more than 0, got { $got }
job-must-be-0-or-more = must be 0 or more, got { $got }
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
job-starts-not-finite = every start point needs finite x and y
job-stepover-at-most-1 = must be at most 1 (a fraction of the tool diameter), got { $got }
job-on-line-finish = a profile on the line has no wall to finish; use inside or outside
# Lengths without their unit.
job-skin-above-step-down = must be at most the step down ({ $step_down }), got { $got }
job-skin-above-cut = must be less than the cut ({ $cut }, end depth minus start depth), got { $got }
job-skin-above-tabs = must be less than the tab height ({ $height }), got { $got }
