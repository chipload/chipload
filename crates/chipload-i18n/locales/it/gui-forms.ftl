# The property forms: an operation, the job settings, tools, DXFs, pictures, texts, parts (crates/chipload-gui/src/app/forms/).

# Depths, shared by the operations. "Through": a button that sets the end depth through the stock.
forms-start-depth = Prof. iniziale
forms-start-depth-hover = Dove inizia il taglio, in giù dalla superficie del grezzo
forms-end-depth = Prof. finale
forms-end-depth-hover = Dove finisce il taglio, in giù dalla superficie del grezzo
forms-groove-width = Larghezza scanalatura
forms-groove-width-hover = Con una fresa a V: quanto è larga la scanalatura in superficie. Cambiarla imposta la profondità finale.
forms-through = Passante
forms-through-hover = Taglia attraverso il grezzo: la profondità finale è il suo spessore più la profondità sotto, e segue lo spessore quando cambia
forms-through-below = Sotto il grezzo
forms-through-below-hover = Quanto scende il taglio sotto la faccia inferiore del grezzo, nel piano di sacrificio. Il grezzo del piazzamento fissa il massimo ({ $depth })
forms-oversize = Maggiorazione
forms-oversize-disabled = Una contornatura sulla linea non ha un lato su cui tagliare di più
forms-step-down = Prof. di passata
# Indented under "Passes": the depth of each pass.

# Cutting direction (milling).
forms-direction = Direzione
forms-climb = concorde
forms-conventional = discorde

# Tabs' sloped sides.
forms-flank-angle = angolo fianchi
forms-flank-angle-hover = Quanto scendono ripidi i fianchi del ponticello dalla sua sommità piana. 90° è un ponticello dritto: l'utensile sale e scende in verticale. Sotto quel valore l'utensile segue la pendenza, quindi non affonda mai dopo un ponticello. Scendere lungo un fianco è una rampa: con un utensile che non taglia al centro, non più ripida dell'angolo di rampa.
forms-flank-straight = dritto
forms-flank-feet = base { $length } per lato

# How the tool goes down into the material.
forms-entry = Ingresso
forms-entry-hover = Come l'utensile scende nel materiale. Una rampa scende lungo il percorso (una contornatura scende a spirale lungo il suo anello, un segui percorso scende andando avanti e indietro, una tasca va in rampa avanti e indietro), un'elica (tasche) lungo un piccolo cerchio, così l'utensile non deve tagliare al centro; entrambe usano l'avanzamento di taglio.
forms-entry-plunge = affondamento dritto
forms-entry-ramp = rampa
forms-entry-helix = elica
forms-entry-ramp-summary = rampa { $angle }
forms-entry-helix-summary = elica r { $radius }, { $angle }
forms-entry-with-lead = { $entry }, raccordo { $radius }
forms-helix-radius = raggio elica
forms-helix-radius-hover = Del cerchio seguito dal centro dell'utensile: minore del raggio dell'utensile, così non resta in piedi un nucleo
forms-ramp-angle-sub = angolo rampa
forms-ramp-angle = Angolo rampa
forms-ramp-angle-hover = L'utensile scende a spirale lungo il contorno, mai più ripido di questo e mai più di una profondità di passata per giro: nessun affondamento nel pezzo. Ogni passata scende con questo angolo, poi prosegue in piano; un angolo più ripido accorcia la rampa.

# The operation kinds, in the form's title.
forms-kind-profile = Contornatura
forms-kind-pocket = Tasca
forms-kind-vcarve = Incisione a V
forms-kind-path = Segui percorso
forms-kind-chamfer = Smusso
forms-kind-holes = Fori
forms-kind-surface = Spianatura
forms-kind-relief = Rilievo
forms-operation-number = Lavorazione { $number }

forms-name = Nome
forms-geometry = Geometria
forms-whole-stock = Tutto il grezzo
forms-whole-stock-hover = Una spianatura lavora tutto il grezzo, una volta, qualunque siano le istanze
forms-use-selection = Usa selezione
forms-use-selection-count = Usa selezione ({ $count })
forms-use-selection-hover = Sostituisci con i contorni selezionati nell'area di disegno.
forms-use-selection-disabled = Seleziona prima i contorni nell'area di disegno

# A profile's side of the line.
forms-side = Lato
forms-side-inside = interno
forms-side-outside = esterno
forms-side-on = sulla linea
forms-side-choose = — scegli —

# Section titles and their folded summaries. "Cut": how the operation cuts (noun).
forms-section-cut = Taglio
forms-profile-cut-summary = { $start }–{ $end } { $unit }, passate di { $step } { $unit }, { $direction }, rampa a spirale { $ramp }°
forms-pocket-cut-summary = { $start }–{ $end } { $unit }, passate di { $step } { $unit }, { $direction }
forms-path-cut-summary = { $start }–{ $end } { $unit }, passate di { $step } { $unit }, sulla linea
forms-chamfer-side-hover = Dov'è lo sfrido: fuori dalla forma (contorno di un pezzo) o dentro (bordo di un foro o di una tasca)
forms-chamfer-cut-summary = largo { $width }, profondo { $depth }, { $direction }
forms-chamfer-width = Larghezza
forms-chamfer-width-hover = La larghezza dello smusso in superficie, dalla linea verso il pezzo. Il fianco della fresa a V passa sulla linea e la punta resta nello sfrido
forms-chamfer-depth = Profondità
forms-chamfer-depth-value = { $depth } con la fresa a { $angle }°
forms-chamfer-needs-vbit = serve una fresa a V con un angolo
forms-chamfer-start-hover = Dov'è lo spigolo, scendendo dalla faccia superiore: 0 per la faccia superiore, di più per lo spigolo di un gradino lavorato prima
forms-chamfer-passes = In passate
forms-chamfer-passes-hover = No: tutto lo smusso in una passata. Sì: passate uguali non più profonde di questo
forms-surface-cut-summary = { $start }–{ $end } { $unit }, passate di { $step } { $unit }, passo laterale { $stepover } × D
forms-relief-cut-summary = { $start }–{ $end } { $unit }, linee a { $angle }°, passo laterale { $stepover } × D
forms-relief-angle = Linee a
forms-relief-zigzag = Avanti e indietro
forms-vcarve-cut-one-pass = da { $start }, al massimo { $end } di profondità, una passata
forms-vcarve-cut-passes = da { $start }, al massimo { $end } di profondità, { $count } passate

# A pocket's clearing, in rings.
forms-section-clearing = Svuotamento
forms-clearing-summary = anelli, passo laterale { $stepover } × D
forms-clearing-summary-rest = anelli, passo laterale { $stepover } × D, ripresa dopo T{ $tool }
forms-stepover = Passo laterale
forms-stepover-hover = Di quanto si sposta ogni anello, come frazione del diametro dell'utensile
forms-surface-stepover-hover = Di quanto si sposta di lato ogni passata, come frazione del diametro dell'utensile
forms-surface-end-depth-hover = Quanto si toglie dalla superficie, in giù dalla superficie del grezzo
forms-section-entry-lead = Ingresso e raccordo
forms-section-entry = Ingresso

# V-carving.
forms-vcarve-start-depth-hover = La superficie incisa, in giù dalla superficie del grezzo (0: la superficie; di più per incidere sul fondo di una tasca)
forms-vcarve-end-depth-hover = La profondità massima della punta, dalla superficie del grezzo (end_depth nel file). I tratti più larghi vengono incisi fino a lì e segnalati
forms-vcarve-passes-hover = Quante passate fino alla profondità finale. Ognuna toglie quanto le altre, quindi diventano meno profonde scendendo
forms-vcarve-pass-depths = a { $depths }
forms-passes = Passate
forms-depth-passes = Passate in profondità
forms-depth-passes-hover = Come la profondità è divisa in passate: carico uguale (ognuna toglie quanto le altre, quindi diventano meno profonde scendendo) o una profondità di passata normale (passi uguali, come in una tasca; ogni passata toglie più della precedente)
forms-depth-passes-equal-load = Carico uguale
forms-vcarve-step-down-hover = Quanto scende al massimo ogni passata. Le passate sono uguali, l'ultima alla profondità finale
forms-ignore-kinks = Ignora le pieghe
forms-ignore-kinks-hover = Gli angoli dove il contorno gira meno di questo angolo non hanno un ramo della linea mediana, così la fresa a V non risale in ogni piega di una curva. A 20° una piega lascia al massimo pochi centesimi di millimetro; 0° incide ogni angolo
forms-section-plug = Tassello
forms-plug-summary = contorno a { $start }, spazio per la colla { $gap }
forms-plug-start = Profondità iniziale del tassello
forms-plug-start-hover = A che profondità sta il contorno del tassello nella sua tavola: la sua faccia sta così in fondo nella tasca. Più della profondità iniziale, meno di quella finale
forms-plug-gap = Spazio per la colla
forms-plug-gap-hover = Quanto più a fondo del contorno si intaglia la tavola: la profondità finale meno quella iniziale del tassello. Il retro del tassello sporge tanto sopra la base

# The operation's tool. "T3" is tool number 3.
forms-tool = Utensile
forms-tool-not-in-project = T{ $tool } (non nel progetto)
forms-from-library = Dalla tua libreria:
forms-use-cutting-values = Usa i valori di taglio dell'utensile
forms-use-cutting-values-hover = Dalla libreria: avanzamento { $feed } / affondamento { $plunge } { $feed_unit }, prof. di passata { $step_down }, passo laterale { $stepover }

forms-section-feeds = Avanzamenti
forms-feed = Avanzamento
forms-chip-load = Avanzamento per dente
forms-chip-load-no-flutes = taglienti non impostati
forms-chip-load-no-rpm = velocità mandrino non impostata
forms-chip-load-missing-hover = Impostali nelle proprietà dell'utensile: Utensili nel browser, poi l'utensile.
forms-flutes = Taglienti
forms-flutes-hover = I taglienti dell'utensile, per l'avanzamento per dente. Non spuntato: sconosciuto.
forms-plunge-feed = Affondamento

# Rest machining: clear only what a larger tool left.
forms-rest-after = Ripresa dopo
forms-rest-after-hover = Rimuovi solo ciò che un utensile più grande lascia in queste forme: gli angoli che lascia arrotondati e le cave in cui non entra. Esegui prima la tasca con quell'utensile
forms-rest-none = nessuna
forms-rest-endmill = fresa { $diameter }

# Square inside corners.
forms-corners = Angoli
forms-corners-hover = Gli angoli interni che un utensile tondo lascia arrotondati: un dogbone entra nell'angolo lungo la bisettrice, un T-bone lungo la parete più lunga (l'intaglio è nella più corta), così un pezzo squadrato entra fino in fondo
forms-corners-round = arrotondati
forms-corners-dogbones = dogbone
forms-corners-tbones = T-bone
forms-corners-disabled = Una contornatura sulla linea non ha angoli interni
forms-corners-up-to = fino a
forms-corners-up-to-hover = Solo gli angoli acuti così o di più (anche gli angoli di 90° disegnati leggermente storti contano)
forms-corners-chosen = { $count ->
    [one] { $count } angolo scelto a parte
   *[other] { $count } angoli scelti uno per uno
}
forms-corners-chosen-hover = Angoli cliccati nell'area di disegno e scelti a parte, qualunque cosa dica la riga sopra
forms-corners-reset = Ripristina
forms-corners-reset-hover = Ogni angolo torna ad avere ciò che dice la riga sopra

# Holes: drilled, or bored with an endmill round a helix.
forms-holes-drilled-pecks = forati a passi di { $peck }
forms-holes-drilled-once = forati in un colpo
forms-holes-bore = foratura elicoidale, { $step } per giro, { $direction }
forms-holes-cut-summary = { $start }–{ $end } { $unit }, { $method }
forms-holes-drill-end-depth-hover = Dove finisce la punta del foretto, in giù dalla superficie del grezzo: il diametro pieno si ferma circa 0.3 × il diametro della punta più in alto
forms-pecks = A passi
forms-pecks-hover = Fora per questa profondità alla volta, uscendo dal foro tra un passo e l'altro per scaricare il truciolo
forms-in-pecks = a passi
forms-peck = passo
forms-step-per-turn = Discesa per giro
forms-step-per-turn-hover = Il massimo di cui l'elica scende in un giro
forms-bore-ramp-angle-hover = L'elica non è mai più ripida di questo
forms-drill-feeds-summary = { $feed } { $unit } in discesa
forms-drill-feed = Avanzamento foratura

# How a profile holds its part.
forms-holding-none = nessuno
forms-holding-tabs = ponticelli
forms-holding-skin = strato sottile
forms-holding-skin-tabs = strato sottile e ponticelli
forms-holding-summary-tabs = { $holding }: { $count } × { $width } { $unit }, alti { $height } { $unit }
forms-holding-summary-skin = { $summary }, strato { $thickness }
forms-section-holding = Fissaggio
forms-hold-the-part = Fissa il pezzo
forms-hold-the-part-hover =
    Come un pezzo ritagliato resta al suo posto.
    Ponticelli: ponti lasciati sul fondo del taglio; il pezzo resta attaccato finché non lo stacchi tu.
    Strato sottile: uno strato sottile lasciato tutto intorno durante la sgrossatura, poi un ultimo giro dopo ogni anello della lavorazione lo taglia. Durante quel giro il pezzo è libero: aggiungi ponticelli se i pezzi non devono muoversi in quel momento.
forms-tab-count = n. ponticelli
# Where a profile's tabs go (docs/specs/tabs-along-grain.md): X and Y name the edges.
forms-tab-placement = disposizione
forms-tab-placement-hover = Dove ogni contorno riceve i suoi ponticelli, se non li hai posti a mano
forms-tab-around = tutto intorno
forms-tab-around-hover = Distribuiti uniformemente intorno al contorno, spostati dagli angoli
forms-tab-along-x = lungo X
forms-tab-along-x-hover = Sui bordi inferiore e superiore del contorno, che vanno in X, metà su ciascuno. Con la venatura lungo i bordi i ponticelli si staccano facilmente; di traverso tengono di più
forms-tab-along-y = lungo Y
forms-tab-along-y-hover = Sui bordi sinistro e destro del contorno, che vanno in Y, metà su ciascuno. Con la venatura lungo i bordi i ponticelli si staccano facilmente; di traverso tengono di più
forms-tab-width = larghezza
forms-tab-height = altezza
forms-tabs-placed = { $count ->
    [one] { $count } ponticello posto a mano
   *[other] { $count } ponticelli posti a mano
}
forms-tabs-placed-hover = Ponticelli trascinati, aggiunti o rimossi nell'area di disegno; i contorni senza ricevono il numero sopra, distribuiti uniformemente
forms-tabs-reset = Ripristina
forms-tabs-reset-hover = I ponticelli di ogni contorno vengono riposizionati secondo la disposizione
forms-skin-thickness = spessore strato
forms-skin-thickness-hover = Lasciato sopra la profondità finale fino all'ultimo giro: al massimo una profondità di passata, e più sottile dei ponticelli

# A profile's finishing pass.
forms-finish-none = nessuna
forms-finish-summary = { $stock } lasciati, poi un giro
forms-finishing-pass = Passata di finitura
forms-finishing-pass-hover = Sgrossa lasciando questo spessore sulla parete, poi toglilo in un giro leggero a piena profondità: una parete più pulita senza segni delle passate. Il giro scende nella cava della sgrossatura, mai sulla parete finita.
forms-finishing-lap = giro di finitura
forms-finishing-disabled = Una contornatura sulla linea non ha pareti da rifinire
# The material left on the wall for the finishing lap.
forms-finish-stock = sovrametallo
forms-pocket-finish-summary = pareti { $wall }, fondo { $floor }
forms-pocket-finish-hover = Sgrossa lasciando un po' sulle pareti e sul fondo, poi toglilo: una passata in piano sul fondo e un giro di ogni parete a piena profondità. Pareti più pulite senza gradini e una misura esatta per un accoppiamento.
forms-pocket-finish-tick = sgrossare, poi rifinire
forms-pocket-finish-rest = Una tasca di ripresa è già una ripassata leggera
forms-pocket-finish-wall = Sovrametallo pareti
forms-pocket-finish-wall-hover = Lasciato su ogni parete, anche sulle isole, per il giro a piena profondità
forms-pocket-finish-floor = Sovrametallo fondo
forms-pocket-finish-floor-hover = Lasciato sopra la profondità finale, per una passata in piano sul fondo; al massimo la profondità di passata. Una tasca passante non ha fondo: 0

# A pocket's lead-in and lead-out.
forms-lead = Raccordo
forms-lead-hover = Un arco verso la parete dalla tasca svuotata e di nuovo via, così l'utensile non si ferma mai contro la parete. Un raccordo senza spazio si riduce, o viene omesso (il report lo dice).
forms-lead-in-out = attacco e stacco
forms-lead-radius = raggio

# The G-code's X0 Y0.
forms-work-zero = Zero pezzo X Y
forms-work-zero-hover = Il punto che diventa X0 Y0 nel G-code. La geometria non si sposta.
forms-zero-drawing-origin = origine del disegno
forms-zero-stock-centre = centro del grezzo
forms-zero-stock-bottom-left = grezzo in basso a sinistra
forms-zero-stock-bottom-right = grezzo in basso a destra
forms-zero-stock-top-left = grezzo in alto a sinistra
forms-zero-stock-top-right = grezzo in alto a destra
forms-zero-point = punto
forms-zero-on-stock = Sul grezzo
forms-zero-on-stock-hover = Fai clic su un angolo o sul centro: lo zero resta lì quando cambia la dimensione del grezzo
forms-zero-drawing-origin-button = Origine del disegno
forms-zero-drawing-origin-hover = X0 Y0 dove il disegno ha il suo 0,0
forms-zero-at = Posizione
forms-zero-at-hover = Dove si trova lo zero, in coordinate del disegno. Digitare un valore lo rende un punto a sé.
forms-zero-note = X0 Y0 Z0 del G-code. Spostarlo non sposta nessun taglio, solo i numeri del programma. Nella vista, selezionalo e trascinalo: si aggancia agli angoli e al centro del grezzo e all'origine del disegno.

# An imported DXF.
forms-file = File
forms-units = Unità
forms-dxf-inch = pollici (convertiti in mm)
forms-shapes = Forme
forms-section-placement = Posizione, rotazione e scala
forms-update-dxf = Aggiorna da un DXF o SVG…
forms-update-dxf-hover = Una nuova versione del disegno: le lavorazioni mantengono i loro layer
# A button (verb).
forms-remove = Rimuovi
forms-remove-dxf-hover = Rimuovi questo file di disegno e le sue forme

# A picture traced into shapes.
forms-picture = Immagine
forms-picture-pixels = { $width } × { $height } pixel
forms-traced-as = Vettorizzata come
forms-traced-outlines = contorni
forms-traced-centre-lines = linee mediane
forms-position = Posizione
forms-picture-reset-position = L'angolo del riquadro in X0 Y0
forms-rotation = Rotazione
forms-instance-number = Istanza { $number }
forms-instance-cut-hover = Un’istanza che non si taglia resta nel progetto, disegnata tratteggiata.
forms-instance-cut-check = questa istanza
forms-instance-mirror-hover = Specchiata attorno al centro del riquadro del pezzo, prima di ruotarla. La concordanza resta concordanza.
forms-instance-angle-hover = Ruotata attorno al centro del riquadro del pezzo, gradi in senso antiorario. Nella vista la sua maniglia tonda la ruota; i quarti di giro si agganciano entro 3°, Alt ruota liberamente.
forms-instance-move-x = spostata X
forms-instance-move-y = spostata Y
forms-instance-move-hover = A che distanza si trova dal pezzo come disegnato, dopo specchiatura e rotazione. Trascini la sua ombra nella vista per spostarla.
forms-instance-note = Collegata al suo pezzo: le forme e le lavorazioni del pezzo si tagliano anche qui.
forms-picture-rotation-hover = Gradi in senso antiorario, qualsiasi angolo, attorno al centro dell'immagine. L'immagine viene vettorizzata così com'è, poi le sue forme vengono ruotate.
forms-turn-ccw-hover = 90° in senso antiorario
forms-turn-cw-hover = 90° in senso orario
forms-picture-reset-rotation = Come l'immagine (0°)
forms-width = Larghezza
forms-picture-width-hover = La larghezza dell'immagine prima della rotazione; l'altezza segue
forms-picture-height = altezza { $height }
forms-layer = Layer
forms-picture-layer-hover = Il layer delle sue forme, per le lavorazioni che lo selezionano
forms-trace-settings = Vettorizzazione…
forms-trace-settings-hover = Soglia, levigatura, contorni o linee mediane, con anteprima
forms-remove-picture-hover = Rimuovi l'immagine e le sue forme

# A text written in a built-in font.
forms-text = Testo
forms-text-hover = Cosa dice; Enter inizia una nuova riga
forms-font = Font
forms-text-font-missing = Il font «{ $font }» non è su questo computer. Il testo mantiene le sue lettere e viene fresato così; per cambiarne le parole o la dimensione, scegli un altro font. Spostarlo e ruotarlo funziona ancora.
forms-fonts-looking = Ricerca dei font…
forms-font-recent = Usati di recente
forms-font-builtin = Integrati
forms-font-computer = Su questo computer
forms-font-computer-none = Nessun altro font trovato. Installane uno o mettilo nella cartella fonts della tua cartella impostazioni.
forms-font-search = Cerca
forms-font-hover = Un font integrato: normale, corsivo o decorativo (Guida utente: Testo)
forms-height = Altezza
forms-text-height-hover = L'altezza di una lettera maiuscola
forms-anchor = Ancoraggio
forms-anchor-hover = Cosa indica Posizione: l'angolo in basso a sinistra del riquadro del testo, o il suo centro. Un testo centrato resta centrato quando cambiano parole, font o dimensione
forms-anchor-corner = angolo in basso a sinistra
forms-anchor-centre = centro
forms-text-reset-position = In X0 Y0
forms-text-rotation-hover = Gradi in senso antiorario, qualsiasi angolo, attorno al centro del testo
forms-text-reset-rotation = Orizzontale (0°)
forms-align = Allinea
forms-align-hover = Come si allineano le righe tra loro
forms-align-left = sinistra
forms-align-centre = centro
forms-align-right = destra
forms-letter-spacing = Spaziatura lettere
forms-letter-spacing-hover = Spazio in più tra le lettere; 0: quello del font
forms-line-spacing = Interlinea
forms-line-spacing-hover = Dalla base di una riga alla successiva, in altezze di maiuscola
# After the number: "1.2 × height".
forms-line-spacing-suffix = × altezza
forms-text-layer-hover = Il layer delle sue lettere, per le lavorazioni che lo selezionano
forms-size = Dimensione
forms-remove-text-hover = Rimuovi il testo e le sue lettere

# The job settings. "Stock": the material the parts are cut from.
forms-job-settings = Macchina
forms-section-stock = Grezzo
forms-stock-size = Dimensione
forms-thickness = Spessore
forms-material = Materiale
forms-material-hover = I tuoi avanzamenti per utensile, da Libreria > Materiali. Sceglierlo dà a ogni operazione, e a quelle aggiunte dopo, questo materiale e i suoi valori. Il grezzo resta com'è
forms-material-none = Nessuno
forms-material-not-in-library = { $name } (non nella tua libreria)
forms-material-from = Da { $name }
forms-material-from-hover = Avanzamenti, profondità di passata e passo laterale sono i valori del materiale per questo utensile
forms-material-changed = Cambiato rispetto a { $name }
forms-material-use = Valori di { $name }
forms-material-use-hover = Lavora con i valori del materiale per questo utensile
forms-material-save = Salva in { $name }
forms-material-save-hover = Tieni questi avanzamenti, profondità di passata e passo laterale come valori del materiale per questo utensile, per i prossimi lavori
forms-material-not-in = { $name } non ha valori per questo utensile
forms-material-no-values = { $name } (nessun valore per questo utensile)
forms-op-material-hover = I tuoi avanzamenti per utensile, da Libreria > Materiali: l'operazione prende i valori di questo materiale per il suo utensile, e li segue quando cambi utensile
forms-max-overcut = Prof. max sotto grezzo
forms-max-overcut-hover = Quanto un taglio può scendere sotto il grezzo (nel piano di sacrificio). I tagli entro questo limite danno un avviso; quelli più profondi vengono rifiutati.
forms-stock-position = Posizione grezzo
forms-stock-corner-at-origin = angolo nell'origine del disegno
forms-stock-centred-on-origin = centrato sull'origine del disegno
forms-stock-centred-on-drawing = centrato sul disegno
forms-stock-margin = margine
forms-stock-fits-hover = Il contorno del disegno più il margine su ogni lato, centrato sul disegno; segue il disegno
forms-stock-fit = Contorno + margine
forms-stock-fit-hover = Centra il grezzo sul contorno del disegno e dimensionalo come il contorno più un margine, seguendolo
forms-stock-fixed-size = Dimensione fissa, centrata
forms-stock-fixed-size-hover = Una dimensione che digiti, per un pannello che vuoi usare, centrata sul contorno del disegno
forms-drawing-size-note = Disegno { $size }
forms-zero-summary = X0 Y0 { $xy } · Z0 { $z }
forms-material-each-op = Ogni operazione può sceglierne uno proprio, sotto il suo utensile.
forms-drawing-size-hover = La dimensione del contorno del disegno (ciò che viene lavorato; senza la geometria di costruzione). Il contorno del taglio, con l'utensile, è nel rapporto dopo Calcola
forms-z-zero = Zero Z
forms-z-zero-top = superficie del grezzo
forms-z-zero-bottom = fondo del grezzo
forms-machine-summary = { $post }, Z sicura { $safe_z }, { $spindle }
forms-section-machine = Macchina
forms-from-profile = Dal profilo
forms-edit-here = Modifica qui
forms-edit-here-hover = Cambia i valori della macchina solo per questo progetto; resta collegata, e mostra cosa differisce
forms-safe-z-job = Z sicura (questo lavoro)
forms-safe-z-hover = La quota per i movimenti rapidi laterali, sopra il grezzo e i suoi morsetti

# The project's tools.
forms-tools = Utensili
forms-add-from-library = Aggiungi da libreria…
forms-new-tool = Nuovo utensile
forms-tool-not-used = non usato
# $operations: a list of operation numbers, "1, 3".
forms-tool-used-by = lavorazioni { $operations }
forms-tool-not-used-long = Non usato da nessuna lavorazione
forms-tool-cuts = Esegue le lavorazioni { $operations }
forms-remove-tool = Rimuovi utensile
forms-remove-tool-disabled = Usato dalle lavorazioni { $operations }; assegna prima loro un altro utensile
# An icon button that puts a value back.
forms-reset = Ripristina

# A DXF's position, rotation and scale.
forms-dxf-reset-position = Torna dove lo pone il file (rotazione e scala restano)
forms-bottom-left = In basso a sinistra
forms-bottom-left-hover = L'angolo del disegno a { $margin } dall'angolo in basso a sinistra del grezzo
# A button: centre the drawing on the stock.
forms-centre = Centra
forms-centre-hover = Il centro del disegno sul centro del grezzo
forms-dxf-rotation-hover = Gradi in senso antiorario rispetto al file come disegnato, qualsiasi angolo, attorno al centro del disegno
forms-dxf-reset-rotation = Torna all'angolo originale del file (0°)
forms-scale = Scala
forms-scale-hover = Rispetto al file come disegnato, attorno al centro del disegno. Dimensioni degli utensili, profondità e ponticelli non vengono scalati.
forms-dxf-reset-scale = Torna alla dimensione originale del file (× 1)

# A part: a group of operations cut one after the other.
forms-part-number = Pezzo { $number }
# Label of the on/off checkbox (verb): whether the part is cut.
forms-part-cut = Lavora
forms-part-cut-hover = Disattivato: il pezzo e le sue lavorazioni restano nel progetto, non lavorati
forms-part-cut-check = questo pezzo e le sue lavorazioni
forms-part-mirror = Specchia
forms-part-mirror-hover = Lavorare il pezzo specchiato rispetto alla linea centrale delle sue forme; il disegno resta com’è. Dentro resta dentro, in concordanza resta in concordanza
forms-part-mirror-none = Non specchiato
forms-active = Attivo
forms-active-hover = Le nuove lavorazioni vanno nel pezzo attivo. Clic destro su un pezzo per attivarlo
forms-active-yes = sì: le nuove lavorazioni vanno qui
forms-activate = Attiva
forms-operations = Lavorazioni
forms-operations-none = ancora nessuna
forms-copies-one = una
forms-copies = Istanze
forms-parts-note = I pezzi vengono lavorati uno dopo l'altro, ciascuno con le sue istanze.
forms-setup = Piazzamento
forms-setup-cut = Lavoralo
forms-setup-cut-hover = Disattivato: resta, ma nessuno dei suoi pezzi viene lavorato
forms-setup-cut-check = questo piazzamento e i suoi pezzi
forms-setup-parts = Pezzi
forms-setup-board = Tavola
forms-setup-board-hover = Il retro di un altro piazzamento è la stessa tavola capovolta: ne condivide il grezzo e lo zero pezzo, e i suoi tagli sono specchiati
forms-setup-board-own = Tavola propria
forms-setup-board-back = Retro di { $setup }
forms-setup-turn = Capovolgi
forms-setup-turn-hover = Come si capovolge la tavola: attorno al suo asse verticale (da sinistra a destra) o orizzontale (dal basso in alto)
forms-setup-turn-left-right = Da sinistra a destra
forms-setup-turn-top-bottom = Dal basso in alto
forms-setup-register = Riposiziona con
forms-setup-register-hover = Spine in fori sull'asse di capovolgimento rimettono la tavola esattamente a posto; un angolo di battuta richiede una misura esatta della tavola
forms-setup-register-pins = Spine
forms-setup-register-corner = Un angolo di battuta
forms-setup-dowels = Fori per spina
forms-setup-dowels-there = Lavorazione { $number }
forms-setup-add-dowels = Aggiungi fori per spina
forms-setup-add-dowels-hover = Due fori sull'asse di capovolgimento, nel margine, lavorati prima con { $front }: attraverso la tavola e 8 mm nel piano di sacrificio, per spine da 8 mm
forms-setup-back-side = Retro
forms-setup-back-side-hover = Il piazzamento che è questa tavola capovolta, per tagliare l'altra faccia
forms-setup-back-none = Nessuno
forms-setup-add-back = Aggiungi il retro
forms-setup-add-back-hover = Un nuovo piazzamento: questa tavola capovolta, con un pezzo proprio. Disegna il retro visto dal fronte, attraverso la tavola: i suoi tagli sono specchiati
forms-setup-dowels-open-hover = Apri la lavorazione che taglia i fori per spina
forms-setup-remove-dowels-hover = Togli i fori per spina: la loro lavorazione e i loro due cerchi
forms-setup-place-dowels = Riposiziona
forms-setup-place-dowels-hover = Sposta i fori per spina dove vanno ora, lontano dai tagli delle due facce; uno zero sulla prima spina si sposta con essa. Solo prima di lavorare il fronte: poi i suoi fori sono fatti
forms-setup-dowels-none = Non ancora
forms-setup-dowels-none-hover = Il retro viene rimesso a posto con spine, e { $front } non ha ancora fori per esse
forms-shared-with-front = { $back } è la tavola di { $front } capovolta: ne condivide il grezzo e lo zero pezzo. Cambiali lì.
forms-open-front = Apri { $front }
forms-thinner-after-surfacing = La sua tavola è spessa { $thickness } dopo la spianatura di { $front }.
forms-setup-make-active = Rendilo il piazzamento attivo, per cambiare pannello e zero
forms-remove-setup = Rimuovi piazzamento
forms-copies-hover = Il pezzo posato di nuovo: ogni istanza lavora le stesse forme e lavorazioni, spostata, ruotata o specchiata. Ne aggiunga una dal menu contestuale del pezzo, o crei una griglia qui sotto. Clicchi un’istanza nella vista per spostarla o ruotarla.
forms-instances-none = Togli tutte
forms-instances-grid = Crea la griglia
forms-grid-columns = colonne
forms-grid-rows = righe
# Instance to instance distance, in X and in Y.
forms-pitch-x = passo X
forms-pitch-y = passo Y
forms-gap = distanza
forms-gap-hover = Crea la griglia con il passo che lascia questa distanza tra i tagli delle vicine.
forms-order = ordine

# The machine. "Post": the post-processor that writes the G-code.
forms-post = Post
forms-post-yours = { $name } (tuo)
forms-post-folder-hover = I tuoi post-processori vanno in { $folder }
forms-post-builtin-hover = I post-processori integrati
forms-gcode-units = Unità G-code
forms-gcode-units-hover = Le unità in cui è scritto il G-code: mm (G21) o pollici (G20), come è impostato il tuo controller. Tutto il resto del progetto resta in mm
forms-gcode-mm = mm (G21)
forms-gcode-inch = pollici (G20)
forms-spindle = Mandrino
forms-spindle-cw = orario (M3)
forms-spindle-ccw = antiorario (M4)
forms-max-feed = Avanzamento max
forms-tool-changes = Cambi utensile
forms-tool-changes-hover = Con più utensili: un file G-code per utensile (cambia l'utensile e imposta lo zero Z tra un file e l'altro), o un file con cambi utensile M6 gestiti dal tuo sender (GRBL stesso rifiuta M6)
forms-tool-change-files = un file per utensile
forms-tool-change-program = un file con M6 (il cambio lo gestisce il tuo sender)
forms-travel = Corse X Y Z
forms-max-feed-not-set = non impostato
forms-travel-not-checked = non verificate
forms-revert-hover = "{ $name }" ha { $value }: clic per prenderlo
forms-post-yours-copied = Tuo, copiato nel progetto
forms-post-changed = Il tuo file del post è cambiato
forms-post-update = Aggiorna
forms-post-update-hover = Copia nel progetto il post com'è ora
forms-post-file-missing = Tuo; il progetto conserva la sua copia (il file non c'è)

# A tool's values.
forms-tool-type = Tipo
forms-diameter = Diametro
forms-spindle-speed = Velocità mandrino
forms-centre-cutting = Taglio al centro
forms-yes = sì
forms-no = no
forms-unknown = sconosciuto
forms-angle = Angolo
forms-angle-hover = L'angolo incluso della V; serve per l'incisione a V
forms-tip = Punta
forms-tip-hover = Il diametro della punta piatta; se non impostato è considerata appuntita
forms-pointed = appuntita
forms-corner-radius = Raggio di spigolo
forms-corner-radius-hover = Il raggio degli spigoli arrotondati (fresa torica): i rilievi sono lavorati con la sua forma reale, le tasche mantengono un raccordo al piede delle pareti. Non impostato: spigoli vivi
forms-square = vivi

# A rest pass of a pocket: a smaller tool clears what the one before left.
forms-rest-pass-of = lavorazione { $number }
forms-rest-pass-of-named = lavorazione { $number } ({ $name })
forms-rest-pass-facts = { $start }–{ $end } { $unit }, maggiorazione { $clearance } { $unit }, { $direction }
forms-rest-pass = Passata di ripresa
forms-rest-of = Ripresa di
forms-rest-of-hover = Lavora le forme e le profondità della sua tasca, e rimuove solo ciò che lascia l'utensile precedente
# A button: show the pocket.
forms-open = Apri
forms-from-pocket = Dalla tasca
forms-after-tool = Dopo l'utensile
forms-after-tool-hover = Rimuove ciò che lascia questo utensile: quello della tasca, o della passata precedente
forms-cut-it = Lavorala
forms-cut-it-hover = Disattivato: conservata, ma non lavorata
forms-rest-pass-note = Eseguita dopo le tasche vicine alla sua che usano lo stesso utensile, tutte le loro passate di ripresa insieme: l'utensile piccolo entra una sola volta.
forms-remove-rest-pass = Rimuovi passata di ripresa
forms-floor-pass = Passata di fondo
forms-floor-of = Fondo di
forms-floor-of-hover = Lavora le forme della sua incisione a V fino alla profondità finale, lontano dalle pareti inclinate
forms-from-vcarve = Dall'incisione a V
forms-floor-pass-facts = { $start }–{ $end } { $unit }, { $inset } { $unit } dentro il contorno
forms-floor-pass-note = Eseguita prima delle incisioni a V vicine che usano la stessa fresa a V, tutte le loro passate di fondo insieme. Dove la fresa piatta non arriva, la fresa a V ripassa lungo la linea mediana.
forms-remove-floor-pass = Rimuovi passata di fondo
forms-floor-hint = Lascia parte del fondo: una da al massimo { $largest } lo svuoterebbe tutto.
forms-floor-hint-library = Nella tua libreria: { $tools }

# Font styles, headings in the font list.
forms-font-plain = Normali
forms-font-script = Corsivi
forms-font-display = Decorativi
forms-explode-dxf = Esplodi…
forms-explode-dxf-hover = Trasforma le forme di questo DXF in forme disegnate modificabili; il legame con il file DXF si perde

## The Statistics section of an operation (docs/specs/operation-statistics.md)
forms-section-statistics = Statistiche
forms-statistics-time = Tempo
forms-statistics-length-time = { $length }, { $time }
forms-statistics-cutting = Taglio
forms-statistics-cutting-hover = Movimenti in avanzamento di lato: rette, archi, rampe ed eliche, misurati in 3D
forms-statistics-plunging = Affondamento
forms-statistics-plunging-hover = Movimenti in avanzamento dritti in giù o in su: affondamenti e foratura
forms-statistics-rapids = Rapidi
forms-statistics-rapids-hover = Movimenti rapidi (G0), compresi in su e in giù
forms-statistics-lifts = Risalite
forms-statistics-lifts-hover = Quante volte l'utensile risale in rapido: tra i percorsi e a ogni scarico di una foratura a scarico
forms-statistics-copies = Per { $copies } istanze, compreso il pezzo.
forms-statistics-generate = Genera (F5) per vedere le statistiche.
forms-statistics-generating = Generazione…
forms-statistics-failed = Non generato: vedi il rapporto.
forms-statistics-off = Disattivata: non lavorata.
forms-statistics-out-of-date = non aggiornato
forms-statistics-not-generated = non generato
forms-statistics-off-summary = disattivata
forms-statistics-operations-hover = Le lavorazioni e le loro passate di ripresa e di fondo contate qui: quelle lavorate
forms-statistics-empty = Nessuna lavorazione.
forms-statistics-empty-summary = nessuna lavorazione
forms-slot-feed = Avanzamento cava
forms-slot-feed-hover = L'avanzamento degli anelli a piena larghezza: il primo anello di ogni profondità e il primo di ogni ramo. Vuoto: l'avanzamento
forms-slot-step-down = Prof. di passata cava
forms-slot-step-down-hover = Gli anelli a piena larghezza scendono a ogni profondità a passi non più profondi di questo, prima che gli altri anelli taglino tutta la passata in una volta
forms-pocket-step-down-hover = La profondità massima di ogni passata; gli anelli al passo laterale la tagliano in una volta
forms-material-speed-differs = I valori di { $name } sono per { $rpm } rpm; l'utensile gira a { $runs }

# Reliefs and their roughing pass (docs/specs/stl-relief.md §9, §14).
forms-relief-depth = Profondità
forms-relief-depth-hover = Quanto scende il rilievo: la profondità del suo modello. Cambiarla qui cambia il modello e ogni rilievo che ne viene
forms-relief-stepover-hover = La distanza tra le linee, come parte del diametro dell'utensile: più piccola è più liscia e più lenta
forms-relief-ridge = Creste
forms-relief-ridge-hover = L'altezza delle creste che la fresa sferica lascia tra le sue linee su un fondo piano. Sotto 0,02 mm il legno non le mostra; un passo più fine richiede solo più tempo
forms-relief-deep = { $depth } di profondità con questo solo utensile: aggiungi una passata di sgrossatura (clic destro sul rilievo) perché la fresa sferica non tolga tutto
forms-section-rim = Bordo
forms-rim-none = nessuno
forms-rim-summary = { $width } intorno: { $shape }, rampa a { $angle }°
forms-rim-hover = Un piano sul fondo del rilievo tutto intorno, poi una rampa fino alla superficie della tavola: il modello non resta in una trincea e ogni linea parte dalla superficie
forms-rim-shape = Intorno a
forms-rim-outline = il contorno
forms-rim-box = un rettangolo
forms-rim-width = Larghezza del piano
forms-rim-width-hover = Fin dove arriva il piano sul fondo dal contorno o dal rettangolo del modello
forms-rim-corner = Raggio degli angoli
forms-rim-ramp = Rampa
forms-rim-ramp-hover = La pendenza dal piano fino alla superficie della tavola, dall'orizzontale: a 45° si estende quanto il rilievo è profondo; 90° è una parete dritta
forms-relief-angle-hover = La direzione delle linee da X. Linee inclinate rispetto ai lati lunghi del modello lasciano meno segni
forms-relief-zigzag-hover = Avanti e indietro lungo le linee, o ogni linea nello stesso verso, sollevando tra l'una e l'altra
forms-roughing-pass = Passata di sgrossatura
forms-roughing-of = Sgrossatura di
forms-roughing-of-hover = Il rilievo che questa passata sgrossa prima della finitura
forms-from-relief = Dal rilievo
forms-roughing-facts = { $start }–{ $end } { $unit }, linee a { $angle }°
forms-roughing-step-down-hover = La distanza tra i livelli: il materiale si toglie un livello alla volta
forms-roughing-allowance = Sovrametallo
forms-roughing-allowance-hover = Ciò che resta sul modello per la finitura, tutto intorno: anche sui fianchi ripidi
forms-roughing-note = Tagliata subito prima del rilievo. Non taglia mai più largo di dove arriva la finitura, e il programma è controllato per lasciare il sovrametallo sul modello.
forms-remove-roughing = Rimuovi passata di sgrossatura
forms-smoothing-pass = Passata di lisciatura
forms-smoothing-of = Lisciatura di
forms-smoothing-of-hover = Il rilievo che questa passata liscia dopo la sgrossatura
forms-smoothing-facts = { $start }–{ $end } { $unit }
forms-smoothing-angle = Linee a
forms-smoothing-angle-hover = La direzione delle linee da X. Di traverso alle linee di sgrossatura, toglie i gradini che lasciano
forms-smoothing-allowance-hover = Ciò che resta sul modello per la finitura, tutto intorno: di solito quello della sgrossatura
forms-smoothing-note = Lavorata dopo la passata di sgrossatura e prima del rilievo, seguendo una volta la superficie. Il programma è controllato per mantenere il sovrametallo sul modello.
forms-remove-smoothing = Rimuovi la passata di lisciatura
forms-add-smoothing = Aggiungi passata di lisciatura
forms-add-smoothing-hover = L'utensile di sgrossatura segue ancora una volta la superficie, con le linee di traverso a queste, e toglie i gradini che lasciano sui pendii
forms-rim-hull = Più pieno
forms-rim-hull-hover = Quanto viene riempito il contorno del bordo: 0 % segue il contorno del modello con tutte le rientranze; di più riempie prima le rientranze strette, poi le più larghe; 100 % gira intorno al modello come un elastico (il suo inviluppo convesso). La rampa mantiene il suo angolo.
forms-picture-white-high = Il bianco è in alto
forms-picture-white-high-hover = Le parti bianche dell'immagine sono la sommità del rilievo e il nero il fondo; togli la spunta per una mappa di profondità al contrario
forms-picture-smoothing = Smussatura
forms-picture-smoothing-hover = Una sfocatura sulle altezze, il suo raggio sulla tavola: toglie il rumore dei pixel e la grana di una foto; 0 lascia l'immagine com'è
forms-picture-background = Sfondo
forms-picture-background-hover = I grigi più scuri fino a questa quota sono sfondo, non il modello: il contorno gira intorno al resto, e il bordo intorno a quello. 0 % prende tutta l'immagine
forms-picture-gamma = Gamma
forms-picture-gamma-hover = Le altezze piegate: sopra 1 il fondo più piatto e le cime più ripide, sotto 1 il contrario; 1 come l'immagine
forms-picture-kind = Tipo
forms-picture-kind-hover = Mappa di profondità: la luminosità dell'immagine è l'altezza. Gonfiata: le sue forme in bianco e nero, ognuna che sale dal proprio bordo, per loghi e scritte
forms-picture-kind-depth = Mappa di profondità
forms-picture-kind-inflate = Gonfiata
forms-picture-threshold = Soglia
forms-picture-threshold-hover = I grigi più scuri di questa sono neri: le forme. Automatica la sceglie dall'immagine
forms-picture-automatic = Automatica
forms-picture-invert = Inverti
forms-picture-invert-hover = Il bianco è la forma, non il nero
forms-picture-profile = Profilo
forms-picture-profile-hover = Come una forma sale dal suo bordo: tondo, un cuscino; dritto, una cresta lungo ogni tratto, come cesellata
forms-picture-profile-round = Tondo
forms-picture-profile-straight = Dritto
forms-picture-reach = Portata
forms-picture-reach-hover = Fin dove dal bordo una forma continua a salire: ciò che è più largo si appiattisce alla piena profondità. Automatica: metà larghezza della forma più larga, nulla si appiattisce. Per le scritte, circa mezzo tratto
forms-kind-flute = Scanalatura
forms-flute-cut-summary = profonda { $depth }, rampe { $start } / { $end }, { $shape }
forms-flute-start-hover = Dove sono le estremità della scanalatura, dal piano superiore: 0 per il piano, di più per una scanalatura sul fondo di una tasca
forms-flute-depth = Profondità
forms-flute-depth-hover = Quanto è profonda la scanalatura al centro, dal piano superiore
forms-flute-width-value = larga { $width } in alto
forms-flute-passes-hover = Spento: tutta la scanalatura in una passata. Acceso: passate uguali non più profonde di questo valore, ognuna con le stesse rampe
forms-flute-ramp-start = Rampa iniziale
forms-flute-ramp-start-hover = Dopo quanto, lungo la forma dalla prima estremità (come disegnata), la scanalatura raggiunge la profondità. 0: un'estremità dritta
forms-flute-ramp-end = Rampa finale
forms-flute-ramp-end-hover = Quanto prima dell'ultima estremità la scanalatura comincia a risalire. 0: un'estremità dritta
forms-flute-ramp = Rampa
forms-flute-ramp-hover = Dritta: una pendenza uniforme. Curva: più ripida in superficie, si raccorda al fondo senza una linea visibile alla fine della rampa
forms-ramp-straight = dritte
forms-ramp-curved = curve
forms-kind-pattern = Motivo
forms-section-pattern = Motivo
forms-pattern = Motivo
forms-pattern-summary = { $kind }, distanti { $spacing }
forms-pattern-lines = linee
forms-pattern-cross = tratteggio incrociato
forms-pattern-rings = anelli
forms-pattern-waves = onde
forms-pattern-spacing = Spaziatura
forms-pattern-spacing-hover = Dal centro di una scanalatura al successivo
forms-pattern-angle = Angolo
forms-pattern-angle-hover = La direzione delle linee dall'asse X, in senso antiorario
forms-pattern-between = Angolo tra
forms-pattern-between-hover = L’angolo tra le due serie di linee: 90° dà quadrati, 60° rombi
forms-pattern-wavelength = Lunghezza d’onda
forms-pattern-height = Altezza
forms-pattern-height-hover = L’altezza dell’onda, dalla cima al fondo
forms-pattern-inside = Dentro il bordo
forms-pattern-inside-hover = Acceso: le scanalature si fermano a mezza larghezza dalla linea, fuori dai fori. Spento: arrivano fino alla linea
forms-pattern-cut-summary = profonde { $depth }
forms-pattern-width-value = ogni scanalatura larga { $width } in alto
forms-pattern-ends = Estremità
forms-pattern-ends-hover = 0: estremità dritte. Di più: ogni scanalatura entra ed esce in rampa su questa lunghezza, a punta con una fresa a V
forms-kind-prism = Prisma
forms-prism-cut-summary = bordi al massimo a { $depth }, { $direction }
forms-prism-start-hover = Dove si trova la superficie, dal piano superiore: 0 per il piano, di più per un prisma sul fondo di una tasca
forms-prism-max-depth = Profondità massima
forms-prism-max-depth-hover = Spento: i bordi scendono quanto richiedono le forme e il cono della fresa, lo spigolo in superficie. Acceso: non più in fondo di così, la cima piatta: forme smussate
forms-prism-flat-top = Cima piatta
forms-prism-deepest-value = bordi al massimo a { $depth } con la fresa a { $angle }°
forms-pattern-dashes = trattini
forms-pattern-herringbone = spina di pesce
forms-pattern-scales = squame
forms-pattern-sunburst = raggiera
forms-pattern-length = Lunghezza del tratto
forms-pattern-gap = Spazio
forms-pattern-radius = Raggio
forms-pattern-pitch = Passo
forms-pattern-pitch-hover = La distanza tra i tratti attorno a ogni anello
forms-pattern-whole = Solo tratti interi
forms-pattern-whole-hover = Acceso: un tratto che il bordo o un foro taglierebbero viene omesso, così ogni tratto mantiene le due estremità a punta. Spento: i tratti vengono tagliati al bordo
forms-kind-moulding = Modanatura
forms-moulding-cove = guscio
forms-moulding-roundover = arrotondamento
forms-moulding-ogee = gola
forms-moulding-bead = tondino
forms-moulding-flute = scanalatura
forms-moulding-shape = forma disegnata
forms-moulding-left = a sinistra del percorso
forms-moulding-right = a destra del percorso
forms-moulding-summary = { $profile }, { $side }
forms-section-profile = Profilo
forms-moulding-profile = Profilo
forms-moulding-width = Larghezza
forms-moulding-shape-id = Forma
forms-moulding-shape-hover = La forma aperta disegnata letta come sezione: verso destra in largo, verso l’alto; disegnala come geometria di costruzione
forms-moulding-side-hover = Dove sta il profilo: fuori o dentro un percorso chiuso, a sinistra o a destra di uno aperto come disegnato
forms-moulding-square = dritte
forms-moulding-round = tonde
forms-moulding-taper = in rampa
forms-moulding-ends = Estremità
forms-moulding-ends-hover = Come finisce la modanatura su un percorso aperto: dritta fino in fondo, girata attorno al punto finale come una fresa fermata, o sfumando nella tavola
forms-moulding-taper-length = Lunghezza della rampa
forms-moulding-cut-summary = passo { $stepover }, { $direction }
forms-moulding-stepover = Passo
forms-moulding-stepover-hover = La distanza tra le passate attraverso il profilo: più piccola, più liscia
forms-moulding-preview-hover = Il profilo in sezione, in scala: la tavola con il profilo tagliato, il percorso al segno
forms-section-pieces = Tasselli
forms-pieces-on = Lavora a tasselli
forms-pieces-on-hover = Una forma più grande delle sue tavole, lavorata come un puzzle: le giunte la dividono in tasselli che stanno ciascuno su una tavola, uniti da denti o dita. Ogni tassello è un programma a sé.
forms-pieces-off = una tavola
forms-pieces-board = Tavole
forms-pieces-board-hover = Le tavole che ha: ogni tassello, con i suoi incastri e il margine, sta su una.
forms-pieces-grid = Griglia
forms-pieces-grid-auto = Il meno possibile
forms-pieces-joint = Incastro
forms-pieces-knobs = Denti a puzzle
forms-pieces-fingers = Dita
forms-pieces-size = Misura dell’incastro
forms-pieces-size-hover = La testa di un dente da parte a parte, o la lunghezza di un dito. La curva più stretta deve adattarsi all’utensile di contornatura: per i denti, il raggio dell’utensile più il gioco è al massimo un quinto della misura.
forms-pieces-spacing = Passo
forms-pieces-spacing-hover = Da dente a dente, o un dito e uno spazio, lungo una giunta: almeno due misure d’incastro.
forms-pieces-clearance = Gioco
forms-pieces-clearance-hover = Lo spazio tra due tasselli a una giunta, per la colla. Ogni tassello ne dà metà.
forms-pieces-shift-x = Giunta { $seam } lungo X
forms-pieces-shift-y = Giunta { $seam } lungo Y
forms-pieces-shift-hover = Sposti la giunta dal suo posto regolare, per tenerla lontana da una lettera o da un punto stretto.
