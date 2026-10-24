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
