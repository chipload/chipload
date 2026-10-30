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
toolpath-vcarve-floor-left-bit = konturer { $contours }: bredare än V-fräsen ({ $across } i diameter) når som högst { $cap } djupt ({ $wide } i ytan): den skär väggarna, och botten mellan dem står kvar i ytan. Lägg till ett bottenpass med en pinnfräs för att rensa den
toolpath-vcarve-floor-left = konturer { $contours }: bredare än ett snitt som är högst { $cap } djupt når ({ $wide } i ytan): V-fräsen skär väggarna, och botten mellan dem står kvar i ytan. Lägg till ett bottenpass med en pinnfräs för att rensa den
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

## Errors: why no toolpath could be made

toolpath-error-cancelled = Beräkningen avbröts
# An error after where it happened: $context is an operation's label, or one of the
# context texts below.
toolpath-error-context = { $context }: { $error }
toolpath-error-label-contour = { $label }, kontur #{ $contour }
# $part is a part's name, $copy the copy's number.
toolpath-error-part-copy = detalj { $part }, kopia { $copy }
toolpath-error-copy = kopia { $copy }
# $at is an operation's number in the cut list ("3", "3.1" for a rest pass).
toolpath-error-no-operation = Det finns ingen operation { $at } i jobbet

## Errors: selecting shapes

# $shapes are shape numbers ("S4, S7").
toolpath-error-shape-missing = formen { $shapes } ingår inte i någon kontur (borttagen, eller ett dubblerat segment eller ett segment med längden noll som importen tog bort)
toolpath-error-construction = { $shapes } är hjälpgeometri: den fräses aldrig. Gör den till en vanlig form eller välj andra former
# $layers are layer names, separated by commas.
toolpath-error-partly-selected = kontur #{ $contour } är bara delvis vald (lager { $layers }); rätta ritningen eller urvalet
# $reason is why the drawing's contour can't be machined, as the DXF import found it (in English).
toolpath-error-contour-invalid = kontur #{ $contour } är ogiltig: { $reason }
# $selector is what the operation selects, built from the toolpath-error-selector texts.
toolpath-error-selects-nothing = urvalet { $selector } träffar ingenting
# $contours are contour numbers ("#3, #4").
toolpath-error-selects-open = urvalet { $selector } träffar bara öppna konturer ({ $contours }); den här operationen behöver slutna
toolpath-error-selector-layer = lager '{ $layer }'
toolpath-error-selector-shapes = formerna { $shapes }
toolpath-error-selector-and = { $first } och { $second }
# $id is the number of a DXF, a picture or a text in the project.
toolpath-error-selector-dxf = DXF { $id }
toolpath-error-selector-image = bild { $id }
toolpath-error-selector-text = text { $id }
# $shapes are the shapes left out ("S3, S5").
toolpath-error-selector-but = { $source } utom { $shapes }
toolpath-error-selector-of = { $what } i { $source }
# $a and $b are contour numbers.
toolpath-error-same-shape = konturerna #{ $a } och #{ $b } är samma form; välj bara en av dem

## Errors: operations and tools

toolpath-error-pass-depths = djupen måste uppfylla 0 <= start ({ $start }) < slut ({ $end }) och step_down ({ $step_down }) måste vara positivt
# $tool is a tool's number.
toolpath-error-rest-tool = rest efter verktyg { $tool }, som projektet inte har
# $diameter is the hole's, $tool the tool's diameter.
toolpath-error-blind-bore = hålet { $diameter } är ett bottenhål och mer än dubbelt så stort som verktyget på { $tool }; använd en ficka
# $tool is the tool's number, $type its kind as written in the project file (endmill, drill, vbit).
toolpath-error-no-hole-fits = ingen vald cirkel kan fräsas med verktyg { $tool } ({ $diameter } { $type })
toolpath-error-no-depth = inget djup att fräsa till
toolpath-error-no-angle = verktyg { $tool } har ingen vinkel
# $got is an angle in degrees, without its unit.
toolpath-error-vbit-angle = V-fräsens vinkel måste vara mer än 0 och mindre än 180 grader, fick { $got }
# $got is a length, without its unit.
toolpath-error-stepover = sidosteget måste vara positivt, fick { $got }

## Errors: profiles, ramps and tabs

toolpath-error-ring-no-length = en slinga i en konturfräsning har ingen längd
toolpath-error-lap-no-course = ett väggvarv har ingen bana
# $angle is in degrees, without its unit; $length is how far the ring goes around.
toolpath-error-ring-ramp = Slingan är för kort för att rampa ned { $drop } i { $angle }° ({ $length } runt om); använd en brantare rampvinkel eller rak nedmatning med ett centrumskärande verktyg
# $reach is how much of the path the ramp may use.
toolpath-error-ramp-reach = Banan är för kort för att rampa ned { $drop } i { $angle }° (bara { $reach } att rampa längs); använd en brantare rampvinkel eller rak nedmatning med ett centrumskärande verktyg
toolpath-error-path-ramp = Banan är för kort för att rampa ned { $drop } i { $angle }° ({ $length } lång); använd en brantare rampvinkel eller rak nedmatning med ett centrumskärande verktyg
# $angle is in degrees, without its unit.
toolpath-error-helix-turns = en helix med radien { $radius } i { $angle }° skulle behöva { $turns } varv för att gå ned { $drop }; använd en större radie eller en brantare vinkel
# $offset is how far inside the line the tool's centre would run.
toolpath-error-tool-does-not-fit = Verktyget får inte plats inuti konturen (dess centrum skulle gå { $offset } innanför linjen)
toolpath-error-offset-verification = Offseten klarade inte kontrollen (banan ligger inte på verktygsradiens avstånd från konturen); konturen kan vara degenererad
# $farthest is how far the finishing lap runs from the roughing path.
toolpath-error-roughing-reach = grovbearbetningen når inte en del av väggen (finskärsvarvet går { $farthest } från grovbearbetningens bana, en verktygsradie eller mer): lämna mindre kvar för finskär eller använd ett mindre verktyg
# $fits is how many tabs would fit.
toolpath-error-tabs-fit = { $count } bryggor på { $width } får inte plats på en bana som är { $length } ({ $fits ->
    [0] inte ens en får plats
   *[other] högst { $fits } får plats
})
toolpath-tab-on-start = bryggan vid { $at } täcker slingans start eller kommer närmare den än en halv verktygsdiameter, där varje pass går ner; flytta bryggan eller slingans start
toolpath-tabs-too-close = bryggorna vid { $at } och { $other } ligger för nära varandra: de behöver sin bredd plus två verktygsdiametrar mellan mittpunkterna
toolpath-tab-no-ring = { $label }: bryggan som placerats vid { $at } ligger inte längre på någon av dess konturer (har ritningen ändrats?); flytta eller ta bort den
# $from is a distance along the ring without its unit, $to one with it.
toolpath-error-tab-carry = kunde inte föra över en brygga ({ $from } till { $to } längs slingan) till grovbearbetningens slinga
# $what is one of the three texts after it.
toolpath-error-tab-verification = Verktygsbanan med bryggor klarade inte kontrollen: { $what }
# $z is a height, $at a distance along the ring.
toolpath-error-tab-below = Z{ $z } vid { $at } längs slingan är under bryggan
toolpath-error-tab-back = verktyget går bakåt längs slingan vid { $at }
toolpath-error-tab-end-below = Z{ $z } vid bryggans slut { $end } längs slingan är under bryggans topp Z{ $top }
# $label is the operation's label; $at the corner's point.
toolpath-error-corner-leg = { $label }: utstickaren vid hörnet { $at } gick inte in i slingan som ut och tillbaka
toolpath-corner-cuts = { $label }: { $style ->
    [dogbone] dogbonen
   *[tbone] T-bonen
} vid hörnet { $at } fräser också in i kontur #{ $contour }: kontrollera detaljen där, eller välj något annat för hörnet
toolpath-corner-cuts-own = { $label }: { $style ->
    [dogbone] dogbonen
   *[tbone] T-bonen
} vid hörnet { $at } fräser också in i en annan vägg av sin egen kontur #{ $contour }: kontrollera detaljen där, eller välj något annat för hörnet (en T-bone längs den andra väggen, eller runt)
toolpath-corner-choice-unmatched = { $label }: hörnet som valts vid { $at } är inte längre något hörn av dess former (har ritningen ändrats?); ta bort valet eller välj igen
# $x and $y are a point's coordinates, without their unit.
toolpath-error-arc-off-circle = bågutdata: ({ $x }, { $y }) ligger inte på cirkeln den märktes med

## Errors: pockets

# $contours are contour numbers ("#3, #4"): the pocket's boundary and its islands.
toolpath-error-islands-enclose = konturerna { $contours }: överlappande öar omsluter en yta mellan sig, vilket fickor inte stöder ännu
toolpath-error-pocket-no-fit = konturerna { $contours }: verktyget (diameter { $diameter }) får inte plats i fickan
toolpath-error-pocket-no-fit-clearance = konturerna { $contours }: verktyget (diameter { $diameter }, förskjutning { $clearance }) får inte plats i fickan
toolpath-error-not-nested = konturerna { $contours }: fickans offsetslingor ligger inte innanför varandra
toolpath-error-too-many-rings = konturerna { $contours }: fler än { $most } slingor i fickan
toolpath-error-pocket-verification = konturerna { $contours }: fickans verktygsbana klarade inte kontrollen (för nära en vägg eller ö); ritningen kan vara degenererad
toolpath-error-nothing-to-clear = Inget att fräsa ur: de valda öarna täcker de valda gränserna (konturerna { $contours })
toolpath-error-island-outside = fickans offset gav en ö utanför alla områden
toolpath-error-shrink-hole = fickans offset gav ett hål när en enda slinga krympte
toolpath-error-open-loop = fickans offset gav en öppen slinga där en sluten väntades
toolpath-error-rest-outside = restbearbetningens område ligger utanför fickan
toolpath-error-left-hole-outside = kvarvarande material: ett hål utanför alla bitar
toolpath-error-left-holes-enclose = kvarvarande material: hål omsluter en yta
toolpath-error-left-wrong-area = kvarvarande material: en boolesk operation gav fel yta
toolpath-error-left-invalid = kvarvarande material: ogiltig geometri från offsetbiblioteket

## Errors: V-carving, centre lines and letters

# What a centre line is for, inside the texts below.
toolpath-error-medial-name = { $name ->
    [vcarve] V-gravyr
   *[centre-line] mittlinjespårning
}
# $name is toolpath-error-medial-name; $error the Voronoi library's own words.
toolpath-error-medial-failed = konturerna { $contours }: mittlinjen för { $name } kunde inte byggas ({ $error }); rör eller korsar konturen sig själv?
toolpath-error-medial-cell = mittlinjen för { $name }: en cell utan sin korda
# $reach is how far from the origin the centre line works.
toolpath-error-medial-far = en kontur för { $name } vid { $at } ligger för långt från ritningens origo (mittlinjen fungerar inom ±{ $reach })
toolpath-error-medial-step = Mittlinjen för { $name }
toolpath-error-vcarve-verification = konturerna { $contours }: V-gravyren klarade inte kontrollen: fräsningen skulle korsa konturen nära { $at }
toolpath-error-join-gave = sammanfogningen av två överlappande konturer gav { $outlines } konturer
toolpath-error-join-changed = sammanfogningen av överlappande konturer ändrade vad som är fyllt nära { $at }
toolpath-error-joined-cross = sammanfogade konturer korsar varandra
toolpath-error-outline-unreadable = en kontur som geometribiblioteket inte kan läsa

## Errors: copies and parts

# $width is the cut's width; $got the pitch, without its unit.
toolpath-error-copies-dx = Kopiorna överlappar: med kolumner måste delningen dx vara minst { $width } (fräsningen är { $width } bred), fick { $got }
toolpath-error-copies-dy = Kopiorna överlappar: med rader måste delningen dy vara minst { $height } (fräsningen är { $height } hög), fick { $got }
# $other and $copy are copies' numbers; $dx and $dy the pitch, without its unit.
toolpath-error-copies-overlap = Kopiorna överlappar: kopiorna { $other } och { $copy } fräser in i varandra; flytta isär dem (delning dx { $dx }, dy { $dy })
# $part and $other_part are parts' names; $x0 and $y0 are without their unit.
toolpath-error-parts-overlap = Detaljen "{ $part }" kopia { $copy } överlappar detaljen "{ $other_part }" kopia { $other_copy } (X{ $x0 } till { $x1 }, Y{ $y0 } till { $y1 }): flytta isär kopiorna eller ändra rutnätet

## Errors: inside the geometry library

# $what is toolpath-error-step or toolpath-error-medial-step.
toolpath-error-library-failed = { $what } misslyckades inuti geometribiblioteket
toolpath-error-step = { $step ->
    [pocket-offset] Fickans offset
    [joining-letters] Sammanfogningen av bokstäver
    [checking-joined-letters] Kontrollen av sammanfogade bokstäver
    [material-left-union] Unionen för kvarvarande material
    [material-left-offset] Offseten för kvarvarande material
    [material-left-boolean] Den booleska operationen för kvarvarande material
    [comparing-shapes] Jämförelsen av former
    [merging-islands] Sammanslagningen av öar
    [subtracting-an-island] Subtraktionen av en ö
   *[offset] Offseten
}
