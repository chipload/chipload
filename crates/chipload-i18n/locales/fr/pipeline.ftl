# Messages of the app services (crates/chipload-pipeline): problems found with the
# post-processor a project uses.

# $post is the post-processor's name, $units the G-code units the machine is set to, as
# written in the project file ("inch"). "units" is a key of the post's config file.
pipeline-post-units = le post-processeur { $post } n'écrit que du G-code en mm (sa configuration ne liste pas "{ $units }" sous units)

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
