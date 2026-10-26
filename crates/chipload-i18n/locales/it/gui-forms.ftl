# The property forms: an operation, the job settings, tools, DXFs, pictures, texts, parts (crates/chipload-gui/src/app/forms.rs).

# Depths, shared by the operations. "Through": a button that sets the end depth through the stock.
forms-start-depth = Prof. iniziale
forms-start-depth-hover = Dove inizia il taglio, in giù dalla superficie del grezzo
forms-end-depth = Prof. finale
forms-end-depth-hover = Dove finisce il taglio, in giù dalla superficie del grezzo
forms-through = Passante
forms-through-hover = Taglia attraverso il grezzo: { $depth }, il suo spessore più la profondità consentita sotto di esso (Impostazioni lavoro > Grezzo)
forms-oversize = Maggiorazione
forms-oversize-disabled = Una contornatura sulla linea non ha un lato su cui tagliare di più
forms-step-down = Prof. di passata
# Indented under "Passes": the depth of each pass.
forms-step-down-sub = prof. di passata

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
forms-kind-holes = Fori
forms-kind-surface = Spianatura
forms-operation-number = Lavorazione { $number }

forms-name = Nome
forms-geometry = Geometria
forms-whole-stock = Tutto il grezzo
forms-whole-stock-hover = Una spianatura lavora tutto il grezzo, una volta, qualunque siano le copie
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
forms-surface-cut-summary = { $start }–{ $end } { $unit }, passate di { $step } { $unit }, passo laterale { $stepover } × D
forms-vcarve-cut-one-pass = da { $start }, al massimo { $end } di profondità, una passata
forms-vcarve-cut-steps = da { $start }, al massimo { $end } di profondità, passate di { $step }

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
forms-passes = Passate
forms-in-steps = a passate

# The operation's tool. "T3" is tool number 3.
forms-tool = Utensile
forms-tool-not-in-project = T{ $tool } (non nel progetto)
forms-from-library = Dalla tua libreria:
forms-use-cutting-values = Usa i valori di taglio dell'utensile
forms-use-cutting-values-hover = Dalla libreria: avanzamento { $feed } / affondamento { $plunge } { $feed_unit }, prof. di passata { $step_down }, passo laterale { $stepover }

forms-section-feeds = Avanzamenti
forms-feed = Avanzamento
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
forms-tab-width = larghezza
forms-tab-height = altezza
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

# An imported DXF.
forms-file = File
forms-units = Unità
forms-dxf-inch = pollici (convertiti in mm)
forms-shapes = Forme
forms-section-placement = Posizione, rotazione e scala
forms-update-dxf = Aggiorna da un DXF…
forms-update-dxf-hover = Una nuova versione del disegno: le lavorazioni mantengono i loro layer
# A button (verb).
forms-remove = Rimuovi
forms-remove-dxf-hover = Rimuovi questo DXF e le sue forme

# A picture traced into shapes.
forms-picture = Immagine
forms-picture-pixels = { $width } × { $height } pixel
forms-traced-as = Vettorizzata come
forms-traced-outlines = contorni
forms-traced-centre-lines = linee mediane
forms-position = Posizione
forms-picture-reset-position = L'angolo del riquadro in X0 Y0
forms-rotation = Rotazione
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
forms-job-settings = Impostazioni lavoro
forms-section-stock = Grezzo
forms-stock-size = Dimensione X × Y
forms-thickness = Spessore
forms-max-overcut = Prof. max sotto grezzo
forms-max-overcut-hover = Quanto un taglio può scendere sotto il grezzo (nel piano di sacrificio). I tagli entro questo limite danno un avviso; quelli più profondi vengono rifiutati.
forms-stock-position = Posizione grezzo
forms-stock-corner-at-origin = angolo nell'origine del disegno
forms-stock-centred-on-origin = centrato sull'origine del disegno
forms-z-zero = Zero Z
forms-z-zero-top = superficie del grezzo
forms-z-zero-bottom = fondo del grezzo
forms-machine-summary = { $post }, Z sicura { $safe_z }, { $spindle }
forms-section-machine = Macchina
forms-from-machine = Dalla macchina
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
forms-dxf-reset-position = Torna dove lo pone il DXF (rotazione e scala restano)
forms-bottom-left = In basso a sinistra
forms-bottom-left-hover = L'angolo del disegno a { $margin } dall'angolo in basso a sinistra del grezzo
# A button: centre the drawing on the stock.
forms-centre = Centra
forms-centre-hover = Il centro del disegno sul centro del grezzo
forms-dxf-rotation-hover = Gradi in senso antiorario rispetto al DXF come disegnato, qualsiasi angolo, attorno al centro del disegno
forms-dxf-reset-rotation = Torna all'angolo originale del DXF (0°)
forms-scale = Scala
forms-scale-hover = Rispetto al DXF come disegnato, attorno al centro del disegno. Dimensioni degli utensili, profondità e ponticelli non vengono scalati.
forms-dxf-reset-scale = Torna alla dimensione originale del DXF (× 1)

# A part: a group of operations cut one after the other.
forms-part-number = Pezzo { $number }
# Label of the on/off checkbox (verb): whether the part is cut.
forms-part-cut = Lavora
forms-part-cut-hover = Disattivato: il pezzo e le sue lavorazioni restano nel progetto, non lavorati
forms-part-cut-check = questo pezzo e le sue lavorazioni
forms-active = Attivo
forms-active-hover = Le nuove lavorazioni vanno nel pezzo attivo. Clic destro su un pezzo per attivarlo
forms-active-yes = sì: le nuove lavorazioni vanno qui
forms-activate = Attiva
forms-operations = Lavorazioni
forms-operations-none = ancora nessuna
forms-copies-one = una
forms-copies = Copie
forms-parts-note = I pezzi vengono lavorati uno dopo l'altro, ciascuno con le sue copie.
forms-copies-hover = Lavora il pezzo più volte in una griglia. Le copie sono disegnate sbiadite; solo il disegno stesso si può selezionare.
forms-copies-grid = più, in griglia
forms-columns-rows = colonne × righe
# Copy to copy distance, in X and Y.
forms-pitch = passo X Y
forms-gap = distanza
forms-gap-hover = Imposta il passo una volta in modo che resti questa distanza tra i tagli di copie vicine. Il progetto conserva il passo.
forms-set-pitch = Imposta passo
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

# Font styles, headings in the font list.
forms-font-plain = Normali
forms-font-script = Corsivi
forms-font-display = Decorativi
forms-explode-dxf = Esplodi…
forms-explode-dxf-hover = Trasforma le forme di questo DXF in forme disegnate modificabili; il legame con il file DXF si perde
