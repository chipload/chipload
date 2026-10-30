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
browser-holes = Fori
browser-holes-hover = I cerchi selezionati: forati con una punta, o con foratura elicoidale con una fresa
browser-surface = Spianatura
browser-surface-hover = Tutto il grezzo abbassato in piano con una fresa larga: spiana il piano di sacrificio o il grezzo. Non serve selezione

# Job settings and Tools.
browser-job-settings = Macchina
browser-job-settings-hover = La macchina: il suo post-processore, le unità del G-code, la Z di sicurezza, le corse e i cambi utensile, comuni a tutti i piazzamenti
browser-work-zero = Zero pezzo
browser-work-zero-hover = Dove si trova X0 Y0 Z0 del G-code. Fai clic per impostarlo, oppure selezionalo e trascinalo nella vista.
browser-job-settings-problems-hover = Macchina, post-processore, unità e corse. { $count ->
        [one] { $count } problema
       *[other] { $count } problemi
    }
browser-tools = Utensili
browser-tools-hover = Gli utensili del progetto: fai clic per vederli tutti e aggiungerne uno

# Parts: groups of operations, each cut with its copies before the next.
browser-setups = Piazzamenti
browser-setups-hover = I pannelli da cui si ricava il progetto, uno dopo l'altro: ognuno un programma a sé, con il suo grezzo e il suo zero pezzo. Un clic lo rende attivo
browser-add-setup = Aggiungi un piazzamento
browser-add-setup-hover = Un altro pannello: il suo grezzo, il suo zero e i suoi pezzi, lavorato come programma a sé
browser-setup-parts = { $count ->
    [one] { $count } pezzo
   *[other] { $count } pezzi
}
browser-setup-active-hover = Il piazzamento attivo: mostrato a colori, simulato e calcolato. Un clic mostra il suo pannello; il suo zero è sotto
browser-setup-hover = Un clic lo rende il piazzamento attivo: mostrato, simulato e calcolato
browser-move-to-setup = Sposta nel piazzamento
browser-new-part = Nuovo pezzo
# At a part row's right end: the part new operations go into.
browser-part-active = attivo
browser-part-active-hover = Il pezzo attivo: qui vanno le nuove lavorazioni. Clic per le sue copie; clic destro per altro
browser-part-hover = Clic per le sue copie; doppio clic per renderlo il pezzo attivo; clic destro per altro
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
