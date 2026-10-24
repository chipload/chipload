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
toolpath-labelled-contour = { $label }, kontur #{ $contour }: { $note }
# Lists built one item at a time: "1, 2, 3" and "1, 2 and 3". $list is the items so far.
toolpath-comma = { $list }, { $item }
toolpath-and = { $list } och { $last }
# An operation's number in a list of operations.
toolpath-number = { $number }
# An operation's number and its kind in a list: "3 (pocket)".
toolpath-operation-kind = { $number } ({ $kind ->
    [profile] konturfräsning
    [pocket] ficka
    [vcarve] V-gravyr
    [path] följ bana
    [holes] hål
   *[surface] planfräsning
})
# $layers is the contour's layer names, $from and $to its two ends ("(10.000, 5.000) mm").
toolpath-skipped-open = hoppade över öppen kontur #{ $contour } på lager { $layers } från { $from } till { $to }
toolpath-into-spoilboard = fräser { $below } under ämnet (ned i offerskivan)
# $operations lists operation numbers with their kind: "1 (pocket), 3 (profile) and 4 (holes)".
toolpath-plunge-unknown = Verktyg { $tool } är inte markerat center_cutting och { $count ->
    [one] operation { $operations } matar
   *[other] operationerna { $operations } matar
} rakt ned: kontrollera att det är ett centrumskärande verktyg, eller rampa ned (entry: ramp)
# $operations is a list of operation numbers ("1, 2 and 4"); $label the surface's label.
toolpath-surface-after = { $label } planfräser ämnet efter att { $count ->
    [one] operation { $operations } har
   *[other] operationerna { $operations } har
} fräst i det: planfräsning kommer oftast först
# $tool is the number of the larger tool the pocket rests after.
toolpath-rest-no-earlier = rest efter verktyg { $tool }, men ingen tidigare ficka fräser dessa former lika djupt med det; resten av fickan lämnas ofräst
toolpath-rest-pass-plunge = matar rakt ned med verktyg { $tool }, som inte är markerat som centrumskärande; använd en helix eller en ramp om det inte är det
# $groups is how many separate runs of operations use the tool; $operations their numbers
# ("1, 2, 5").
toolpath-tool-returns = Verktyg { $tool } används i { $groups } grupper (operationerna { $operations }); läggs de efter varandra sparas ett verktygsbyte
toolpath-vbit-no-tip = verktyg { $tool } har ingen spetsdiameter angiven; V-gravyren räknar med att det är spetsigt
# Angles in degrees.
toolpath-flank-unknown-centre-cutting = bryggornas flanker ({ $flank }°) är brantare än rampvinkeln ({ $ramp }°); verktyg { $tool } är inte markerat center_cutting, kontrollera att det är ett centrumskärande verktyg

## Planning notes: profiles

# $loops is how many separate loops the contour is cut as.
toolpath-narrow-loops = verktyget får inte plats genom en smal del; fräses som { $loops } separata slingor och material blir kvar emellan
# $point is a start point the user set ("(10.000, 5.000) mm").
toolpath-start-far-contour = startpunkten { $point } ligger { $distance } från kontur #{ $contour }; närmaste punkten används
toolpath-start-twice-contour = startpunkten { $point } ligger också på kontur #{ $contour }, som redan har en; den ignoreras
toolpath-tabs-inside = bryggor på en konturfräsning inuti håller kvar den utfrästa biten
# $contours is a list of contour numbers ("#3, #4").
toolpath-tabs-on-curves = några bryggor på konturerna { $contours } sitter på ett hörn eller en kurva (ingen rak sträcka i närheten)
# $pairs lists the tabs that meet, by number ("1 and 2, 3 and 4"); $flank is in degrees.
toolpath-tabs-joined = bryggorna { $pairs } möts under sin topp (flanker på { $flank }°): de håller som en längre brygga
toolpath-finish-lap-cuts = finskärsvarvet fräser upp till { $most } i hörn eller smala delar (kvar för finskär { $stock })
# $style is the operation's corner style.
toolpath-corner-unreached = hörn vid { $point }: verktyget når inte in i det; { $style ->
    [dogbone] ingen dogbone
   *[tbone] ingen T-bone
}

## Planning notes: pockets
## $contours is a list of contour numbers ("#3, #4"): the pocket's boundary and its islands.

toolpath-corners-not-rounded = konturerna { $contours }: slingornas hörn är inte rundade; att runda dem skulle lämna material kvar
toolpath-pocket-narrow = konturerna { $contours }: verktyget får inte plats genom en smal del; fräses ur som { $areas } områden och material blir kvar emellan
# $width is a number without its unit; $height brings the unit; $near is a point.
toolpath-uncut = konturerna { $contours }: ofräst material på { $count ->
    [one] { $count } ställe
   *[other] { $count } ställen
}, största ungefär { $width } x { $height } nära { $near }; minska sidosteget för att få bort det
toolpath-lead-reduced = konturerna { $contours }: inkörningsradien minskad till { $used } (inte plats för { $radius })
toolpath-no-lead = konturerna { $contours }: inte plats för en inkörning (inte ens { $least }); väggen fräses utan inkörning
toolpath-helix-reduced = konturerna { $contours }: helixradien minskad till { $used } (inte plats för { $radius })
toolpath-no-helix-room = konturerna { $contours }: inte plats för en helix (inte ens { $least }); rampar ned i stället
toolpath-no-helix-narrow = konturerna { $contours }: inte plats för en helix (fickan är för smal); rampar ned i stället
toolpath-no-helix-way = konturerna { $contours }: inte plats för en helix (dess väg till första banan passerar en vägg eller en ö); rampar ned i stället
toolpath-start-far-wall = konturerna { $contours }: startpunkten { $point } ligger { $distance } från väggen; närmaste punkten används
toolpath-start-twice-wall = konturerna { $contours }: startpunkten { $point } ligger på en vägg som redan har en; den ignoreras
toolpath-rest-nothing-left = det större verktyget når hela fickan; inget är kvar att restbearbeta
# $width is a number without its unit; $height brings the unit.
toolpath-rest-too-small = restbearbetning: materialet på { $count ->
    [one] { $count } ställe
   *[other] { $count } ställen
} är för litet även för detta verktyg (största ungefär { $width } × { $height })

## Planning notes: V-carving, follow paths and holes

# $across is the V-bit's diameter, $cap the deepest it carves, $wide the widest stroke it
# carves at that depth, $need the depth the widest stroke needs.
toolpath-vcarve-too-wide-bit = konturerna { $contours }: för breda för V-fräsen ({ $across } i diameter), som graverar högst { $cap } djupt ({ $wide } brett vid ytan); kanterna på de bredaste delarna nås inte (de kräver { $need }). Tillåt större djup, använd en bredare vinkel eller fräs ur botten
toolpath-vcarve-too-wide = konturerna { $contours }: för breda för en gravyr som är högst { $cap } djup ({ $wide } bred vid ytan); kanterna på de bredaste delarna nås inte (de kräver { $need }). Tillåt större djup, använd en bredare vinkel eller fräs ur botten
# $narrow is a length of centre line, $tip the V-bit's tip diameter.
toolpath-vcarve-narrow = konturerna { $contours }: { $narrow } av mittlinjen är smalare än spetsen ({ $tip }) och graveras inte
toolpath-path-too-short = kontur #{ $contour } är för kort att följa ({ $length }); hoppas över
toolpath-groove-no-angle = verktyg { $tool } är en V-fräs utan vinkel: spårets bredd är okänd
toolpath-groove-past-cone = V-spåret är djupare än V-fräsens kon: { $width } brett upptill, med raka väggar ovanför konen
toolpath-groove-width = V-spåret är { $width } brett upptill
toolpath-holes-not-circles = hoppade över { $count ->
    [one] { $count } form som inte är en cirkel
   *[other] { $count } former som inte är cirklar
}
# $diameter is the holes' diameter, $tool the tool's, $within how far they may differ.
toolpath-holes-not-drill-size = { $count ->
    [one] hål { $diameter }
   *[other] { $count } hål på { $diameter }
}: inte samma storlek som borret på { $tool } (inom { $within }); borras inte
toolpath-holes-too-small-to-bore = { $count ->
    [one] hål { $diameter }: för litet
   *[other] { $count } hål på { $diameter }: för små
} för att helixborra med verktyget på { $tool }; fräses inte
toolpath-holes-core = { $count ->
    [one] { $count } hål lämnar
   *[other] { $count } hål lämnar
} en lös kärna (hålet är mer än dubbelt så stort som verktyget); håll fast den eller använd en ficka
