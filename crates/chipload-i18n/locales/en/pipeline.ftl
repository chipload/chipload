# Messages of the app services (crates/chipload-pipeline): problems found with the
# post-processor a project uses, planning notes, the time estimate, chip loads and the
# drawing's facts. A length or feed argument brings its own unit ("12.5 mm",
# "1200 mm/min").

# $post is the post-processor's name, $units the G-code units the machine is set to, as
# written in the project file ("inch"). "units" is a key of the post's config file.
pipeline-post-units = post { $post } writes only mm G-code (its config doesn't list "{ $units }" under units)

## Planning notes

pipeline-drawing-outside-stock = The drawing extends outside the stock
# $count is the number of copies. $x0 and $y0 are numbers without their unit; $x1, $y1
# and $stock_y bring it; $stock_x is a number without it.
pipeline-copies-beyond-stock = The { $count } copies cut X{ $x0 } to { $x1 }, Y{ $y0 } to { $y1 }, beyond the stock ({ $stock_x } × { $stock_y }); check the stock or the part's copies.
# As pipeline-copies-beyond-stock, for one part of several; $part is its name.
pipeline-part-copies-beyond-stock = Part { $part }: the { $count } copies cut X{ $x0 } to { $x1 }, Y{ $y0 } to { $y1 }, beyond the stock ({ $stock_x } × { $stock_y }); check the stock or the part's copies.
# $label is an operation's label, $layer a layer's name, $sources where the layer's shapes
# come from (DXF and picture names, joined with pipeline-and).
pipeline-layer-clash = { $label }: layer "{ $layer }" is in { $sources }; it cuts all of them. Limit it to one (select the layer under that DXF in the Browser)
# Names joined: "a.dxf and b.dxf".
pipeline-and = { $list } and { $last }
# Where shapes come from: a DXF's or picture's name, or one without a name.
pipeline-source-named = { $name }
pipeline-source-dxf = DXF { $id }
pipeline-source-image = image { $id }
pipeline-source-drawn = drawn shapes

## The time estimate (after planning and generating)

# A duration: "45 s", "12 min 30 s", "1 h 05 min". $minutes after hours has two digits.
pipeline-duration-seconds = { $seconds } s
pipeline-duration-minutes = { $minutes } min { $seconds } s
pipeline-duration-hours = { $hours } h { $minutes } min
# The durations are pipeline-duration-… texts; $changes is the number of tool changes.
pipeline-estimate = Estimated time: { $total } (cutting { $cutting }, rapids { $rapids }, spindle { $spindle }){ $changes ->
    [0] {""}
    [one] , plus { $changes } tool change
   *[other] , plus { $changes } tool changes
}
# The lines under it start with spaces, written {"  "}: keep them, they indent the line.
# $name is the part's name.
pipeline-estimate-part = {"  "}Part { $number } ({ $name }): { $time }
# $label is an operation's label.
pipeline-estimate-operation = {"  "}{ $label }: { $time }
# An operation under its part: indented deeper.
pipeline-estimate-part-operation = {"    "}{ $label }: { $time }
# One G-code file of a project written one file per tool.
pipeline-estimate-file = {"  "}File { $number }: { $time }
# $rate is a feed ("5000 mm/min").
pipeline-estimate-rapids-assumed = {"  "}At the programmed feeds (rapids at { $rate }, assumed: set the machine's max feed for a better estimate); acceleration and corners make the real time longer.
pipeline-estimate-rapids = {"  "}At the programmed feeds (rapids at the machine's max feed, { $rate }); acceleration and corners make the real time longer.

## Chip load

pipeline-chip-load-title = Chip load (at the cutting feed):
# $label is an operation's label, $load a pipeline-chip-load… text.
pipeline-chip-load-line = {"  "}{ $label }: { $load }
# $load brings its unit ("0.042 mm/tooth").
pipeline-chip-load = { $load }
pipeline-chip-load-no-flutes = set the tool's flutes to see the chip load
pipeline-chip-load-no-rpm = set the tool's spindle speed to see the chip load
pipeline-chip-load-no-tool = no tool
# The hover text next to an operation's chip load. Tool diameters $small_tool (a number)
# and $large_tool (with its unit); chip loads $wood_low, $aluminium_low and
# $aluminium_high (numbers) and $wood_high (with its unit).
pipeline-chip-load-hint = Chip load: how thick a chip each cutting edge takes, feed / (spindle speed × flutes). Too thin and the tool rubs and heats up; too thick and it may break. It depends on the material, the tool and the machine; as a rough guide, wood and plastics with { $small_tool }–{ $large_tool } tools often take { $wood_low }–{ $wood_high }, aluminium { $aluminium_low }–{ $aluminium_high }. Check the tool maker's figures.

## The drawing's facts (after importing)

# $units is "mm" or "inch", as the DXF gives them or the user chose them.
pipeline-report-units = Units: { $units } (from { $from ->
    [dxf] dxf
    [svg] svg
   *[user] user
})
# Numbers without their unit, except $height, which brings it.
pipeline-report-extents = Extents: X { $x0 } .. { $x1 }, Y { $y0 } .. { $y1 } ({ $width } x { $height })
# $items lists DXF entity types with their counts ("LINE 4, ARC 2").
pipeline-report-imported = Imported: { $items }
pipeline-report-flattened = Flattened: { $items }
pipeline-report-ignored = Ignored: { $items }
pipeline-report-no-issues = Issues: none
pipeline-report-issues = Issues ({ $count }):
# One issue of the import (a dxf-… text), indented.
pipeline-report-issue = {"  "}[{ $severity ->
    [warning] warning
   *[error] error
}] { $issue }
pipeline-report-contours = Contours ({ $count }):
# A row of the contour table, in a fixed-width font: $id, $state and $layers are padded
# with spaces to line up. The coordinates are numbers without their unit.
pipeline-report-contour = {"  "}#{ $id } { $state } layer { $layers } X { $x0 } .. { $x1 }  Y { $y0 } .. { $y1 }
# A contour's state in the table: keep it short.
pipeline-report-contour-state = { $state ->
    [open] open
    [invalid] INVALID
    [hole] hole
   *[outer] outer
}

## The G-code checker's findings (check-… texts), placed in the program

# A finding on a line of the program; $message is the finding.
pipeline-check-line = line { $line }: { $message }
# A finding in one of several G-code files: $file is the file's number.
pipeline-check-file = file { $file }: { $message }
# A finding on a line of one of several G-code files.
pipeline-check-file-line = file { $file }, line { $line }: { $message }

## Errors: a change or run the project refuses

# The independent checker refused the program; each finding follows on its own line.
pipeline-error-check-failed = G-code check failed:
# One more finding (a pipeline-check-… text) on a line of its own, indented, after $list.
pipeline-error-check-finding =
    { $list }
    {"  "}{ $finding }
# Words of the command line or another library, kept as they are.
pipeline-text = { $text }

## Reports of changes to the drawing

# $x0 and $y0 are lengths without their unit, $x1 and $y1 with it; $width and $length the
# stock's size, the first without its unit.
pipeline-stock-fit = The drawing (X{ $x0 } to { $x1 }, Y{ $y0 } to { $y1 }) extends beyond the stock ({ $width } × { $length }); check the setup's stock.

## Placing a DXF: moving, turning and scaling it

# $count is how many DXFs the project has, $dxfs their numbers and names ("1 part-a, 2 part-b").
pipeline-place-which-dxf = the project has { $count } DXFs ({ $dxfs }): say which to place
pipeline-no-dxf = there is no DXF { $id }
pipeline-place-nothing-imported = the project has no imported drawing to move
# What is placed, when the project has a single DXF (otherwise it is the DXF's name).
pipeline-place-the-drawing = the imported drawing
# $dx and $dy are numbers as typed, without a unit.
pipeline-place-cannot-move = can't move the drawing by ({ $dx }, { $dy })
pipeline-place-already-there = The drawing is already there; nothing moved.
# $what is pipeline-place-the-drawing or a DXF's name; $degrees without its unit.
pipeline-place-turned = Turned { $what } { $degrees }° { $direction ->
    [ccw] counter-clockwise
   *[cw] clockwise
}
pipeline-place-reset = Put { $what } back where the DXF has it
# $factor is a scale factor ("2", "0.5").
pipeline-place-scaled = Scaled { $what } ×{ $factor }
# $scaled is pipeline-place-scaled; $width is a length without its unit, $height with it.
pipeline-place-scaled-size = { $scaled } (now { $width } × { $height })
# $dx is a length without its unit, $dy with it.
pipeline-place-moved = Moved { $what } by X{ $dx } Y{ $dy }
# $done is one of the texts above; $placement a pipeline-placement text.
pipeline-place-done = { $done }; it is now { $placement }.
pipeline-place-moved-starts = Moved { $count } start point(s) with it.
pipeline-place-moved-corners = Moved { $count } corner choice(s) with it.
pipeline-place-moved-tabs = Moved { $count } tab(s) placed by hand with it.
pipeline-place-drawn-stay = Shapes drawn in Chipload stay where they are.
pipeline-place-other-dxfs-stay = The other DXFs stay where they are.
pipeline-place-work-zero-stays = The work zero stays at the point X{ $x }, Y{ $y }; check that it's still where you want it.
pipeline-place-margin = the margin must be 0 or more, got { $got }
pipeline-place-stock-follows = the stock is centred on the drawing and follows it, so aligning the drawing to it changes nothing: choose a stock position at the drawing origin first
pipeline-cut-outline = Cut outline: { $width } × { $length } (X { $x0 } … { $x1 }, Y { $y0 } … { $y1 }, the tool included): the smallest stock that holds the job.
# $got is a number as typed, in degrees, without a unit.
pipeline-place-rotation = the rotation must be a number of degrees, got { $got }
pipeline-place-scale = the scale must be a number greater than 0, got { $got }
# Scale factors: the result, and the smallest and largest allowed.
pipeline-place-scale-range = that would scale the drawing ×{ $factor } from the DXF; it must stay between ×{ $min } and ×{ $max }
# $what is "width" or "height"; $got a length without its unit.
pipeline-place-size = the { $what ->
    [height] height
   *[width] width
} must be greater than 0, got { $got }
pipeline-place-no-size = the imported drawing has no { $what ->
    [height] height
   *[width] width
} to scale
# How a DXF is placed: "scaled ×2, turned 90°, moved X12.000 Y-3.500 from the DXF".
pipeline-placement-as-dxf = as in the DXF
pipeline-placement-from-dxf = { $parts } from the DXF
pipeline-placement-and = { $list }, { $last }
pipeline-placement-scaled = scaled ×{ $factor }
pipeline-placement-turned = turned { $degrees }°
# $dx and $dy are lengths without their unit.
pipeline-placement-moved = moved X{ $dx } Y{ $dy }
# After scaling: $within and $off are lengths, $factor the scale factor.
pipeline-scale-curves = { $count } shape(s) from splines or ellipses were approximated within { $within } at the size before; scaled ×{ $factor }, they may be up to { $off } off their curves. Update from the DXF to approximate them again at this size.
pipeline-scale-contours = { $was } closed contour(s) before, { $now } now: gaps are joined within { $within } at the new size. Check the import report and the operations' selections.
# --units is an option of the command line.
pipeline-scale-units = If the DXF's units are wrong, set them on update instead (--units), so the project records them.

## Adding, updating and removing a DXF

# $file is the DXF's file, $id its number, $name its name in the project.
pipeline-dxf-added = Added { $file } as { $kind } { $id } "{ $name }": { $count } { $count ->
    [one] shape
   *[other] shapes
}.
# $operations are operation numbers ("2, 5"); $name a DXF's name.
pipeline-dxf-pinned = { $count ->
    [one] Operation { $operations } keeps cutting { $name } only: the new DXF has its layers too.
   *[other] Operations { $operations } keep cutting { $name } only: the new DXF has their layers too.
}
pipeline-dxf-removed = Removed { $kind } { $id } "{ $name }" and its { $count } { $count ->
    [one] shape
   *[other] shapes
}.
# $label is an operation's label; $shapes shape numbers ("S3, S4").
pipeline-dxf-shapes-gone = { $label }: shapes { $shapes } no longer exist; choose its geometry again
pipeline-dxf-gone = { $label }: its DXF is gone; choose its geometry again
pipeline-dxf-updated = Updated from { $file }: { $unchanged } unchanged, { $changed } changed, { $added } added, { $removed } removed.
# $placement is a pipeline-placement text.
pipeline-dxf-placed-as-before = Placed as before: { $placement }.
pipeline-dxf-geometry-changed = { $label }: geometry changed (shapes { $shapes }); check its toolpath
pipeline-dxf-layer-updated = { $label }: layer { $layer } now uses the updated drawing; check its toolpath

## Pictures and texts

# $number is an operation's number.
pipeline-left-out-found = Operation { $number }: its { $count } left-out { $count ->
    [one] shape
   *[other] shapes
} found again.
pipeline-left-out-lost = Operation { $number }: { $count } left-out { $count ->
    [one] shape
   *[other] shapes
} not found again: pick the shapes to leave out again.
pipeline-no-image = there is no image { $id }
# $error is the PNG writer's own words.
pipeline-image-png-failed = writing the picture: { $error }
# drawing.images is a place in the project file; $error the decoder's own words.
pipeline-image-unreadable = drawing.images: the picture of image { $id } can't be read: { $error }
# $file is the picture's file; $width a length without its unit, $height with it;
# $threshold a grey level (0–255); $picked "yes" when the threshold was picked from the
# picture; $lines "yes" for centre lines, "no" for outlines.
pipeline-image-traced = Traced { $file } ({ $width } × { $height }, threshold { $threshold }{ $picked ->
    [yes] , picked
   *[no] {""}
}): { $count } { $lines ->
    [yes] { $count ->
        [one] centre line
       *[other] centre lines
    }
   *[no] { $count ->
        [one] shape
       *[other] shapes
    }
} on layer { $layer }.
pipeline-image-nothing-black = Nothing is black: change the threshold, or invert if the picture is light on dark.
# $narrow is a length without its unit, $wide with it.
pipeline-image-strokes = Strokes { $narrow } to { $wide } wide: cut them with a follow path.
pipeline-image-closed-lines = { $count ->
    [one] { $count } closed line: a profile or pocket can cut round it, as round any outline.
   *[other] { $count } closed lines: a profile or pocket can cut round them, as round any outline.
}
pipeline-image-dots = { $count } { $count ->
    [one] dot
   *[other] dots
} too small for a centre line left out.
# $area is the smallest piece's area in mm², without its unit.
pipeline-image-specks = { $count } { $count ->
    [one] speck
   *[other] specks
} smaller than { $area } mm² dropped.
pipeline-image-black-at-border = Black touches the picture's edge and is closed along it; if that's the background, invert.
pipeline-image-without-arcs = { $count ->
    [one] { $count } shape kept as lines: its arcs came too near another shape.
   *[other] { $count } shapes kept as lines: their arcs came too near another shape.
}
pipeline-image-unsimplified = { $count } { $count ->
    [one] shape
   *[other] shapes
} kept as traced: smoothing made them cross.
pipeline-no-text = there is no text { $id }
# $text is the text's first line in quotes; $error why (a toolpath-error text).
pipeline-text-join-failed = { $text }: couldn't join the letters' overlapping outlines: { $error }
pipeline-text-font-missing = Font "{ $font }" is not on this computer. { $text } keeps its letters and cuts as they are; to change its words or size, choose another font.
pipeline-text-no-font = There is no font "{ $font }" on this computer. Built in: { $fonts }; `chipload text fonts` lists the computer's too.
pipeline-text-moved = Moved { $text }: its { $count } { $count ->
    [one] letter
   *[other] letters
} as they are (its font is not on this computer)
# $text is the text's first line in quotes; $height its height; $width the box's width
# without its unit, $tall its height with it.
pipeline-text-wrote = Wrote { $text } in { $font }, { $height } high ({ $width } × { $tall }): { $count } { $count ->
    [one] shape
   *[other] shapes
} on layer { $layer }.
# $missing are the characters the font has no letter for.
pipeline-text-missing = { $font } has no letter for "{ $missing }": left out.
pipeline-text-dropped = { $count } { $count ->
    [one] outline
   *[other] outlines
} too small to cut left out.

## Errors: operations and tools

# `tool use` is a command of the command line: keep it as written.
pipeline-no-tool-use = the project has no tool { $tool }; copy one in from your library with `tool use`
pipeline-no-tool = the project has no tool { $tool }
# $number is the operation's number as listed, $count how many the project has.
pipeline-no-operation = there is no operation { $number }; the project has { $count }
pipeline-tab-no-ring = operation { $number } has no contour within a tool diameter of { $at }; `chipload op tabs` lists its tabs
pipeline-tab-none-there = operation { $number } has no tab within a tool diameter of { $at }; `chipload op tabs` lists its tabs
pipeline-tab-no-tabs = operation { $number } has no tabs: only a profile with tabs has them to place
pipeline-corner-none-there = operation { $number } has no corner at { $at }; `chipload op corners` lists its corners
pipeline-corner-not-profile-or-pocket = operation { $number } has no corners to choose: only profiles and pockets have dogbones
pipeline-no-setup = no setup { $id } in the project
pipeline-setup-name-empty = a setup needs a name
pipeline-last-setup = a project needs at least one setup
pipeline-setup-has-parts = setup { $name } has { $count } part(s); move them to another setup first, or remove them with it
pipeline-no-setup-cut = no setup is on with something to cut
pipeline-not-a-pocket = operation { $number } is not a pocket; only pockets rest machine
# $reason is one of the pipeline-rest-… texts.
pipeline-operation-refused = operation { $number }: { $reason }
pipeline-rest-dogbones = it has dogbones, and dogbones and rest machining can't be combined yet; dogbone corners need no rest
pipeline-rest-not-endmill = tool { $tool } is not an endmill
pipeline-not-a-vcarve = operation { $number } is not a V-carve; only V-carves have a floor pass
pipeline-floor-not-endmill = tool { $tool } is not an end mill; a floor pass cuts with one
pipeline-rest-before-not-endmill = tool { $tool } before it is not an endmill
pipeline-rest-not-smaller = tool { $tool } ({ $diameter }) is not smaller than tool { $before } ({ $before_diameter }) before it
# $dxfs lists the project's DXFs by number and name ("1 part-a, 2 part-b").
pipeline-update-which-dxf = the project has { $count } DXFs ({ $dxfs }): say which to update
# `tool use` is a command of the command line: keep it as written.
pipeline-surface-no-endmill = the project has no endmill: a surface needs one (copy one in with `tool use`)
# $type is the tool's type as written in the project file (drill, vbit).
pipeline-surface-not-endmill = tool { $tool } is a { $type }; a surface needs an endmill

## Errors: drawn shapes

pipeline-shape-not-drawable = only circles, rectangles and polylines can be drawn
# $kind is the shape's kind as written in the project file (circle, rectangle, polyline).
pipeline-shape-kind = shape { $id } is a { $kind }; its kind can't change
pipeline-no-shapes-given = no shapes given
pipeline-shape-twice = shape { $id } is given twice
pipeline-no-shape = there is no shape { $id }
pipeline-shape-imported-copy = shape { $id } was imported from the DXF; only drawn shapes can be copied or mirrored
pipeline-shapes-no-extent = the shapes have no extent
pipeline-shape-imported-transform = shape { $id } isn't a drawn shape: only drawn shapes can be moved, turned or scaled (explode its DXF first)
pipeline-construction-imported = shape { $id } isn't a drawn shape: only drawn shapes can be construction geometry (explode its DXF first)
pipeline-construction-used = { $label } names construction geometry, which is never cut: it can't run until it cuts other shapes.
pipeline-transform-not-numbers = the move, turn and centre must be numbers
pipeline-shapes-copy-imported = only drawn shapes can be copied, not imported ones
# "Update drawing from DXF" is the command in the app's File menu.
pipeline-shape-imported-change = shape { $id } was imported from the DXF and can't be changed here; change the drawing and use Update drawing from DXF

## Errors: parts and copies

pipeline-no-part = No part { $id } in the project
pipeline-part-name-empty = A part's name can't be empty
pipeline-part-name-taken = Another part is named "{ $name }"
pipeline-last-part = A project needs at least one part: this is the last one
pipeline-part-has-operations = Part { $name } has { $count ->
    [one] { $count } operation
   *[other] { $count } operations
}: move them to another part first, or remove them with it
pipeline-part-no-operation = No operation { $number } in the project
pipeline-part-cuts-nothing = The part cuts nothing
pipeline-copies-gap = The gap must be 0 or more, got { $got }

## Errors: the simulation

# $min is a length without its unit, $max and $got with it.
pipeline-simulate-cell-size = cell size must be between { $min } and { $max }, got { $got }
# $cell is the size of one cell, a length.
pipeline-simulate-too-many-cells = a { $cell } cell makes { $cells } cells for this stock, more than { $most }; use a larger cell
pipeline-simulate-stock-too-large = the stock is too large to simulate at { $cell } cells (more than { $most } cells)
# $error is the PNG encoder's own words.
pipeline-simulate-png = PNG: { $error }

## Post-processors

pipeline-post-no-settings = no settings folder, so no post "{ $name }"
pipeline-post-no-folder = No post-processor "{ $name }": it isn't built in, and there is no posts folder
# Why a project's copy of a user post can't be compared with its file.
pipeline-post-no-posts-folder = no posts folder
# What a post-processor is, one line each (Settings, This computer). $name, $extension,
# the paths and the units (mm, inch) and modes (files, program) are written as in its file.
pipeline-post-name = Name: { $name }
pipeline-post-builtin = Built in
pipeline-post-yours = Yours
pipeline-post-settings = Settings: { $path }
pipeline-post-template = Template: { $path }
pipeline-post-extension = File extension: { $extension }
pipeline-post-decimals = Decimals: { $mm } (mm), { $inch } (inch)
pipeline-post-units-list = G-code units: { $units }
pipeline-post-arcs = Arcs: { $arcs ->
    [arcs] G2/G3
   *[lines] as short lines (G1)
}
pipeline-post-tool-changes = Several tools: { $modes }
# $seconds is a time in seconds, without its unit.
pipeline-post-dwell = Spindle dwell: { $seconds } s
pipeline-post-no-dwell = Spindle dwell: none

## The tool library and machine profiles

# $error is why the library file couldn't be read (a job-library-error text).
pipeline-library-defaults = { $error } (using built-in defaults)
pipeline-library-no-profile = no machine profile named "{ $name }"
# What a new project started with. $how is "profile" (asked for) or "default".
pipeline-library-machine = machine: { $name } ({ $how ->
    [default] default profile
   *[profile] profile
})
pipeline-library-tool = tool { $tool }: { $name } (default tool)
pipeline-library-material = material: { $name } (default material)
# $error is why the post can't be used (a post-error text).
pipeline-library-post-unusable = post-processor "{ $name }": { $error }
pipeline-library-unknown-post = Machine "{ $name }": this Chipload has no post-processor "{ $post }"; projects using it can't make G-code until it has
pipeline-material-chosen = Material: { $name }
pipeline-material-none = Material { $name } no longer chosen; the feeds stay as they are
pipeline-material-op-set = Operation { $op } ({ $name }): feeds from { $material }
pipeline-material-op-kept = Operation { $op } ({ $name }): tool { $tool } has no values for { $material }; its feeds are unchanged
pipeline-material-tool-not-in-library = Tool { $tool } isn't in your tool library: save it there first (tool save), then its values can be kept in a material

## Recovery copies

# $reason is the operating system's words, or the text below.
pipeline-recovery-io = Cannot write the recovery copy { $path }: { $reason }
pipeline-recovery-locked = already locked

## Dragging a picture's or text's box

pipeline-box-turn-centre = the turn handle is on the centre
pipeline-box-no-size = the box has no size
# $min is the smallest size, a length.
pipeline-box-too-small = it can't be smaller than { $min }
pipeline-box-no-corner = a box has no corner { $corner }

## G-code files

# One of several G-code files: $name is its file name, $operations the numbers of the
# operations it cuts ("1, 3").
pipeline-gcode-file = { $name } (tool { $tool }, operations { $operations })

## Explode a DXF, join close ends (docs/specs/explode-and-join.md)

pipeline-dxf-exploded = Exploded { $kind } { $id } "{ $name }" into { $count } drawn { $count ->
    [one] shape
   *[other] shapes
}. The project no longer knows the { $kind } file.
# $shapes shape numbers ("S3, S4"); $tolerance a length.
pipeline-explode-split = { $count ->
    [one] Shape { $shapes } was in pieces that don't meet: each piece is a shape now.
   *[other] Shapes { $shapes } were in pieces that don't meet: each piece is a shape now.
}
pipeline-explode-circle-start = { $count ->
    [one] Circle { $shapes } now starts at its right-hand point, counter-clockwise, as drawn circles do: its cut may start elsewhere on it.
   *[other] Circles { $shapes } now start at their right-hand points, counter-clockwise, as drawn circles do: their cuts may start elsewhere on them.
}
pipeline-explode-dropped = { $count ->
    [one] Shape { $shapes } had only segments shorter than { $tolerance } and is gone.
   *[other] Shapes { $shapes } had only segments shorter than { $tolerance } and are gone.
}
# $label is an operation's label.
pipeline-explode-selects = { $label } now selects shapes { $shapes }.
pipeline-explode-no-dxf = the project has no DXF to explode
pipeline-explode-which-dxf = the project has { $count } DXFs ({ $dxfs }): say which to explode
pipeline-join-gap-range = The gap must be { $least } to { $most }, got { $got }.
pipeline-join-imported = Shape S{ $id } comes from a DXF: explode the DXF first.
pipeline-join-none = No ends were joined (gap { $gap }).
pipeline-joined = Joined { $pieces } shapes into { $count }, { $closed } of them closed; widest gap closed { $widest }.
pipeline-joined-touching = Joined { $pieces } shapes into { $count }, { $closed } of them closed; their ends met.
pipeline-join-open = { $count } { $count ->
    [one] end stays
   *[other] ends stay
} open.
# $at a point, $nearest a length.
pipeline-join-open-end = The end at { $at }: the nearest other end is { $nearest } away.
pipeline-join-open-alone = The end at { $at }: no other end on its layer.
pipeline-join-branch = Three or more ends meet at { $at }: not joined there.
pipeline-edit-imported = S{ $id } comes from a DXF, a picture or a text: explode the DXF, or draw the shape, to change it
pipeline-offset-distance = the offset distance must be more than 0, got { $got }
pipeline-offset-vanishes = nothing of S{ $id } is left { $distance } inside it
pipeline-offset-side = S{ $id } is { $closed ->
    [closed] closed: offset it outside or inside
   *[open] open: offset it left or right
}
pipeline-offset-failed = S{ $id } couldn't be offset: { $why }
pipeline-new-shapes-cut = { $count ->
    [one] The new shape is
   *[other] The { $count } new shapes are
} on layer { $layer }, which { $ops ->
    [one] operation { $operations } cuts
   *[other] operations { $operations } cut
}.
pipeline-corner-none = S{ $id } has no corners to round or cut
pipeline-corner-not = corner { $corner } of S{ $id } isn't between two straight sides
pipeline-corner-straight = S{ $id } runs straight on at { $corner }: there is no corner
pipeline-corner-too-large = that is too large for the sides of S{ $id }: at most { $most }
pipeline-corner-size = the radius or distance must be more than 0
pipeline-trim-nothing-crosses = nothing crosses S{ $id } there (a closed shape needs two crossings): delete it instead
pipeline-trim-everything = trimming S{ $id } there would leave nothing of it: delete it instead
pipeline-extend-not-straight = only a straight end of S{ $id } can be extended
pipeline-extend-nothing-ahead = nothing lies ahead of that end of S{ $id }
pipeline-extend-closed = S{ $id } is closed: it has no end to extend
pipeline-break-at-end = that is an end of S{ $id }: there is nothing to break
pipeline-combine-two = combining needs two shapes or more
pipeline-combine-open = S{ $id } is open: only closed shapes can be combined
pipeline-combine-empty = { $op ->
    [subtract] nothing of S{ $id } is left: the other shapes cover all of it
   *[other] the shapes have no part in common: nothing is left
}
pipeline-combine-failed = the shapes couldn't be combined: { $why }
pipeline-array-count = an array has 2 to { $most } shapes including the original, got { $got }
pipeline-array-spacing = the copies would lie on each other: give a spacing other than 0 where there is more than one column or row
pipeline-array-angle = the angle must be more than 0 and at most 360 degrees, got { $got }
pipeline-align-two = aligning needs two shapes or more, or the stock to align to
pipeline-distribute-three = distributing needs three shapes or more
pipeline-shape-gone = S{ $id } is gone: { $ops ->
    [one] operation { $operations } used it; choose its geometry again
   *[other] operations { $operations } used it; choose their geometry again
}.

## DXF export (docs/specs/dxf-export.md)

pipeline-export-nothing = there is nothing to export: no shapes are shown there
pipeline-export-no-layer = the drawing has no layer { $layer }
# $why is in English.
pipeline-export-unverified = the DXF wasn't written: S{ $id } didn't read back the same ({ $why })
pipeline-export-summary = { $count ->
    [one] 1 entity
   *[other] { $count } entities
} on { $layers ->
    [one] 1 layer
   *[other] { $layers } layers
}, in { $units ->
    [inch] inches
   *[mm] mm
}.
pipeline-export-renamed = Layer { $from } is { $to } in the DXF: a DXF layer name can't hold some of its characters.

# Frames (docs/specs/shape-library.md §3)
pipeline-frame-borders = a frame has at most { $most } borders, got { $got }
pipeline-frame-gap = the gap between borders must be more than 0, got { $got }
pipeline-frame-margin = the margin must be 0 or more, got { $got }
pipeline-frame-nothing = there is nothing to frame: none of those shapes is in the drawing
pipeline-frame-size = a frame's width and height must be more than 0
pipeline-frame-r = the corner size of this frame is at most { $largest }
pipeline-frame-no-fit = no frame of this kind fits round those shapes
pipeline-frame-borders-fit = { $count ->
    [one] a border { $gap } inside doesn't fit in this frame
   *[other] { $count } borders { $gap } apart don't fit in this frame
}
