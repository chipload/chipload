# The main window: opening, importing, saving and exporting, parts, rest passes, the
# selection (crates/chipload-gui/src/app/mod.rs).

# The heading of the right-hand panel.
app-properties = Proprietà

app-start-hint = Importa un DXF o SVG (Ctrl+I) per iniziare un progetto, o aprine uno (Ctrl+O).

app-cannot-open = Impossibile aprire { $file }:
app-project = Progetto: { $file }
app-old-schema = Questo file è della versione di schema { $version }; il salvataggio scrive la versione { $current }.

app-new-project = Nuovo progetto.
# $what: a machine or tool from the user's library, e.g. machine "Router".
app-started-with = Iniziato con { $what }.
app-new-project-next = Disegna forme dal menu Disegna, o porta un disegno con Aggiungi DXF o SVG. Controlla il pannello nel piazzamento, la macchina in Macchina e l'utensile in Utensili.
app-units-missing = { $file } non indica le unità che usa; scegli mm o pollici.
app-px-missing = { $file } indica la sua dimensione solo in pixel; scegli i suoi pixel per pollice.
app-cannot-import = Impossibile importare { $file }:
app-new-project-from = Nuovo progetto da { $file } ({ $shapes } forme).
app-import-next = Seleziona i contorni, poi Nuova contornatura o Nuova tasca. Controlla il pannello nel piazzamento, la macchina in Macchina e l'utensile in Utensili.

# Followed by the list of problems.
app-project-incomplete = Il progetto non è completo:
app-lost-geometry = Alcune lavorazioni hanno perso la geometria: sceglila di nuovo
app-could-not-update-drawing = Impossibile aggiornare il disegno:
app-could-not-add = Impossibile aggiungere { $file }:
app-could-not-move-drawing = Impossibile spostare il disegno:

# The distance between a grid's cells; $dy and $gap carry their unit.
app-pitch-set = Passo { $dx } × { $dy } (distanza { $gap })
app-pitch-needs-plan = Impossibile impostare il passo da una distanza: prima va pianificato il pezzo.
app-instance-needs-plan = Impossibile posizionare la nuova istanza: prima va pianificato il pezzo.

app-deleted-part = { $count ->
    [0] Eliminato il pezzo { $name } (Ctrl+Z lo ripristina)
    [one] Eliminati il pezzo { $name } e la sua lavorazione (Ctrl+Z li ripristina)
   *[other] Eliminati il pezzo { $name } e le sue { $count } lavorazioni (Ctrl+Z li ripristina)
}
app-part-needed = Un progetto richiede almeno un pezzo
app-board-surface-stays = La spianatura di tutta la tavola appartiene al piazzamento e vi resta per prima

# A rest pass clears what a larger tool left in a pocket.
app-could-not-add-rest-pass = Impossibile aggiungere una passata di ripresa: { $error }
app-could-not-add-floor-pass = Impossibile aggiungere una passata di fondo: { $error }
app-setup-refused = Impossibile modificare i piazzamenti: { $error }
app-dowels-placed = Fori per spina riposizionati, lontano dai tagli delle due facce. Lo zero si è spostato con la prima spina se era lì.
app-setup-has-operations = Il piazzamento ha pezzi con lavorazioni: spostali prima in un altro piazzamento, o eliminali.
app-exported-setup = Solo il piazzamento «{ $name }»: rendi attivo un altro piazzamento ed esporta anche quello.
app-floor-no-endmill = Nessuna fresa piatta nel progetto o nella tua libreria utensili
app-rest-with-corners = Questa tasca ha angoli dogbone o T-bone, e le passate di ripresa non si possono ancora combinare con essi. Gli angoli arrivano già dove arriverebbe una passata di ripresa.
# $tool: a tool number such as T3.
app-rest-tool-missing = L'utensile { $tool } non è nel progetto: assegna prima un utensile alla tasca.
app-rest-no-smaller = Nessuna fresa più piccola di { $tool } ({ $diameter }) nel progetto o nella tua libreria utensili. Aggiungine una in Utensili, poi fai di nuovo clic destro sulla tasca.
app-now-rest-pass = La lavorazione { $op } è ora una passata di ripresa della lavorazione { $parent }; viene eseguita dopo la serie di tasche a cui appartiene

app-generate-before-export = Calcola (F5) prima di esportare il G-code.
app-planning-then-export = Pianificazione di ogni pezzo, poi esportazione
app-planning-part-then-export = Pianificazione del pezzo { $name }, poi esportazione
app-could-not-export = Impossibile esportare { $path }:
app-dxf-exported = Scritto { $path }.
app-could-not-export-dxf = Il disegno non è stato esportato:
app-sheet-failed = G-code esportato, ma impossibile scrivere la scheda di attrezzaggio { $path }:
app-exported-n-files = Esportati { $count } file: tra uno e l'altro cambia utensile e imposta lo zero Z, mantenendo lo zero X e Y.
app-sheet-for-operator = Scheda di attrezzaggio per l'operatore: { $path }
app-exported-files = Esportati { $count } file accanto a { $path }
app-exported-files-sheet = Esportati { $count } file e la scheda di attrezzaggio { $sheet } accanto a { $path }
app-exported = Esportato { $path }
app-export-all-running = Calcolo e scrittura di ogni piazzamento…
app-exported-all = { $setups } piazzamenti esportati, { $count } file, accanto a { $path }
app-export-all-failed = Non è stato possibile esportare i piazzamenti; non è stato scritto nulla
app-export-all-stopped = L’esportazione di tutti i piazzamenti si è fermata inaspettatamente; forse non è stato scritto nulla
app-exported-sheet = Esportati { $path } e la scheda di attrezzaggio { $sheet }

app-cannot-save-yet = Il progetto non può ancora essere salvato:
app-could-not-save = Impossibile salvare { $path }:
app-saved = Salvato { $path }

# What is selected on the canvas.
app-nothing-selected = Nessuna selezione.
# $count contours selected; $what says how (one of the texts below); $list their numbers.
app-selected = { $count } selezionati ({ $what }): { $list }
app-selection-layer = layer { $layer }
app-selection-dxf = disegno { $id }
app-selection-image = immagine { $id }
app-selection-model = modello { $id }
app-selection-text = testo { $id }
# $source: one of the three texts above.
app-selection-left-out = { $source }, { $count } esclusi
app-selection-contours = contorni
# Open contours (not closed) are not cut.
app-open-not-cut = Aperti (non lavorati): { $list }

# File menu, for a project started without a drawing.
app-import-dxf-into-project = Importa DXF o SVG nel progetto…
app-import-dxf-into-project-path = Importa DXF o SVG nel progetto da percorso…
# File menu, a new version of the project's drawing.
app-update-drawing-from-dxf = Aggiorna disegno da DXF o SVG…
app-update-drawing-from-path = Aggiorna disegno da percorso…

# The Nest window (docs/specs/nesting.md §7)
nest-menu = Annida…
nest-menu-hover = Annida…: ogni istanza dei pezzi del piazzamento posta sulla sua tavola il più vicino possibile, separata dalla distanza; prima un’anteprima, in una finestra
nest-again = Annida di nuovo
nest-again-hover = Annida di nuovo il piazzamento con distanza, margine e rotazioni dell’ultima volta, senza finestra
nest-title = Annida: { $setup }
nest-title-one = Annida
nest-board-follows = La tavola segue il disegno: annidare le dà questa misura, con l’angolo nell’origine del disegno.
nest-board = Tavola
nest-gap = Distanza
nest-gap-hover = La minima distanza tra ciò che tagliano due istanze: una striscia di tavola che tiene rigido il pannello e salde le linguette
nest-margin = Margine
nest-margin-hover = Lo spazio lasciato al bordo della tavola, per viti o morsetti
nest-goal = Obiettivo
nest-goal-hover = Ciò che l’annidamento rende il più piccolo possibile, dopo aver posizionato tutto ciò che può
nest-goal-compact = Compatto
nest-goal-compact-hover = Il riquadro più piccolo attorno ai pezzi: un blocco nell’angolo in basso a sinistra, il resto della tavola una L attorno
nest-goal-low = Lascia libero in alto
nest-goal-low-hover = I pezzi il più in basso possibile, su tutta la larghezza: resta libera una striscia in alto
nest-goal-left = Lascia libera la destra
nest-goal-left-hover = I pezzi il più a sinistra possibile, su tutta l’altezza: resta libera una striscia a destra
nest-part = Pezzo
nest-count = Volte
nest-count-hover = Quante volte si taglia il pezzo: prima si aggiungono o tolgono istanze, mai il pezzo stesso
nest-turns = Rotazioni
nest-turns-none = Com’è
nest-turns-half = Mezzi giri
nest-turns-quarter = Quarti di giro
nest-turns-every-15 = Ogni 15°
nest-running = Annidamento…
nest-stale = Impostazioni cambiate: Anteprima per vedere l’annidamento
nest-apply = Applica
nest-preview = Anteprima
nest-stopped-unexpectedly = L’annidamento si è fermato inaspettatamente
