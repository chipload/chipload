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
