# Messages of the planner (crates/chipload-toolpath): rules a project must meet before it
# can be cut. Most are shown after the operation's label, as "Operation 3 (Outline): …";
# see job-operation-problem. Words in the texts that name fields or values of the project
# file (safe_z, z_zero, max_feed, entry: ramp, center_cutting, …) are written in the file
# that way and are never translated. A length or feed argument brings its own unit
# ("12.5 mm", "1200 mm/min").

## The project as a whole

toolpath-no-operations = Il lavoro non ha lavorazioni
toolpath-all-disabled = Tutte le lavorazioni sono disattivate: niente da tagliare
# $tool is a tool's number.
toolpath-tool-undefined = L'utensile { $tool } non è definito nel lavoro
# Under the tool's spindle speed field.
toolpath-no-rpm-short = nessuna velocità mandrino
toolpath-no-rpm = L'utensile { $tool } non ha una velocità mandrino (rpm)
# Under the machine's safe height; $top is the height of the stock's top.
toolpath-safe-z-short = deve essere sopra la superficie del grezzo (Z { $top })
# $zero is "top" or "bottom", as written in the project file.
toolpath-safe-z = safe_z { $safe } deve essere sopra la superficie del grezzo (Z { $top }); z_zero è '{ $zero }'

## Depths and sizes

# $below is how far below the stock's bottom it goes, $allowed the setting "max depth
# below stock".
toolpath-too-deep = la profondità finale { $end } è { $below } sotto il grezzo (spessore { $thickness }), più dei { $allowed } consentiti (prof. max sotto grezzo)
toolpath-clearance-radius = la maggiorazione { $clearance } deve essere minore del raggio dell'utensile { $radius }
toolpath-finish-radius = il sovrametallo di finitura { $stock } deve essere minore del raggio dell'utensile { $radius }
toolpath-tab-height = l'altezza dei ponticelli { $height } deve essere minore della profondità di taglio { $cut } (profondità finale meno profondità iniziale)
# $field is "feed" or "plunge_feed", as written in the project file.
toolpath-max-feed = { $field } { $feed } supera max_feed della macchina { $max }
toolpath-helix-for-path = l'ingresso a elica è per le tasche; un segui percorso può entrare in rampa (entry: ramp)
toolpath-helix-radius = il raggio dell'elica { $radius } deve essere minore del raggio dell'utensile { $tool_radius }, altrimenti resta in piedi un nucleo al centro
# $most is a number of tool diameters, $longest that as a length.
toolpath-lead-radius = il raggio di raccordo { $radius } è più di { $most } × il diametro dell'utensile ({ $longest })

## Tools

# $tool is a tool's number, $type a kind of tool (job-tool-type).
toolpath-surface-endmill = l'utensile { $tool } è una { $type }; una spianatura richiede una fresa
toolpath-vcarve-vbit = l'incisione a V richiede una fresa a V, l'utensile { $tool } è una { $type }
toolpath-vbit-no-angle = l'utensile { $tool } è una fresa a V senza angolo; impostane l'angolo
toolpath-pocket-endmill = le tasche richiedono una fresa, l'utensile { $tool } è una { $type }
toolpath-path-drill = l'utensile { $tool } è una punta, che non può seguire un percorso; usa una fresa o una fresa a V
toolpath-holes-vbit = l'utensile { $tool } è una fresa a V, che non può tagliare fori; usa una punta o una fresa
# $method is the holes' method as written in the project file, "drill" or "bore"; $type
# a kind of tool with its article (job-tool-type-article).
toolpath-holes-method = il metodo è { $method ->
    [drill] drill, che richiede una punta
   *[bore] bore, che richiede una fresa
}; l'utensile { $tool } è { $type }
# Angles in degrees.
toolpath-flank-not-centre-cutting = l'utensile { $tool } non taglia al centro e i fianchi dei ponticelli ({ $flank }°) sono più ripidi dell'angolo di rampa ({ $ramp }°); scendere lungo un fianco è una rampa, quindi rendili non più ripidi di { $ramp }°
toolpath-plunge-not-centre-cutting = l'utensile { $tool } non taglia al centro e questa lavorazione affonda in verticale; entra invece in rampa (entry: ramp) o usa un utensile che taglia al centro

## Rest machining

# $tool is the number of the larger tool the pocket rests after.
toolpath-rest-no-tool = ripresa dopo l'utensile { $tool }, che il progetto non ha
toolpath-rest-not-endmill = ripresa dopo l'utensile { $tool }, una { $type }; l'utensile di una tasca è una fresa
# Two tool diameters.
toolpath-rest-not-larger = la ripresa dopo l'utensile { $tool } ({ $large }) richiede un utensile più grande di quello di questa tasca ({ $small })
toolpath-rest-dogbones = dogbone e ripresa dei residui non si possono ancora combinare; metti i dogbone nella tasca dell'utensile grande
toolpath-rest-passes-dogbones = dogbone e passate di ripresa non si possono ancora combinare; gli angoli dogbone non richiedono ripresa
toolpath-rest-pass-no-tool = l'utensile { $tool } non è definito nel lavoro
toolpath-rest-pass-endmill = l'utensile { $tool } è una { $type }; una passata di ripresa taglia con una fresa
toolpath-rest-pass-larger = l'utensile { $tool } ({ $diameter }) deve essere più piccolo dell'utensile { $before } ({ $before_diameter }) che lo precede
toolpath-rest-pass-no-rpm = l'utensile { $tool } non ha una velocità mandrino (rpm)

## Planning notes: warnings that don't stop the G-code

# A note about an operation after its label ("Operation 3 (Outline): …"); $note is one
# of the notes below.
toolpath-labelled = { $label }: { $note }
# A note about one contour of an operation: "Operation 3 (Outline), contour #4: …".
toolpath-labelled-contour = { $label }, contorno #{ $contour }: { $note }
# Lists built one item at a time: "1, 2, 3" and "1, 2 and 3". $list is the items so far.
toolpath-comma = { $list }, { $item }
toolpath-and = { $list } e { $last }
# An operation's number in a list of operations.
toolpath-number = { $number }
# An operation's number and its kind in a list: "3 (pocket)".
toolpath-operation-kind = { $number } ({ $kind ->
    [profile] contornatura
    [pocket] tasca
    [vcarve] incisione a V
    [path] segui percorso
    [holes] fori
   *[surface] spianatura
})
# $layers is the contour's layer names, $from and $to its two ends ("(10.000, 5.000) mm").
toolpath-skipped-open = saltato il contorno aperto #{ $contour } su { $layers } da { $from } a { $to }
toolpath-into-spoilboard = taglia { $below } sotto il grezzo (nel piano di sacrificio)
# $operations lists operation numbers with their kind: "1 (pocket), 3 (profile) and 4 (holes)".
toolpath-plunge-unknown = L'utensile { $tool } non è indicato come center_cutting e { $count ->
    [one] la lavorazione { $operations } affonda
   *[other] le lavorazioni { $operations } affondano
} in verticale: assicurati che sia un utensile che taglia al centro, o entra in rampa (entry: ramp)
# $operations is a list of operation numbers ("1, 2 and 4"); $label the surface's label.
toolpath-surface-after = { $label } spiana il grezzo dopo che { $count ->
    [one] la lavorazione { $operations } l'ha tagliato
   *[other] le lavorazioni { $operations } l'hanno tagliato
}: di solito la spianatura viene per prima
# $tool is the number of the larger tool the pocket rests after.
toolpath-rest-no-earlier = ripresa dopo l'utensile { $tool }, ma nessuna tasca precedente taglia queste forme con esso alla stessa profondità; il resto della tasca resterà non lavorato
toolpath-rest-pass-plunge = affonda in verticale con l'utensile { $tool }, che non è indicato come utensile che taglia al centro; usa un'elica o una rampa se non lo è
# $groups is how many separate runs of operations use the tool; $operations their numbers
# ("1, 2, 5").
toolpath-tool-returns = L'utensile { $tool } è usato in { $groups } gruppi (lavorazioni { $operations }); metterle una accanto all'altra risparmia un cambio utensile
toolpath-vbit-no-tip = l'utensile { $tool } non ha un diametro della punta impostato; l'incisione a V lo considera appuntito
# Angles in degrees.
toolpath-flank-unknown-centre-cutting = i fianchi dei ponticelli ({ $flank }°) sono più ripidi dell'angolo di rampa ({ $ramp }°); l'utensile { $tool } non è indicato come center_cutting, assicurati che sia un utensile che taglia al centro

## Planning notes: profiles

# $loops is how many separate loops the contour is cut as.
toolpath-narrow-loops = l'utensile non passa in una parte stretta; tagliato come { $loops } anelli separati e in mezzo resta materiale
# $point is a start point the user set ("(10.000, 5.000) mm").
toolpath-start-far-contour = il punto iniziale { $point } è a { $distance } dal contorno #{ $contour }; si usa il punto più vicino
toolpath-start-twice-contour = il punto iniziale { $point } è anche sul contorno #{ $contour }, che ne ha già uno; viene ignorato
toolpath-tabs-inside = i ponticelli su una contornatura interna tratterranno lo sfrido ritagliato
# $contours is a list of contour numbers ("#3, #4").
toolpath-tabs-on-curves = alcuni ponticelli sui contorni { $contours } cadono su un angolo o una curva (nessun tratto dritto vicino)
# $pairs lists the tabs that meet, by number ("1 and 2, 3 and 4"); $flank is in degrees.
toolpath-tabs-joined = i ponticelli { $pairs } si toccano sotto la sommità (fianchi a { $flank }°): tengono come un unico ponticello più lungo
toolpath-finish-lap-cuts = il giro di finitura taglia fino a { $most } negli angoli o nelle parti strette (sovrametallo di finitura { $stock })
# $style is the operation's corner style.
toolpath-corner-unreached = angolo in { $point }: l'utensile non riesce a entrarci; { $style ->
    [dogbone] nessun dogbone
   *[tbone] nessun T-bone
}

## Planning notes: pockets
## $contours is a list of contour numbers ("#3, #4"): the pocket's boundary and its islands.

toolpath-corners-not-rounded = contorni { $contours }: angoli degli anelli non arrotondati; arrotondarli lascerebbe materiale
toolpath-pocket-narrow = contorni { $contours }: l'utensile non passa in una parte stretta; svuotata come { $areas } zone separate e in mezzo resta materiale
# $width is a number without its unit; $height brings the unit; $near is a point.
toolpath-uncut = contorni { $contours }: materiale non tagliato in { $count ->
    [one] { $count } punto
   *[other] { $count } punti
}, il più grande circa { $width } x { $height } vicino a { $near }; riduci il passo laterale per rimuoverlo
toolpath-lead-reduced = contorni { $contours }: raggio di raccordo ridotto a { $used } (non c'è spazio per { $radius })
toolpath-no-lead = contorni { $contours }: non c'è spazio per un raccordo (nemmeno { $least }); la parete viene tagliata senza
toolpath-helix-reduced = contorni { $contours }: raggio dell'elica ridotto a { $used } (non c'è spazio per { $radius })
toolpath-no-helix-room = contorni { $contours }: non c'è spazio per un'elica (nemmeno { $least }); si entra invece in rampa
toolpath-no-helix-narrow = contorni { $contours }: non c'è spazio per un'elica (la tasca è troppo stretta); si entra invece in rampa
toolpath-no-helix-way = contorni { $contours }: non c'è spazio per un'elica (il suo tragitto verso il primo percorso incontra una parete o un'isola); si entra invece in rampa
toolpath-start-far-wall = contorni { $contours }: il punto iniziale { $point } è a { $distance } dalla parete; si usa il punto più vicino
toolpath-start-twice-wall = contorni { $contours }: il punto iniziale { $point } è su una parete che ne ha già uno; viene ignorato
toolpath-rest-nothing-left = l'utensile più grande raggiunge tutta la tasca; non resta nulla da riprendere
# $width is a number without its unit; $height brings the unit.
toolpath-rest-too-small = ripresa dei residui: il materiale in { $count ->
    [one] { $count } punto
   *[other] { $count } punti
} è troppo piccolo anche per questo utensile (il più grande circa { $width } × { $height })

## Planning notes: V-carving, follow paths and holes

# $across is the V-bit's diameter, $cap the deepest it carves, $wide the widest stroke it
# carves at that depth, $need the depth the widest stroke needs.
toolpath-vcarve-too-wide-bit = contorni { $contours }: troppo larghi per la fresa a V ({ $across } di diametro), che incide al massimo a { $cap } di profondità ({ $wide } di larghezza in superficie); i bordi delle parti più larghe non vengono raggiunti (richiedono { $need }). Consenti più profondità, usa un angolo più ampio o svuota il fondo
toolpath-vcarve-too-wide = contorni { $contours }: troppo larghi per un'incisione profonda al massimo { $cap } ({ $wide } di larghezza in superficie); i bordi delle parti più larghe non vengono raggiunti (richiedono { $need }). Consenti più profondità, usa un angolo più ampio o svuota il fondo
# $narrow is a length of centre line, $tip the V-bit's tip diameter.
toolpath-vcarve-narrow = contorni { $contours }: { $narrow } di linea mediana sono più stretti della punta ({ $tip }) e non vengono incisi
toolpath-path-too-short = il contorno #{ $contour } è troppo corto da seguire ({ $length }); saltato
toolpath-groove-no-angle = l'utensile { $tool } è una fresa a V senza angolo: la larghezza della scanalatura è sconosciuta
toolpath-groove-past-cone = la scanalatura a V è più profonda del cono della fresa a V: larga { $width } in alto, con pareti dritte sopra il cono
toolpath-groove-width = la scanalatura a V è larga { $width } in alto
toolpath-holes-not-circles = { $count ->
    [one] saltata { $count } forma che non è un cerchio
   *[other] saltate { $count } forme che non sono cerchi
}
# $diameter is the holes' diameter, $tool the tool's, $within how far they may differ.
toolpath-holes-not-drill-size = { $count ->
    [one] foro da { $diameter }: non della misura della punta da { $tool } (entro { $within }); non forato
   *[other] { $count } fori da { $diameter }: non della misura della punta da { $tool } (entro { $within }); non forati
}
toolpath-holes-too-small-to-bore = { $count ->
    [one] foro da { $diameter }: troppo piccolo per la foratura elicoidale con la fresa da { $tool }; non tagliato
   *[other] { $count } fori da { $diameter }: troppo piccoli per la foratura elicoidale con la fresa da { $tool }; non tagliati
}
toolpath-holes-core = { $count ->
    [one] { $count } foro lascia
   *[other] { $count } fori lasciano
} un nucleo libero (foro più del doppio dell'utensile); fissalo o usa una tasca

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
