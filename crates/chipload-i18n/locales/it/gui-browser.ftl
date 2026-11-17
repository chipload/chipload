# The Browser: the project as one tree of folders and rows (crates/chipload-gui/src/app/browser.rs).

# The panel's caption, shown in capitals.
browser-caption = Browser
browser-no-project = Nessun progetto aperto.
browser-new-project = Nuovo progetto

# A row's eye (shown on the canvas or not) and tick (cut or not).
browser-eye-hide-hover = Nascondilo nell'area di disegno; viene comunque lavorato
browser-eye-show-hover = Nascosto nell'area di disegno (comunque lavorato): mostralo
browser-tick-on-hover = Lavorato: togli la spunta per escluderlo dal programma
browser-tick-off-hover = Non lavorato: metti la spunta per lavorarlo

# A project tool's link to the user's tool library, at its row's right end ($name: the
# library tool's name).
browser-link-changed = { $name } (modificato)
browser-link-missing = (non in libreria)

# The kinds of operation, and what a new one of each does.
browser-profile = Contornatura
browser-profile-hover = Una lavorazione di contornatura per i contorni selezionati
browser-pocket = Tasca
browser-pocket-hover = Una lavorazione di tasca per i contorni selezionati
browser-vcarve = Incisione a V
browser-vcarve-hover = Una fresa a V lungo la linea mediana dei contorni chiusi selezionati, alla profondità che rende il taglio largo quanto la forma
browser-follow-path = Segui percorso
browser-follow-path-hover = L'utensile sulla linea delle forme selezionate, aperte o chiuse: incisioni, scanalature e linee di tracciatura
browser-chamfer = Smusso
browser-chamfer-hover = Lo spigolo superiore delle forme chiuse scelte smussato con una fresa a V, largo quanto imposti
browser-holes = Fori
browser-holes-hover = I cerchi selezionati: forati con una punta, o con foratura elicoidale con una fresa
browser-surface = Spianatura
browser-relief = Rilievo
browser-relief-hover = Un modello 3D scolpito dall'alto in linee parallele, ogni punto basso quanto l'utensile può scendere senza intaccare il modello. Dentro il contorno del modello selezionato, o del primo
browser-relief-needs-model = Aggiungi prima un modello 3D (File > Aggiungi modello 3D…)
browser-surface-hover = Tutto il grezzo abbassato in piano con una fresa larga: spiana il piano di sacrificio o il grezzo. Non serve selezione

# Job settings and Tools.
browser-job-settings = Macchina
browser-job-settings-hover = La macchina: il suo post-processore, le unità del G-code, la Z di sicurezza, le corse e i cambi utensile, comuni a tutti i piazzamenti
browser-stock = Grezzo
browser-stock-hover = La tavola che questo piazzamento lavora: spessore, dimensioni e materiale. Fai clic per impostarli.
browser-work-zero = Zero pezzo
browser-work-zero-hover = Dove si trova X0 Y0 Z0 del G-code. Fai clic per impostarlo, oppure selezionalo e trascinalo nella vista.
browser-job-settings-problems-hover = Macchina, post-processore, unità e corse. { $count ->
        [one] { $count } problema
       *[other] { $count } problemi
    }
browser-tools = Utensili
browser-tools-hover = Gli utensili del progetto: fai clic per vederli tutti e aggiungerne uno

# Parts: groups of operations, each cut with its instances before the next.
browser-setups = Piazzamenti
browser-setups-hover = I pannelli da cui si ricava il progetto, uno dopo l'altro: ognuno un programma a sé, con il suo grezzo e il suo zero pezzo. Un clic lo rende attivo
browser-add-setup = Aggiungi un piazzamento
browser-add-setup-hover = Un altro pannello: il suo grezzo, il suo zero e i suoi pezzi, lavorato come programma a sé
browser-add-back-side = Aggiungi il retro
browser-board-settings = Tasselli e retro…
browser-board-settings-hover = Le impostazioni del grezzo: tagliarlo in tasselli come un puzzle o dargli un retro capovolto
browser-add-back-side-hover = Un nuovo piazzamento: questa tavola capovolta, con un pezzo proprio. Disegna il retro visto dal fronte, attraverso la tavola: i suoi tagli sono specchiati
browser-setup-no-dowels-hover = Rimesso a posto con spine, e il fronte non ha ancora fori per spina: aggiungili nel suo modulo
browser-setup-parts = { $count ->
    [one] { $count } pezzo
   *[other] { $count } pezzi
}
browser-setup-active-hover = Il piazzamento attivo: mostrato a colori, simulato e calcolato. Un clic mostra il suo pannello; il suo zero è sotto
browser-setup-hover = Un clic lo rende il piazzamento attivo: mostrato, simulato e calcolato
browser-move-to-setup = Sposta nel piazzamento
browser-place-on-setup = Posiziona nel piazzamento
browser-place-on-setup-hover = Una nuova istanza sul grezzo di questo piazzamento, dove il pezzo è disegnato
browser-part-not-placed = non posizionato
browser-instance-of = { $part } #{ $number }
browser-parts = Pezzi
browser-parts-hover = Ogni pezzo con le sue lavorazioni. Le sue istanze sono sotto i piazzamenti in cui si tagliano: trascina un pezzo su un piazzamento per posizionarlo lì
browser-export-setup = Esporta il G-code di questo piazzamento…
browser-export-setup-hover = Il suo programma e il foglio di attrezzaggio, chiamati -S1, -S2 se ce ne sono più; diventa il piazzamento attivo e viene calcolato prima se non è mostrato
browser-new-part = Nuovo pezzo
# At a part row's right end: the part new operations go into.
browser-part-active = attivo
browser-part-active-hover = Il pezzo attivo: qui vanno le nuove lavorazioni. Clic per le sue proprietà e istanze; clic destro per altro
browser-part-mirrored = specchiato
browser-add-instance = Aggiungi istanza
browser-add-instance-hover = Posiziona di nuovo il pezzo accanto a ciò che taglia, collegato: modifichi il pezzo e l’istanza lo segue. La trascini per spostarla, la sua maniglia tonda per ruotarla.
browser-add-mirrored-instance = Aggiungi istanza specchiata
browser-add-mirrored-instance-hover = Lo stesso, specchiato da sinistra a destra: l’altro lato di una scatola. Sempre tagliato in concordanza.
browser-instance-hover = Il pezzo posizionato di nuovo: le stesse forme e lavorazioni, spostate, ruotate o specchiate. Clicchi per modificarla, o clicchi la sua ombra nella vista.
browser-instance-mirror = Specchia
browser-instance-turn = Ruota di 90°
browser-instance-remove = Rimuovi
browser-face-board = Spiana la tavola…
browser-face-board-hover = Una spianatura di tutta la tavola, prima in questo piazzamento, tagliata una volta qualunque siano i pezzi e le istanze: spianare il piano di sacrificio o il grezzo. L'area di un pezzo è una spianatura del suo contorno selezionato
browser-surface-needs-shapes = Seleziona prima il contorno del pezzo. Tutta la tavola è del piazzamento: clic destro su di esso, Spiana la tavola…
app-face-board-failed = Impossibile spianare la tavola
browser-place-part = Posiziona…
browser-place-part-hover = Seleziona dove si taglia il pezzo: un fantasma da spostare e ruotare (trascinalo, la sua maniglia tonda, o Q per la bussola); le sue forme restano dove sono disegnate
browser-reset-placement = Ripristina collocazione
browser-reset-placement-hover = Taglia di nuovo il pezzo dove sono disegnate le sue forme
browser-part-hover = Clic per le sue proprietà e istanze; doppio clic per renderlo il pezzo attivo; clic destro per altro
# A verb: make this the active part.
browser-activate = Attiva
browser-activate-hover = Le nuove lavorazioni vanno nel pezzo attivo
browser-export-part = Esporta il G-code di questo pezzo…
browser-export-part-hover = Scrivere il G-code solo di questo pezzo, come se tutti gli altri pezzi fossero disattivati; anche la sua scheda di attrezzaggio se quell'impostazione è attiva.
browser-new-operation = Nuova lavorazione
browser-select-shapes-first = Seleziona prima le forme
browser-move-up = Sposta su
browser-move-down = Sposta giù
# $count: the part's operations.
browser-delete-part = { $count ->
        [0] Elimina pezzo
        [one] Elimina pezzo e la sua lavorazione
       *[other] Elimina pezzo e le sue { $count } lavorazioni
    }
browser-delete-part-disabled-hover = Un progetto richiede almeno un pezzo
browser-part-empty-active = Seleziona forme, poi aggiungi una lavorazione
browser-part-empty = Nessuna lavorazione: trascinane una qui

# An operation's row and its right-click menu.
browser-no-name = (senza nome)
# $count: contours the operation cuts that are hidden on the canvas.
browser-op-hidden = { $count } nascosti
browser-duplicate = Duplica
browser-add-rest-pass = Aggiungi passata di ripresa
# A tool for a rest pass: $tool the project tool's number, $size its diameter.
browser-rest-project-tool = T{ $tool } · fresa { $size }
browser-rest-library-tool = { $name } · fresa { $size }
browser-from-library = Dalla tua libreria utensili
browser-copied-into-project-hover = Copiato negli Utensili del progetto
browser-add-rest-pass-hover = Un utensile più piccolo che rimuove solo ciò che lascia l'utensile di questa tasca, nelle sue forme e profondità. Segue la tasca, e viene eseguito dopo le tasche vicine con lo stesso utensile
browser-make-rest-pass = Rendila passata di ripresa della lavorazione { $number }
browser-make-rest-pass-hover = Copia quella tasca: come sua passata di ripresa la segue, e viene eseguita dopo le tasche vicine con lo stesso utensile
browser-delete = Elimina

# A rest pass's row. "Rest": a rest pass, a smaller tool after the pocket's own.
browser-rest = Ripresa
browser-rest-pass-hover = Una passata di ripresa: rimuove ciò che lascia l'utensile precedente, nelle forme e profondità della sua tasca, e viene eseguita dopo le tasche vicine alla sua con lo stesso utensile
browser-add-floor-pass = Aggiungi passata di fondo
browser-add-floor-pass-hover = Una fresa piatta che svuota il fondo tra le pareti dei tratti troppo larghi, nelle forme e profondità di questa incisione a V. Viene eseguita prima dell'incisione a V
browser-floor = Fondo
browser-floor-pass-hover = Una passata di fondo: una fresa piatta svuota il fondo tra le pareti della sua incisione a V, prima di questa; la fresa a V ripassa dove non arriva
browser-remove = Rimuovi

# An operation row's hover text: its lines.
browser-op-rest-after = ripresa dopo T{ $tool }
browser-op-not-cut = Non lavorata (spunta tolta)
browser-op-hidden-hover = Lavora { $count } { $count ->
        [one] contorno nascosto
       *[other] contorni nascosti
    }: mostrali in Disegno per vederli
browser-op-drag-hover = Trascina per riordinare; clic destro per altro

# Properties: what is selected on the canvas.
browser-selection = Selezione
browser-copy-as-selector = Copia come selettore
browser-copy-as-selector-hover = La selezione come JSON, per una lavorazione in un file di progetto

# A thin line between operation rows where the tool changes; $tool the new tool's number.
browser-tool-change = cambio utensile → T{ $tool }

# Reliefs and their roughing pass (docs/specs/stl-relief.md §9, §14).
browser-add-roughing = Aggiungi passata di sgrossatura
browser-add-roughing-hover = Una fresa piatta o sferica semplice che prima toglie il materiale a livelli, lasciando sovrametallo per il rilievo
browser-add-smoothing-needs-roughing = La passata di lisciatura segue una passata di sgrossatura: aggiungi prima una passata di sgrossatura
browser-roughing = Sgrossatura
browser-roughing-hover = Una passata di sgrossatura: il materiale tolto a livelli prima del rilievo, con sovrametallo sul modello
browser-smoothing = Lisciatura
browser-smoothing-hover = Una passata di lisciatura: dopo la sgrossatura, la superficie seguita una volta, toglie i gradini che la sgrossatura lascia sui pendii
browser-flute = Scanalatura
browser-flute-hover = Una scanalatura lungo le forme aperte selezionate, che entra ed esce in rampa alle estremità
browser-pattern = Motivo
browser-pattern-hover = Scanalature a linee, tratteggio incrociato, anelli o onde incise dentro le forme chiuse selezionate
browser-prism = Prisma
browser-prism-hover = Le forme chiuse selezionate restano in piedi come tetti, con uno spigolo vivo al centro, tagliate con una fresa a V
browser-moulding = Modanatura
browser-moulding-hover = Un profilo (guscio, gola, tondino…) fatto scorrere lungo i percorsi selezionati
