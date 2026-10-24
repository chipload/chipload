# Messages of the planner (crates/chipload-toolpath): rules a project must meet before it
# can be cut. Most are shown after the operation's label, as "Operation 3 (Outline): …";
# see job-operation-problem. Words in the texts that name fields or values of the project
# file (safe_z, z_zero, max_feed, entry: ramp, center_cutting, …) are written in the file
# that way and are never translated. A length or feed argument brings its own unit
# ("12.5 mm", "1200 mm/min").

## The project as a whole

toolpath-no-operations = Job has no operations
toolpath-all-disabled = Every operation is disabled: nothing to cut
# $tool is a tool's number.
toolpath-tool-undefined = Tool { $tool } is not defined in the job
# Under the tool's spindle speed field.
toolpath-no-rpm-short = no spindle speed
toolpath-no-rpm = Tool { $tool } has no spindle speed (rpm)
# Under the machine's safe height; $top is the height of the stock's top.
toolpath-safe-z-short = must be above the stock top (Z { $top })
# $zero is "top" or "bottom", as written in the project file.
toolpath-safe-z = safe_z { $safe } must be above the stock top (Z { $top }); z_zero is '{ $zero }'

## Depths and sizes

# $below is how far below the stock's bottom it goes, $allowed the setting "max depth
# below stock".
toolpath-too-deep = end depth { $end } is { $below } below the stock ({ $thickness } thick), more than the allowed { $allowed } (max depth below stock)
toolpath-clearance-radius = clearance { $clearance } must be less than the tool radius { $radius }
toolpath-finish-radius = finishing stock { $stock } must be less than the tool radius { $radius }
toolpath-tab-height = tab height { $height } must be less than the cut depth { $cut } (end depth minus start depth)
# $field is "feed" or "plunge_feed", as written in the project file.
toolpath-max-feed = { $field } { $feed } exceeds machine max_feed { $max }
toolpath-helix-for-path = helix entry is for pockets; a follow path can ramp in (entry: ramp)
toolpath-helix-radius = helix radius { $radius } must be less than the tool radius { $tool_radius }, or it leaves a core standing in the middle
# $most is a number of tool diameters, $longest that as a length.
toolpath-lead-radius = lead radius { $radius } is more than { $most } × the tool diameter ({ $longest })

## Tools

# $tool is a tool's number, $type a kind of tool (job-tool-type).
toolpath-surface-endmill = tool { $tool } is a { $type }; a surface needs an endmill
toolpath-vcarve-vbit = V-carving needs a V-bit, tool { $tool } is a { $type }
toolpath-vbit-no-angle = tool { $tool } is a V-bit with no angle; set its angle
toolpath-pocket-endmill = pockets need an endmill, tool { $tool } is a { $type }
toolpath-path-drill = tool { $tool } is a drill, which can't follow a path; use an endmill or a V-bit
toolpath-holes-vbit = tool { $tool } is a V-bit, which can't cut holes; use a drill or an endmill
# $method is the holes' method as written in the project file, "drill" or "bore"; $type
# a kind of tool with its article (job-tool-type-article).
toolpath-holes-method = the method is { $method ->
    [drill] drill, which needs a drill
   *[bore] bore, which needs an endmill
}; tool { $tool } is { $type }
# Angles in degrees.
toolpath-flank-not-centre-cutting = tool { $tool } is not centre-cutting and the tabs' flanks ({ $flank }°) are steeper than the ramp angle ({ $ramp }°); going down a flank is a ramp, so make them no steeper than { $ramp }°
toolpath-plunge-not-centre-cutting = tool { $tool } is not centre-cutting and this operation plunges straight down; ramp in instead (entry: ramp) or use a centre-cutting tool

## Rest machining

# $tool is the number of the larger tool the pocket rests after.
toolpath-rest-no-tool = rest after tool { $tool }, which the project doesn't have
toolpath-rest-not-endmill = rest after tool { $tool }, a { $type }; a pocket's tool is an endmill
# Two tool diameters.
toolpath-rest-not-larger = rest after tool { $tool } ({ $large }) needs a tool larger than this pocket's ({ $small })
toolpath-rest-dogbones = dogbones and rest machining can't be combined yet; dogbone the large tool's pocket
toolpath-rest-passes-dogbones = dogbones and rest passes can't be combined yet; dogbone corners need no rest
toolpath-rest-pass-no-tool = tool { $tool } is not defined in the job
toolpath-rest-pass-endmill = tool { $tool } is a { $type }; a rest pass cuts with an endmill
toolpath-rest-pass-larger = tool { $tool } ({ $diameter }) must be smaller than tool { $before } ({ $before_diameter }) before it
toolpath-rest-pass-no-rpm = tool { $tool } has no spindle speed (rpm)

## Planning notes: warnings that don't stop the G-code

# A note about an operation after its label ("Operation 3 (Outline): …"); $note is one
# of the notes below.
toolpath-labelled = { $label }: { $note }
# A note about one contour of an operation: "Operation 3 (Outline), contour #4: …".
toolpath-labelled-contour = { $label }, contour #{ $contour }: { $note }
# Lists built one item at a time: "1, 2, 3" and "1, 2 and 3". $list is the items so far.
toolpath-comma = { $list }, { $item }
toolpath-and = { $list } and { $last }
# An operation's number in a list of operations.
toolpath-number = { $number }
# An operation's number and its kind in a list: "3 (pocket)".
toolpath-operation-kind = { $number } ({ $kind ->
    [profile] profile
    [pocket] pocket
    [vcarve] V-carve
    [path] follow path
    [holes] holes
   *[surface] surface
})
# $layers is the contour's layer names, $from and $to its two ends ("(10.000, 5.000) mm").
toolpath-skipped-open = skipped open contour #{ $contour } on { $layers } { $from } to { $to }
toolpath-into-spoilboard = cuts { $below } below the stock (into the spoilboard)
# $operations lists operation numbers with their kind: "1 (pocket), 3 (profile) and 4 (holes)".
toolpath-plunge-unknown = Tool { $tool } isn't marked center_cutting and { $count ->
    [one] operation { $operations } plunges
   *[other] operations { $operations } plunge
} straight down: make sure it is a centre-cutting tool, or ramp in (entry: ramp)
# $operations is a list of operation numbers ("1, 2 and 4"); $label the surface's label.
toolpath-surface-after = { $label } faces the stock after { $count ->
    [one] operation { $operations } has
   *[other] operations { $operations } have
} cut it: surfacing usually comes first
# $tool is the number of the larger tool the pocket rests after.
toolpath-rest-no-earlier = rests after tool { $tool }, but no earlier pocket cuts these shapes with it as deep; the rest of the pocket will be left uncut
toolpath-rest-pass-plunge = plunges straight down with tool { $tool }, which isn't marked centre-cutting; use a helix or a ramp if it isn't
# $groups is how many separate runs of operations use the tool; $operations their numbers
# ("1, 2, 5").
toolpath-tool-returns = Tool { $tool } is used in { $groups } groups (operations { $operations }); moving them next to each other saves a tool change
toolpath-vbit-no-tip = tool { $tool } has no tip diameter set; V-carving takes it as pointed
# Angles in degrees.
toolpath-flank-unknown-centre-cutting = the tabs' flanks ({ $flank }°) are steeper than the ramp angle ({ $ramp }°); tool { $tool } isn't marked center_cutting, make sure it is a centre-cutting tool

## Planning notes: profiles

# $loops is how many separate loops the contour is cut as.
toolpath-narrow-loops = tool does not fit through a narrow part; cut as { $loops } separate loops and material remains in between
# $point is a start point the user set ("(10.000, 5.000) mm").
toolpath-start-far-contour = start point { $point } is { $distance } from contour #{ $contour }; the nearest point is used
toolpath-start-twice-contour = start point { $point } is on contour #{ $contour } too, which already has one; it is ignored
toolpath-tabs-inside = tabs on an inside profile will hold the cut-out slug
# $contours is a list of contour numbers ("#3, #4").
toolpath-tabs-on-curves = some tabs on contours { $contours } sit on a corner or curve (no straight stretch nearby)
# $pairs lists the tabs that meet, by number ("1 and 2, 3 and 4"); $flank is in degrees.
toolpath-tabs-joined = tabs { $pairs } meet below their top (flanks at { $flank }°): they hold as one longer tab
toolpath-finish-lap-cuts = the finishing lap cuts up to { $most } in corners or narrow parts (finishing stock { $stock })
# $style is the operation's corner style.
toolpath-corner-unreached = corner at { $point }: the tool can't reach into it; { $style ->
    [dogbone] no dogbone
   *[tbone] no T-bone
}

## Planning notes: pockets
## $contours is a list of contour numbers ("#3, #4"): the pocket's boundary and its islands.

toolpath-corners-not-rounded = contours { $contours }: ring corners not rounded; rounding would leave material
toolpath-pocket-narrow = contours { $contours }: the tool does not fit through a narrow part; cleared as { $areas } areas and material remains in between
# $width is a number without its unit; $height brings the unit; $near is a point.
toolpath-uncut = contours { $contours }: uncut material in { $count ->
    [one] { $count } place
   *[other] { $count } places
}, largest about { $width } x { $height } near { $near }; reduce the stepover to clear it
toolpath-lead-reduced = contours { $contours }: lead radius reduced to { $used } (no room for { $radius })
toolpath-no-lead = contours { $contours }: no room for a lead (not even { $least }); the wall is cut without one
toolpath-helix-reduced = contours { $contours }: helix radius reduced to { $used } (no room for { $radius })
toolpath-no-helix-room = contours { $contours }: no room for a helix (not even { $least }); ramping in instead
toolpath-no-helix-narrow = contours { $contours }: no room for a helix (the pocket is too narrow); ramping in instead
toolpath-no-helix-way = contours { $contours }: no room for a helix (its way to the first path passes a wall or island); ramping in instead
toolpath-start-far-wall = contours { $contours }: start point { $point } is { $distance } from the wall; the nearest point is used
toolpath-start-twice-wall = contours { $contours }: start point { $point } is on a wall that already has one; it is ignored
toolpath-rest-nothing-left = the larger tool reaches the whole pocket; nothing is left to rest machine
# $width is a number without its unit; $height brings the unit.
toolpath-rest-too-small = rest machining: material in { $count ->
    [one] { $count } place
   *[other] { $count } places
} is too small for this tool too (largest about { $width } × { $height })

## Planning notes: V-carving, follow paths and holes

# $across is the V-bit's diameter, $cap the deepest it carves, $wide the widest stroke it
# carves at that depth, $need the depth the widest stroke needs.
toolpath-vcarve-too-wide-bit = contours { $contours }: too wide for the V-bit ({ $across } across), which carves at most { $cap } deep ({ $wide } wide at the surface); the edges of the widest parts aren't reached (they need { $need }). Allow deeper, use a wider angle, or clear the floor
toolpath-vcarve-too-wide = contours { $contours }: too wide for a carve at most { $cap } deep ({ $wide } wide at the surface); the edges of the widest parts aren't reached (they need { $need }). Allow deeper, use a wider angle, or clear the floor
# $narrow is a length of centre line, $tip the V-bit's tip diameter.
toolpath-vcarve-narrow = contours { $contours }: { $narrow } of centre line is narrower than the tip ({ $tip }) and isn't carved
toolpath-path-too-short = contour #{ $contour } is too short to follow ({ $length }); skipped
toolpath-groove-no-angle = tool { $tool } is a V-bit without an angle: the groove's width is unknown
toolpath-groove-past-cone = the V-groove is deeper than the V-bit's cone: { $width } wide at the top, with straight walls above the cone
toolpath-groove-width = the V-groove is { $width } wide at the top
toolpath-holes-not-circles = skipped { $count ->
    [one] { $count } shape that isn't a circle
   *[other] { $count } shapes that aren't circles
}
# $diameter is the holes' diameter, $tool the tool's, $within how far they may differ.
toolpath-holes-not-drill-size = { $count ->
    [one] hole { $diameter }
   *[other] { $count } holes of { $diameter }
}: not the { $tool } drill's size (within { $within }); not drilled
toolpath-holes-too-small-to-bore = { $count ->
    [one] hole { $diameter }
   *[other] { $count } holes of { $diameter }
}: too small for the { $tool } tool to bore; not cut
toolpath-holes-core = { $count ->
    [one] { $count } hole leaves
   *[other] { $count } holes leave
} a loose core (hole more than twice the tool); hold it down or use a pocket

## Errors: why no toolpath could be made

toolpath-error-cancelled = Generation cancelled
# An error after where it happened: $context is an operation's label, or one of the
# context texts below.
toolpath-error-context = { $context }: { $error }
toolpath-error-label-contour = { $label }, contour #{ $contour }
# $part is a part's name, $copy the copy's number.
toolpath-error-part-copy = part { $part }, copy { $copy }
toolpath-error-copy = copy { $copy }
# $at is an operation's number in the cut list ("3", "3.1" for a rest pass).
toolpath-error-no-operation = No operation { $at } in the job

## Errors: selecting shapes

# $shapes are shape numbers ("S4, S7").
toolpath-error-shape-missing = shape { $shapes } is not part of any contour (deleted, or a duplicate or zero-length segment the import removed)
# $layers are layer names, separated by commas.
toolpath-error-partly-selected = contour #{ $contour } is only partly selected (layers { $layers }); fix the drawing or the selector
# $reason is why the drawing's contour can't be machined, as the DXF import found it (in English).
toolpath-error-contour-invalid = contour #{ $contour } is invalid: { $reason }
# $selector is what the operation selects, built from the toolpath-error-selector texts.
toolpath-error-selects-nothing = selector { $selector } matches nothing
# $contours are contour numbers ("#3, #4").
toolpath-error-selects-open = selector { $selector } matches only open contours ({ $contours }); this operation needs closed ones
toolpath-error-selector-layer = layer '{ $layer }'
toolpath-error-selector-shapes = shapes { $shapes }
toolpath-error-selector-and = { $first } and { $second }
# $id is the number of a DXF, a picture or a text in the project.
toolpath-error-selector-dxf = DXF { $id }
toolpath-error-selector-image = image { $id }
toolpath-error-selector-text = text { $id }
# $shapes are the shapes left out ("S3, S5").
toolpath-error-selector-but = { $source } but { $shapes }
toolpath-error-selector-of = { $what } of { $source }
# $a and $b are contour numbers.
toolpath-error-same-shape = contours #{ $a } and #{ $b } are the same shape; select only one of them

## Errors: operations and tools

toolpath-error-pass-depths = depths must satisfy 0 <= start ({ $start }) < end ({ $end }) and step_down ({ $step_down }) must be positive
# $tool is a tool's number.
toolpath-error-rest-tool = rest after tool { $tool }, which the project doesn't have
# $diameter is the hole's, $tool the tool's diameter.
toolpath-error-blind-bore = hole { $diameter } is blind and more than twice the { $tool } tool; use a pocket
# $tool is the tool's number, $type its kind as written in the project file (endmill, drill, vbit).
toolpath-error-no-hole-fits = no selected circle can be cut with tool { $tool } ({ $diameter } { $type })
toolpath-error-no-depth = no depth to cut to
toolpath-error-no-angle = tool { $tool } has no angle
# $got is an angle in degrees, without its unit.
toolpath-error-vbit-angle = the V-bit angle must be more than 0 and less than 180 degrees, got { $got }
# $got is a length, without its unit.
toolpath-error-stepover = stepover must be positive, got { $got }

## Errors: profiles, ramps and tabs

toolpath-error-ring-no-length = a profile ring has no length
toolpath-error-lap-no-course = a wall lap has no course
# $angle is in degrees, without its unit; $length is how far the ring goes around.
toolpath-error-ring-ramp = The ring is too short to ramp down { $drop } at { $angle }° ({ $length } around); use a steeper ramp angle or a straight plunge with a centre-cutting tool
# $reach is how much of the path the ramp may use.
toolpath-error-ramp-reach = The path is too short to ramp down { $drop } at { $angle }° (only { $reach } to ramp along); use a steeper ramp angle or a straight plunge with a centre-cutting tool
toolpath-error-path-ramp = The path is too short to ramp down { $drop } at { $angle }° ({ $length } long); use a steeper ramp angle or a straight plunge with a centre-cutting tool
# $angle is in degrees, without its unit.
toolpath-error-helix-turns = a helix of radius { $radius } at { $angle }° would need { $turns } turns to go down { $drop }; use a larger radius or a steeper angle
# $offset is how far inside the line the tool's centre would run.
toolpath-error-tool-does-not-fit = Tool does not fit inside the contour (its centre would run { $offset } inside the line)
toolpath-error-offset-verification = Offset failed verification (path not at tool radius from the contour); the contour may be degenerate
# $farthest is how far the finishing lap runs from the roughing path.
toolpath-error-roughing-reach = the roughing can't reach part of the wall (the finishing lap runs { $farthest } from the roughing path, a tool radius or more): use less finishing stock or a smaller tool
# $fits is how many tabs would fit.
toolpath-error-tabs-fit = { $count } tabs of { $width } don't fit on a { $length } path ({ $fits ->
    [0] not even one fits
   *[other] at most { $fits } fit
})
# $from is a distance along the ring without its unit, $to one with it.
toolpath-error-tab-carry = could not carry a tab ({ $from } to { $to } along the ring) onto the roughing ring
# $what is one of the three texts after it.
toolpath-error-tab-verification = Tab toolpath failed verification: { $what }
# $z is a height, $at a distance along the ring.
toolpath-error-tab-below = Z{ $z } at { $at } along the ring is below the tab
toolpath-error-tab-back = the tool goes back along the ring at { $at }
toolpath-error-tab-end-below = Z{ $z } at the tab end { $end } along the ring is below the tab top Z{ $top }
# $label is the operation's label; $at the corner's point.
toolpath-error-corner-leg = { $label }: the leg at corner { $at } did not go into the ring as out and back
toolpath-error-corner-cuts = { $label }: the { $style ->
    [dogbone] dogbone
   *[tbone] T-bone
} at corner { $at } would cut into contour #{ $contour }; lower the corners' max angle or change the drawing
# $x and $y are a point's coordinates, without their unit.
toolpath-error-arc-off-circle = arc output: ({ $x }, { $y }) is not on the circle it was tagged with

## Errors: pockets

# $contours are contour numbers ("#3, #4"): the pocket's boundary and its islands.
toolpath-error-islands-enclose = contours { $contours }: overlapping islands enclose an area between them, which pockets don't support yet
toolpath-error-pocket-no-fit = contours { $contours }: tool (diameter { $diameter }) does not fit in the pocket
toolpath-error-pocket-no-fit-clearance = contours { $contours }: tool (diameter { $diameter }, clearance { $clearance }) does not fit in the pocket
toolpath-error-not-nested = contours { $contours }: pocket offsets are not nested
toolpath-error-too-many-rings = contours { $contours }: more than { $most } pocket rings
toolpath-error-pocket-verification = contours { $contours }: pocket toolpath failed verification (too close to a wall or island); the drawing may be degenerate
toolpath-error-nothing-to-clear = Nothing to clear: the selected islands cover the selected boundaries (contours { $contours })
toolpath-error-island-outside = pocket offset produced an island outside every area
toolpath-error-shrink-hole = pocket offset made a hole while shrinking a single loop
toolpath-error-open-loop = pocket offset returned an open loop where a closed one was expected
toolpath-error-rest-outside = the rest machining area lies outside the pocket
toolpath-error-left-hole-outside = material left: a hole outside every piece
toolpath-error-left-holes-enclose = material left: holes enclose an area
toolpath-error-left-wrong-area = material left: a boolean gave the wrong area
toolpath-error-left-invalid = material left: invalid geometry from the offset library

## Errors: V-carving, centre lines and letters

# What a centre line is for, inside the texts below.
toolpath-error-medial-name = { $name ->
    [vcarve] V-carve
   *[centre-line] centre line
}
# $name is toolpath-error-medial-name; $error the Voronoi library's own words.
toolpath-error-medial-failed = contours { $contours }: the { $name } centre line could not be built ({ $error }); does the outline touch or cross itself?
toolpath-error-medial-cell = { $name } centre line: a cell without its chord
# $reach is how far from the origin the centre line works.
toolpath-error-medial-far = a { $name } outline at { $at } is too far from the drawing origin (the centre line works within ±{ $reach })
toolpath-error-medial-step = The { $name } centre line
toolpath-error-vcarve-verification = contours { $contours }: V-carve verification failed: the cut would cross the outline near { $at }
toolpath-error-join-gave = joining two overlapping outlines gave { $outlines } outlines
toolpath-error-join-changed = joining overlapping outlines changed what is filled near { $at }
toolpath-error-joined-cross = joined outlines cross each other
toolpath-error-outline-unreadable = an outline the geometry library can't read

## Errors: copies and parts

# $width is the cut's width; $got the pitch, without its unit.
toolpath-error-copies-dx = Copies overlap: with columns the pitch dx must be at least { $width } (the cut is { $width } wide), got { $got }
toolpath-error-copies-dy = Copies overlap: with rows the pitch dy must be at least { $height } (the cut is { $height } high), got { $got }
# $other and $copy are copies' numbers; $dx and $dy the pitch, without its unit.
toolpath-error-copies-overlap = Copies overlap: copies { $other } and { $copy } cut into each other; move them apart (pitch dx { $dx }, dy { $dy })
# $part and $other_part are parts' names; $x0 and $y0 are without their unit.
toolpath-error-parts-overlap = Part "{ $part }" copy { $copy } overlaps part "{ $other_part }" copy { $other_copy } (X{ $x0 } to { $x1 }, Y{ $y0 } to { $y1 }): move the copies apart or change the grid

## Errors: inside the geometry library

# $what is toolpath-error-step or toolpath-error-medial-step.
toolpath-error-library-failed = { $what } failed inside the geometry library
toolpath-error-step = { $step ->
    [pocket-offset] Pocket offset
    [joining-letters] Joining letters
    [checking-joined-letters] Checking joined letters
    [material-left-union] Material left union
    [material-left-offset] Material left offset
    [material-left-boolean] Material left boolean
    [comparing-shapes] Comparing shapes
    [merging-islands] Merging islands
    [subtracting-an-island] Subtracting an island
   *[offset] Offset
}
