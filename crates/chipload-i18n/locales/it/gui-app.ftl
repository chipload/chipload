# The main window: opening, importing, saving and exporting, parts, rest passes, the
# selection (crates/chipload-gui/src/app/mod.rs).

# The buttons of a folded side panel; P is the key that folds and unfolds both panels.
app-show-browser-hover = Mostra il Browser (P: entrambi i pannelli)
app-show-properties-hover = Mostra Proprietà (P: entrambi i pannelli)
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

# The distance between copies of a part; $dy and $gap carry their unit.
app-pitch-set = Passo { $dx } × { $dy } (distanza { $gap })
app-pitch-needs-plan = Impossibile impostare il passo da una distanza: prima va pianificato il pezzo.

app-deleted-part = { $count ->
    [0] Eliminato il pezzo { $name } (Ctrl+Z lo ripristina)
    [one] Eliminati il pezzo { $name } e la sua lavorazione (Ctrl+Z li ripristina)
   *[other] Eliminati il pezzo { $name } e le sue { $count } lavorazioni (Ctrl+Z li ripristina)
}
app-part-needed = Un progetto richiede almeno un pezzo

# A rest pass clears what a larger tool left in a pocket.
app-could-not-add-rest-pass = Impossibile aggiungere una passata di ripresa: { $error }
app-could-not-add-floor-pass = Impossibile aggiungere una passata di fondo: { $error }
app-setup-refused = Impossibile modificare i piazzamenti: { $error }
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
