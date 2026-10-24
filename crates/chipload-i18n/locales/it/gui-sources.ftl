# The Browser's Drawing folder: the project's DXFs with their layers, its pictures, texts
# and drawn shapes (crates/chipload-gui/src/app/sources.rs).

# The folder's name: everything that is drawn, a noun.
sources-drawing = Disegno
sources-drawing-hover = DXF, immagini, testi e forme disegnate; clic destro per aggiungerne
sources-add-dxf = Aggiungi DXF…
sources-add-dxf-hover = Un altro pezzo: un DXF posizionato e aggiornato per conto suo
# "File > Import DXF" names a menu item.
sources-add-dxf-disabled-hover = File > Importa DXF avvia un progetto da un DXF
sources-import-image = Importa immagine…
sources-import-image-hover = Vettorizza un'immagine in forme, per l'incisione a V
sources-add-text = Aggiungi testo
sources-add-text-hover = Scrivi un testo, poi fai clic dove inizia
sources-no-shapes = Ancora nessuna forma.
# The name of DXF files in the file dialog's filter.
sources-dxf-filter = Disegni DXF

# Rows of the folder. $file: the picture's file name; $text: what the text says.
sources-image-title = { $file } (immagine { $id })
sources-text-title = { $text } (testo { $id })
# Rows for the shapes drawn in the app, by kind.
sources-rectangles = Rettangoli
sources-circles = Cerchi
sources-polylines = Polilinee
sources-other-paths = Altri percorsi

sources-dxf-hover = Il clic seleziona tutto il DXF: una lavorazione creata da esso ne segue gli aggiornamenti. Posizione e scala sono in Proprietà; clic destro per altro
sources-image-hover = Il clic seleziona tutta l'immagine: una lavorazione creata da essa segue le sue nuove vettorizzazioni. Dimensione e posizione sono in Proprietà; doppio clic per le impostazioni di vettorizzazione
sources-text-hover = Il clic seleziona tutto il testo: una lavorazione creata da esso ne segue le modifiche. Contenuto, font, dimensione e posizione sono in Proprietà
sources-drawn-hover = Il clic seleziona queste forme; Ctrl+clic le aggiunge
sources-layer-hover = Il clic seleziona questo layer di questo DXF: una lavorazione creata da esso lavora solo questo layer del DXF

# Right-click menu items, verbs.
sources-select = Seleziona
sources-show = Mostra
sources-hide = Nascondi
sources-place = Posiziona…
sources-place-hover = Sposta, ruota o scala questo DXF, nelle sue proprietà
sources-update-from-dxf = Aggiorna da un DXF…
sources-update-from-path = Aggiorna da un percorso…
sources-remove-dxf-hover = Rimuovi questo DXF e le sue forme
sources-trace-settings = Vettorizzazione…
sources-trace-settings-hover = Soglia, levigatura, contorni o linee mediane; dimensione e posizione sono nelle sue proprietà
sources-remove-image-hover = Rimuovi l'immagine e le sue forme
sources-remove-text-hover = Rimuovi il testo e le sue lettere

# Followed by the project's problems, one per line.
sources-project-incomplete = Il progetto non è completo:
# Followed by the error on its own line.
sources-remove-dxf-failed = Impossibile rimuovere il DXF:
