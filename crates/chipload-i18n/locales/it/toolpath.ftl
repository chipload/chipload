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

toolpath-error-cancelled = Generazione annullata
# An error after where it happened: $context is an operation's label, or one of the
# context texts below.
toolpath-error-context = { $context }: { $error }
toolpath-error-label-contour = { $label }, contorno #{ $contour }
# $part is a part's name, $copy the copy's number.
toolpath-error-part-copy = pezzo { $part }, copia { $copy }
toolpath-error-copy = copia { $copy }
# $at is an operation's number in the cut list ("3", "3.1" for a rest pass).
toolpath-error-no-operation = Nessuna lavorazione { $at } nel lavoro

## Errors: selecting shapes

# $shapes are shape numbers ("S4, S7").
toolpath-error-shape-missing = la forma { $shapes } non fa parte di nessun contorno (eliminata, oppure un segmento duplicato o di lunghezza nulla rimosso dall'importazione)
# $layers are layer names, separated by commas.
toolpath-error-partly-selected = il contorno #{ $contour } è selezionato solo in parte (layer { $layers }); correggi il disegno o il selettore
# $reason is why the drawing's contour can't be machined, as the DXF import found it (in English).
toolpath-error-contour-invalid = il contorno #{ $contour } non è valido: { $reason }
# $selector is what the operation selects, built from the toolpath-error-selector texts.
toolpath-error-selects-nothing = il selettore { $selector } non corrisponde a nulla
# $contours are contour numbers ("#3, #4").
toolpath-error-selects-open = il selettore { $selector } corrisponde solo a contorni aperti ({ $contours }); questa lavorazione richiede contorni chiusi
toolpath-error-selector-layer = layer '{ $layer }'
toolpath-error-selector-shapes = forme { $shapes }
toolpath-error-selector-and = { $first } e { $second }
# $id is the number of a DXF, a picture or a text in the project.
toolpath-error-selector-dxf = DXF { $id }
toolpath-error-selector-image = immagine { $id }
toolpath-error-selector-text = testo { $id }
# $shapes are the shapes left out ("S3, S5").
toolpath-error-selector-but = { $source } tranne { $shapes }
toolpath-error-selector-of = { $what } di { $source }
# $a and $b are contour numbers.
toolpath-error-same-shape = i contorni #{ $a } e #{ $b } sono la stessa forma; selezionane solo uno

## Errors: operations and tools

toolpath-error-pass-depths = le profondità devono rispettare 0 <= inizio ({ $start }) < fine ({ $end }) e step_down ({ $step_down }) deve essere positivo
# $tool is a tool's number.
toolpath-error-rest-tool = ripresa dopo l'utensile { $tool }, che il progetto non ha
# $diameter is the hole's, $tool the tool's diameter.
toolpath-error-blind-bore = il foro da { $diameter } è cieco e più del doppio dell'utensile da { $tool }; usa una tasca
# $tool is the tool's number, $type its kind as written in the project file (endmill, drill, vbit).
toolpath-error-no-hole-fits = nessun cerchio selezionato si può tagliare con l'utensile { $tool } ({ $type } da { $diameter })
toolpath-error-no-depth = nessuna profondità a cui tagliare
toolpath-error-no-angle = l'utensile { $tool } non ha un angolo
# $got is an angle in degrees, without its unit.
toolpath-error-vbit-angle = l'angolo della fresa a V deve essere più di 0 e meno di 180 gradi, trovato { $got }
# $got is a length, without its unit.
toolpath-error-stepover = il passo laterale deve essere positivo, trovato { $got }

## Errors: profiles, ramps and tabs

toolpath-error-ring-no-length = un anello di contornatura non ha lunghezza
toolpath-error-lap-no-course = un giro sulla parete non ha percorso
# $angle is in degrees, without its unit; $length is how far the ring goes around.
toolpath-error-ring-ramp = L'anello è troppo corto per scendere in rampa di { $drop } a { $angle }° ({ $length } di giro); usa un angolo di rampa più ripido o un affondamento dritto con un utensile che taglia al centro
# $reach is how much of the path the ramp may use.
toolpath-error-ramp-reach = Il percorso è troppo corto per scendere in rampa di { $drop } a { $angle }° (solo { $reach } su cui fare la rampa); usa un angolo di rampa più ripido o un affondamento dritto con un utensile che taglia al centro
toolpath-error-path-ramp = Il percorso è troppo corto per scendere in rampa di { $drop } a { $angle }° ({ $length } di lunghezza); usa un angolo di rampa più ripido o un affondamento dritto con un utensile che taglia al centro
# $angle is in degrees, without its unit.
toolpath-error-helix-turns = un'elica di raggio { $radius } a { $angle }° richiederebbe { $turns } giri per scendere di { $drop }; usa un raggio più grande o un angolo più ripido
# $offset is how far inside the line the tool's centre would run.
toolpath-error-tool-does-not-fit = L'utensile non entra nel contorno (il suo centro passerebbe { $offset } all'interno della linea)
toolpath-error-offset-verification = La verifica dell'offset non è riuscita (percorso non a un raggio utensile dal contorno); il contorno potrebbe essere degenere
# $farthest is how far the finishing lap runs from the roughing path.
toolpath-error-roughing-reach = la sgrossatura non raggiunge parte della parete (il giro di finitura passa a { $farthest } dal percorso di sgrossatura, un raggio utensile o più): usa meno sovrametallo di finitura o un utensile più piccolo
# $fits is how many tabs would fit.
toolpath-error-tabs-fit = { $count } ponticelli da { $width } non entrano in un percorso di { $length } ({ $fits ->
    [0] non ne entra nemmeno uno
   *[other] ne entrano al massimo { $fits }
})
# $from is a distance along the ring without its unit, $to one with it.
toolpath-error-tab-carry = impossibile riportare un ponticello (da { $from } a { $to } lungo l'anello) sull'anello di sgrossatura
# $what is one of the three texts after it.
toolpath-error-tab-verification = La verifica del percorso dei ponticelli non è riuscita: { $what }
# $z is a height, $at a distance along the ring.
toolpath-error-tab-below = Z{ $z } a { $at } lungo l'anello è sotto il ponticello
toolpath-error-tab-back = l'utensile torna indietro lungo l'anello a { $at }
toolpath-error-tab-end-below = Z{ $z } alla fine del ponticello, { $end } lungo l'anello, è sotto la sommità del ponticello Z{ $top }
# $label is the operation's label; $at the corner's point.
toolpath-error-corner-leg = { $label }: il tratto all'angolo { $at } non è entrato nell'anello come andata e ritorno
toolpath-error-corner-cuts = { $label }: { $style ->
    [dogbone] il dogbone
   *[tbone] il T-bone
} all'angolo { $at } taglierebbe nel contorno #{ $contour }; scegli altro per questo angolo, abbassa l'angolo max degli angoli o cambia il disegno
toolpath-error-corner-cuts-own = { $label }: { $style ->
    [dogbone] il dogbone
   *[tbone] il T-bone
} all'angolo { $at } taglierebbe in un'altra parete del suo stesso contorno #{ $contour }; scegli altro per questo angolo (un T-bone lungo l'altra parete, o arrotondato) o cambia il disegno
toolpath-corner-choice-unmatched = { $label }: l'angolo scelto in { $at } non è più un angolo delle sue forme (il disegno è cambiato?); rimuovi la scelta o scegli di nuovo
# $x and $y are a point's coordinates, without their unit.
toolpath-error-arc-off-circle = uscita ad archi: ({ $x }, { $y }) non è sul cerchio a cui era associato

## Errors: pockets

# $contours are contour numbers ("#3, #4"): the pocket's boundary and its islands.
toolpath-error-islands-enclose = contorni { $contours }: isole sovrapposte racchiudono un'area tra loro, cosa che le tasche non supportano ancora
toolpath-error-pocket-no-fit = contorni { $contours }: l'utensile (diametro { $diameter }) non entra nella tasca
toolpath-error-pocket-no-fit-clearance = contorni { $contours }: l'utensile (diametro { $diameter }, maggiorazione { $clearance }) non entra nella tasca
toolpath-error-not-nested = contorni { $contours }: gli offset della tasca non sono annidati
toolpath-error-too-many-rings = contorni { $contours }: più di { $most } anelli di tasca
toolpath-error-pocket-verification = contorni { $contours }: la verifica del percorso della tasca non è riuscita (troppo vicino a una parete o a un'isola); il disegno potrebbe essere degenere
toolpath-error-nothing-to-clear = Niente da svuotare: le isole selezionate coprono i contorni esterni selezionati (contorni { $contours })
toolpath-error-island-outside = l'offset della tasca ha prodotto un'isola fuori da ogni area
toolpath-error-shrink-hole = l'offset della tasca ha creato un foro restringendo un singolo anello
toolpath-error-open-loop = l'offset della tasca ha restituito un anello aperto dove ne era atteso uno chiuso
toolpath-error-rest-outside = l'area di ripresa dei residui è fuori dalla tasca
toolpath-error-left-hole-outside = materiale residuo: un foro fuori da ogni pezzo
toolpath-error-left-holes-enclose = materiale residuo: dei fori racchiudono un'area
toolpath-error-left-wrong-area = materiale residuo: un'operazione booleana ha dato l'area sbagliata
toolpath-error-left-invalid = materiale residuo: geometria non valida dalla libreria di offset

## Errors: V-carving, centre lines and letters

# What a centre line is for, inside the texts below.
toolpath-error-medial-name = { $name ->
    [vcarve] dell'incisione a V
   *[centre-line] dei tratti
}
# $name is toolpath-error-medial-name; $error the Voronoi library's own words.
toolpath-error-medial-failed = contorni { $contours }: impossibile costruire la linea mediana { $name } ({ $error }); il contorno tocca o attraversa se stesso?
toolpath-error-medial-cell = linea mediana { $name }: una cella senza la sua corda
# $reach is how far from the origin the centre line works.
toolpath-error-medial-far = un contorno { $name } in { $at } è troppo lontano dall'origine del disegno (la linea mediana funziona entro ±{ $reach })
toolpath-error-medial-step = La linea mediana { $name }
toolpath-error-vcarve-verification = contorni { $contours }: verifica dell'incisione a V non riuscita: il taglio attraverserebbe il contorno vicino a { $at }
toolpath-error-join-gave = l'unione di due contorni sovrapposti ha dato { $outlines } contorni
toolpath-error-join-changed = l'unione di contorni sovrapposti ha cambiato ciò che è pieno vicino a { $at }
toolpath-error-joined-cross = i contorni uniti si attraversano
toolpath-error-outline-unreadable = un contorno che la libreria geometrica non riesce a leggere

## Errors: copies and parts

# $width is the cut's width; $got the pitch, without its unit.
toolpath-error-copies-dx = Copie sovrapposte: con più colonne il passo dx deve essere almeno { $width } (il taglio è largo { $width }), trovato { $got }
toolpath-error-copies-dy = Copie sovrapposte: con più righe il passo dy deve essere almeno { $height } (il taglio è alto { $height }), trovato { $got }
# $other and $copy are copies' numbers; $dx and $dy the pitch, without its unit.
toolpath-error-copies-overlap = Copie sovrapposte: le copie { $other } e { $copy } si tagliano a vicenda; allontanale (passo dx { $dx }, dy { $dy })
# $part and $other_part are parts' names; $x0 and $y0 are without their unit.
toolpath-error-parts-overlap = Il pezzo "{ $part }" copia { $copy } si sovrappone al pezzo "{ $other_part }" copia { $other_copy } (da X{ $x0 } a { $x1 }, da Y{ $y0 } a { $y1 }): allontana le copie o cambia la griglia

## Errors: inside the geometry library

# $what is toolpath-error-step or toolpath-error-medial-step.
toolpath-error-library-failed = { $what }: errore all'interno della libreria geometrica
toolpath-error-step = { $step ->
    [pocket-offset] Offset della tasca
    [joining-letters] Unione delle lettere
    [checking-joined-letters] Verifica delle lettere unite
    [material-left-union] Unione del materiale residuo
    [material-left-offset] Offset del materiale residuo
    [material-left-boolean] Operazione booleana del materiale residuo
    [comparing-shapes] Confronto delle forme
    [merging-islands] Unione delle isole
    [subtracting-an-island] Sottrazione di un'isola
   *[offset] Offset
}
