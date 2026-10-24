# Messages of the planner (crates/chipload-toolpath): rules a project must meet before it
# can be cut. Most are shown after the operation's label, as "Operation 3 (Outline): …";
# see job-operation-problem. Words in the texts that name fields or values of the project
# file (safe_z, z_zero, max_feed, entry: ramp, center_cutting, …) are written in the file
# that way and are never translated. A length or feed argument brings its own unit
# ("12.5 mm", "1200 mm/min").

## The project as a whole

toolpath-no-operations = Jobbet har inga operationer
toolpath-all-disabled = Alla operationer är avstängda: inget att fräsa
# $tool is a tool's number.
toolpath-tool-undefined = Verktyg { $tool } finns inte i jobbet
# Under the tool's spindle speed field.
toolpath-no-rpm-short = inget spindelvarvtal
toolpath-no-rpm = Verktyg { $tool } har inget spindelvarvtal (rpm)
# Under the machine's safe height; $top is the height of the stock's top.
toolpath-safe-z-short = måste vara ovanför ämnets ovansida (Z { $top })
# $zero is "top" or "bottom", as written in the project file.
toolpath-safe-z = safe_z { $safe } måste vara ovanför ämnets ovansida (Z { $top }); z_zero är '{ $zero }'

## Depths and sizes

# $below is how far below the stock's bottom it goes, $allowed the setting "max depth
# below stock".
toolpath-too-deep = slutdjupet { $end } går { $below } under ämnet ({ $thickness } tjockt), mer än tillåtna { $allowed } (max djup under ämnet)
toolpath-clearance-radius = förskjutningen { $clearance } måste vara mindre än verktygsradien { $radius }
toolpath-finish-radius = materialet kvar för finskär { $stock } måste vara mindre än verktygsradien { $radius }
toolpath-tab-height = bryggornas höjd { $height } måste vara mindre än fräsdjupet { $cut } (slutdjup minus startdjup)
# $field is "feed" or "plunge_feed", as written in the project file.
toolpath-max-feed = { $field } { $feed } överskrider maskinens max_feed { $max }
toolpath-helix-for-path = helixnedgång är för fickor; Följ bana kan rampa ned (entry: ramp)
toolpath-helix-radius = helixradien { $radius } måste vara mindre än verktygsradien { $tool_radius }, annars blir en kärna stående i mitten
# $most is a number of tool diameters, $longest that as a length.
toolpath-lead-radius = inkörningsradien { $radius } är mer än { $most } × verktygsdiametern ({ $longest })

## Tools

# $tool is a tool's number, $type a kind of tool (job-tool-type).
toolpath-surface-endmill = verktyg { $tool } är av typen { $type }; planfräsning kräver en pinnfräs
toolpath-vcarve-vbit = V-gravyr kräver en V-fräs, verktyg { $tool } är av typen { $type }
toolpath-vbit-no-angle = verktyg { $tool } är en V-fräs utan vinkel; ange dess vinkel
toolpath-pocket-endmill = fickor kräver en pinnfräs, verktyg { $tool } är av typen { $type }
toolpath-path-drill = verktyg { $tool } är ett borr, som inte kan följa en bana; använd en pinnfräs eller en V-fräs
toolpath-holes-vbit = verktyg { $tool } är en V-fräs, som inte kan fräsa hål; använd ett borr eller en pinnfräs
# $method is the holes' method as written in the project file, "drill" or "bore"; $type
# a kind of tool with its article (job-tool-type-article).
toolpath-holes-method = metoden är { $method ->
    [drill] drill, som kräver ett borr
   *[bore] bore, som kräver en pinnfräs
}; verktyg { $tool } är { $type }
# Angles in degrees.
toolpath-flank-not-centre-cutting = verktyg { $tool } är inte centrumskärande och bryggornas flanker ({ $flank }°) är brantare än rampvinkeln ({ $ramp }°); att gå ned längs en flank är en ramp, så gör dem inte brantare än { $ramp }°
toolpath-plunge-not-centre-cutting = verktyg { $tool } är inte centrumskärande och denna operation matar rakt ned; rampa ned i stället (entry: ramp) eller använd ett centrumskärande verktyg

## Rest machining

# $tool is the number of the larger tool the pocket rests after.
toolpath-rest-no-tool = rest efter verktyg { $tool }, som projektet inte har
toolpath-rest-not-endmill = rest efter verktyg { $tool }, av typen { $type }; en fickas verktyg är en pinnfräs
# Two tool diameters.
toolpath-rest-not-larger = rest efter verktyg { $tool } ({ $large }) kräver ett verktyg som är större än denna fickas ({ $small })
toolpath-rest-dogbones = dogbones och restbearbetning kan inte kombineras ännu; gör dogbones i det stora verktygets ficka
toolpath-rest-passes-dogbones = dogbones och restbearbetningar kan inte kombineras ännu; dogbone-hörn behöver ingen restbearbetning
toolpath-rest-pass-no-tool = verktyg { $tool } finns inte i jobbet
toolpath-rest-pass-endmill = verktyg { $tool } är av typen { $type }; en restbearbetning fräser med en pinnfräs
toolpath-rest-pass-larger = verktyg { $tool } ({ $diameter }) måste vara mindre än verktyg { $before } ({ $before_diameter }) före det
toolpath-rest-pass-no-rpm = verktyg { $tool } har inget spindelvarvtal (rpm)

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
