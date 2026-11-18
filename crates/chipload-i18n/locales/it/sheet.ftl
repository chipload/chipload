# The setup sheet for the operator (crates/chipload-pipeline/src/sheet/,
# docs/specs/setup-sheet.md): an HTML page printed next to the machine. A length argument
# brings its own unit ("6 mm", "0.25 in"); $start, $end, $x, $y, $width, $step, $radius and
# $feed are numbers alone, in the sheet's units, as are $z and $peck where said. Names
# (project, tools, parts, operations, layers, files) are never translated.

sheet-title = Scheda di attrezzaggio: { $project }
# Only some of the project's parts were exported.
sheet-title-parts = Scheda di attrezzaggio: { $project }, { $count ->
    [one] pezzo
   *[other] pezzi
} { $parts }
# The line under the title, its parts joined with " · ".
# With several setups, after the title: which one this sheet is for.
sheet-setup = piazzamento «{ $name }»
sheet-post = post { $post }
sheet-written-by = scritta da { $by }
# $length is "mm" or "in", $feed "mm/min" or "in/min".
sheet-units = lunghezze in { $length }, avanzamenti in { $feed }
sheet-gcode-inch = G-code in pollici (G20)

## Stock and zero
sheet-stock-and-zero = Grezzo e zero
sheet-stock = Grezzo
# This text is HTML: keep &times; (the × sign). $width is a number alone.
sheet-stock-size = { $width } &times; { $length }, spessore { $thickness }
sheet-back-side = Retro
# $back is the setup that cuts the board's back.
sheet-back-side-value = { $back } lavora il retro dopo aver capovolto la tavola. Lascia la tavola dov'è finché questo piazzamento non è finito.
sheet-turn-heading = Capovolgi la tavola
sheet-turn-left-right = Capovolgi la tavola da sinistra a destra, attorno al suo asse verticale: il bordo sinistro va a destra, e la faccia che era sotto ora è sopra.
sheet-turn-top-bottom = Capovolgi la tavola dal basso in alto, attorno al suo asse orizzontale: il bordo in basso va in alto, e la faccia che era sotto ora è sopra.
sheet-turn-pins = Metti una spina da { $diameter } in ogni foro per spina, fino nel piano di sacrificio.
sheet-turn-pins-down = Affonda le spine almeno { $depth } sotto la faccia del retro: il retro viene spianato sopra di esse.
sheet-mirrored = Specchiato
sheet-mirrored-left-right = { $part } è lavorato specchiato da sinistra a destra, rispetto alla linea centrale delle sue forme: l’immagine speculare del disegno.
sheet-mirrored-top-bottom = { $part } è lavorato specchiato dall’alto in basso, rispetto alla linea centrale delle sue forme: l’immagine speculare del disegno.
sheet-placed = Collocato
sheet-placed-moved = { $part } è tagliato spostato di { $dx }, { $dy } rispetto a dove è disegnato.
sheet-placed-turned = { $part } è tagliato ruotato di { $angle }° attorno al centro delle sue forme e spostato di { $dx }, { $dy } rispetto a dove è disegnato.
sheet-nested = Annidato
sheet-nested-text = { $count } istanze annidate a { $gap } l’una dall’altra e a { $margin } dal bordo; i tagli occupano { $width } × { $height } della tavola di { $board_width } × { $board_height }.
sheet-inlay = Intarsio
sheet-inlay-steps = Una volta lavorato: incollare il tassello a faccia in giù nella tasca del piazzamento { $setup } ({ $name }), stringerlo o pressarlo finché non poggia sulle pareti, lasciar fare presa la colla, poi segare o piallare il retro della tavola del tassello fino alla superficie della base.
sheet-inlay-steps-no-pocket = Una volta lavorato: incollare il tassello a faccia in giù nella sua tasca, stringerlo o pressarlo finché non poggia sulle pareti, lasciar fare presa la colla, poi segare o piallare il retro della tavola del tassello fino alla superficie della base.
sheet-turn-no-dowels = { $front } non ha fori per spina: aggiungili prima di lavorarlo, o appoggia la tavola in un angolo.
sheet-turn-corner = Spingi la tavola nello stesso angolo della battuta usato per { $front }. Deve misurare esattamente { $width } × { $length }.
sheet-turn-picture = L'immagine mostra la tavola come sta sulla macchina, con il retro in alto.
sheet-copies = Istanze
sheet-part = Pezzo
sheet-dxf = DXF { $id }
# $placement says how the drawing was moved, turned and scaled.
sheet-dxf-placed = { $name } ({ $file }), { $placement }
sheet-image = Immagine { $id }
sheet-image-placed = { $file } sul layer { $layer }, larghezza { $width }, a X{ $x } Y{ $y }
sheet-text = Testo { $id }
# $text is the text's first words in quotes, $font its font.
sheet-text-placed = { $text } in { $font } sul layer { $layer }, altezza { $height }, a X{ $x } Y{ $y }

## Where X0 Y0 is, said on the stock
sheet-zero-centre = il centro del grezzo
sheet-zero-bottom-left = l'angolo in basso a sinistra del grezzo
sheet-zero-bottom-right = l'angolo in basso a destra del grezzo
sheet-zero-top-left = l'angolo in alto a sinistra del grezzo
sheet-zero-top-right = l'angolo in alto a destra del grezzo
# A zero away from the stock's bottom left corner: "25 mm right of and 10 mm above the
# stock's bottom left corner". $offset is one or two of the four below, joined with -and.
sheet-zero-from-corner = { $offset } rispetto all'angolo in basso a sinistra del grezzo
sheet-zero-right-of = { $distance } a destra
sheet-zero-left-of = { $distance } a sinistra
sheet-zero-above = { $distance } in alto
sheet-zero-below = { $distance } in basso
sheet-zero-and = { $first } e { $second }
sheet-zero-outside = { $zero } (fuori dal grezzo)
# Where Z0 is.
sheet-z-top = la superficie del grezzo
sheet-z-bottom = il fondo del grezzo (il piano di sacrificio)

## A file's tools. $what is the tool: "6 mm end mill, 2 flutes".
# $xy is where X0 Y0 is, $z where Z0 is (texts above).
sheet-load-tool = Monta l'utensile T{ $tool } ({ $what }). Zero X0 Y0: { $xy }. Zero Z0: { $z }.
sheet-tool-change-m6 = Cambio utensile (M6) a T{ $tool } ({ $what }), come lo gestisce il tuo controller.
sheet-change-tool = Cambia utensile: T{ $tool } ({ $what }). Imposta di nuovo Z0 con questo utensile: { $z }. Non toccare X0 Y0.
# These two texts are HTML: keep &ldquo; and &rdquo; (the quotes round the tool's name).
# $rpm is the spindle speed, $spindle one of the two -spindle- texts.
sheet-tool-named = Utensile T{ $tool }: &ldquo;{ $name }&rdquo;, { $what }, { $rpm } rpm in senso { $spindle }.
sheet-tool = Utensile T{ $tool }: { $what }, { $rpm } rpm in senso { $spindle }.
sheet-spindle-cw = orario (M3)
sheet-spindle-ccw = antiorario (M4)
# The parts of $what, joined with ", ".
sheet-end-mill = fresa da { $diameter }
sheet-corner-radius = raggio di spigolo { $radius }
sheet-drill = punta da { $diameter }
# $angle in degrees.
sheet-vbit-angle = fresa a V da { $angle }°, { $diameter }
sheet-vbit = fresa a V, { $diameter }
sheet-tip = punta { $diameter }
sheet-ball-nose = fresa sferica da { $diameter }
sheet-tapered-ball = fresa sferica conica da { $angle }°, { $diameter }
sheet-ball = sfera { $diameter }
sheet-flutes = { $count ->
    [one] { $count } tagliente
   *[other] { $count } taglienti
}

## The table of operations
sheet-column-operation = Lavorazione
sheet-column-depth = Profondità
sheet-column-feeds = Avanzamento / affondamento
sheet-column-entry = Entrata
sheet-column-tabs = Ponticelli
sheet-column-time = Tempo
# A part's row over its operations, numbered from 1.
sheet-part-row = Pezzo { $number }: { $name }
sheet-part-row-copies = Pezzo { $number }: { $name }, { $count ->
    [one] tagliato una volta
   *[other] tagliato { $count } volte
}
# An operation without a name.
sheet-operation = Lavorazione { $number }
# A drill's one feed: $feed is a number alone.
sheet-feed-down = { $feed } in discesa
# The kind column, its parts joined with ", ": "Profile, outside, climb, leaves 0.2 mm".
sheet-kind-profile = Contornatura
sheet-side-outside = esterno
sheet-side-inside = interno
sheet-side-on = sulla linea
sheet-kind-pocket = Tasca, anelli
sheet-climb = concorde
sheet-conventional = discorde
# $percent: the stepover in percent of the tool's diameter.
sheet-stepover = passo laterale { $percent } %
sheet-kind-vcarve = Incisione a V
sheet-kind-path = Segui percorso, sulla linea
sheet-kind-chamfer = Smusso di { $width }
sheet-kind-flute = Scanalatura, rampe { $start } / { $end }
sheet-ramp-straight = rampe dritte
sheet-ramp-curved = rampe curve
sheet-pattern-lines = Motivo, linee distanti { $spacing } a { $angle }°
sheet-pattern-cross = Motivo, tratteggio incrociato distante { $spacing } a { $angle }°, { $between }° tra le due
sheet-pattern-rings = Motivo, anelli distanti { $spacing }
sheet-pattern-waves = Motivo, onde distanti { $spacing } a { $angle }°, lunghe { $wavelength }, alte { $height }
sheet-pattern-dashes = Motivo, trattini lunghi { $length } con spazi di { $gap }, file distanti { $spacing } a { $angle }°
sheet-pattern-herringbone = Motivo, spina di pesce di tratti da { $length } distanti { $spacing } a { $angle }°
sheet-pattern-scales = Motivo, squame di raggio { $radius }, file distanti { $spacing } a { $angle }°
sheet-pattern-sunburst = Motivo, raggiera di tratti da { $length }, anelli distanti { $spacing }, { $pitch } attorno
sheet-pattern-inside = dentro il bordo
sheet-pattern-on-line = sulla linea
sheet-pattern-ramp = estremità in rampa di { $ramp }
sheet-kind-prism = Prisma
sheet-moulding-cove = Modanatura, guscio R { $radius }
sheet-moulding-roundover = Modanatura, arrotondamento R { $radius }
sheet-moulding-ogee = Modanatura, gola { $width } × { $depth }
sheet-moulding-bead = Modanatura, tondino R { $radius }
sheet-moulding-flute = Modanatura, scanalatura R { $radius }
sheet-moulding-shape = Modanatura, profilo di S{ $shape }
sheet-moulding-left = a sinistra del percorso
sheet-moulding-right = a destra del percorso
sheet-moulding-round-ends = estremità tonde
sheet-moulding-tapered-ends = estremità in rampa su { $length }
sheet-stepover-mm = passo { $stepover }
sheet-kind-holes-drilled = Fori, foratura con punta
sheet-kind-holes-bored = Fori, foratura elicoidale
sheet-kind-surface = Spianatura, tutto il grezzo, avanti e indietro (concorde e discorde), passo laterale { $percent } %
sheet-kind-surface-part = Spianatura, le sue forme più { $margin }, anelli, passo laterale { $percent } %
# $surface is the text above; $distance half the tool's diameter.
sheet-kind-surface-past = { $surface }; esce di { $distance } oltre i bordi del grezzo: tieni lontani i bloccaggi
sheet-kind-relief-zigzag = Rilievo del modello { $model }, linee a { $angle }° avanti e indietro, passo laterale { $percent } %
sheet-kind-relief-one-way = Rilievo del modello { $model }, linee a { $angle }° tutte in un verso, passo laterale { $percent } %
sheet-cuts-past = taglia { $distance } oltre la linea
sheet-leaves = lascia { $distance }
sheet-rest-after = ripresa dopo T{ $tool }
sheet-rest-after-size = ripresa dopo T{ $tool } ({ $diameter })
# $degrees: the sharpest corner angle that gets one.
sheet-dogbones = dogbone negli angoli fino a { $degrees }°
sheet-tbones = T-bone negli angoli fino a { $degrees }°
sheet-corner-choices = { $count ->
    [one] { $count } angolo scelto a parte
   *[other] { $count } angoli scelti uno per uno
}
# The depth column, its parts joined with ", ": "0 to 6, 2 per pass".
sheet-depths = da { $start } a { $end }
sheet-depths-at-most = da { $start } ad al massimo { $end }
sheet-depths-drill = da { $start } a { $end } (al vertice della punta)
sheet-pecks = a passi di { $peck }
sheet-one-go = in un'unica discesa
sheet-per-turn = { $step } per giro
sheet-per-pass = { $step } per passata
sheet-groove-width = larga { $width } in alto
sheet-one-pass = una passata
sheet-passes = { $count } passate
sheet-finish = finitura { $stock }
sheet-skin = strato sottile { $thickness }
sheet-pocket-finish = finitura: pareti { $wall }, fondo { $floor }
# The entry column.
sheet-entry-beside = discesa a lato del grezzo
sheet-entry-straight = discesa dritta
sheet-entry-plunge = affondamento
sheet-entry-ramp = rampa { $degrees }°
sheet-entry-helix = elica { $degrees }°
sheet-entry-helix-radius = elica r { $radius }, { $degrees }°
sheet-lead = raccordo r { $radius }
# The tabs column: $count tabs, $width and $height numbers alone.
sheet-tabs = { $count } × { $width } di larghezza, { $height } di altezza
sheet-flanks = fianchi { $degrees }°
sheet-tabs-none = nessuno
# Where the tabs are, when not evenly round the contour.
sheet-tabs-along-x = lungo X (bordi inferiore e superiore)
sheet-tabs-along-y = lungo Y (bordi sinistro e destro)
# Under the table: how deep the tool goes. $z is a number alone; the parts are joined with ", ".
sheet-cuts-down = Scende fino a Z{ $z }: { $depth } sotto la superficie del grezzo
sheet-into-spoilboard = { $depth } dentro il piano di sacrificio
sheet-through = attraversa tutto il grezzo
sheet-sticks-out = { $cut }. Verifica che l'utensile sporga abbastanza.

## Watch out: the notes and the checker's findings
sheet-watch-out = Attenzione
sheet-nothing-reported = Nessuna segnalazione.
# $note is the post-processor's own note, as its file writes it.
sheet-post-note = Post { $post }: { $note }
sheet-check = Controllo: { $finding }

## The picture's legend
sheet-legend-file = File { $number } (T{ $tool })
sheet-legend-tabs = ponticelli
sheet-legend-grey = Grigio: i pezzi. Solo movimenti di taglio, nessun rapido.

## The time
sheet-time = Tempo
sheet-pieces = Tasselli
sheet-pieces-summary = { $count } tasselli, { $columns } × { $rows }, uniti con { $joint ->
    [knobs] denti a puzzle
   *[fingers] dita
} di { $size }. Ogni tassello si lavora da una tavola propria, qui sotto.
sheet-piece = Tassello { $name }
sheet-piece-mark = Scriva { $name } a matita sul retro prima di sbloccarlo.
sheet-pieces-glue-title = Incollaggio
sheet-pieces-glue = Provi prima i tasselli a secco: gli incastri hanno { $clearance } di gioco per la colla. Incolli i tasselli di ogni fila, poi unisca le file.
