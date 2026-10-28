# The header: the top bar and the ribbon of grouped buttons (crates/chipload-gui/src/app/header.rs).

# Export's hover text. $count is the number of parts that are cut (always more than one),
# $part the name of the part the preview shows.
header-export-hover = Salva il G-code verificato
header-export-hover-parts = Salva il G-code verificato di ogni pezzo lavorato ({ $count } pezzi)
header-export-hover-parts-shown = Salva il G-code verificato di ogni pezzo lavorato ({ $count } pezzi); l'anteprima mostra solo "{ $part }"
header-export-generating-hover = Salva il G-code appena è calcolato
header-export-first-hover = Calcola prima

# The top bar's icon buttons.
header-open-hover = Apri un progetto
header-save-hover = Salva il progetto
header-undo = Annulla
header-redo = Ripeti
header-help = Aiuto
# The colour theme button: what is set now, a click goes on to the next.
header-theme-system-hover = Tema colori: come il sistema. Clic per il successivo.
header-theme-light-hover = Tema colori: chiaro. Clic per il successivo.
header-theme-dark-hover = Tema colori: scuro. Clic per il successivo.

# The tab with the open project's file name.
header-new-project = Nuovo progetto
header-tab-unsaved-hover =
    { $path }
    Modifiche non salvate
header-tab-not-saved-hover = Non ancora salvato

# The ribbon's groups: their captions (shown in capitals) open the menu of the same name.
header-group-file = File
header-group-draw = Disegna
header-group-operation = Lavorazione
header-group-library = Libreria
header-group-view = Vista
header-group-hover = Tutto in { $group }

# FILE: short words under icons.
header-new = Nuovo
header-new-hover = Nuovo progetto
header-open = Apri
header-import = Importa
header-import-hover = Nuovo progetto da un DXF
header-save = Salva

# DRAW: short words under icons, and what each does.
header-circle = Cerchio
header-circle-hover = Disegna un cerchio: centro, poi un punto sul cerchio
header-rectangle = Rettangolo
header-rectangle-hover = Disegna un rettangolo: due angoli opposti
header-polyline = Polilinea
header-polyline-hover = Disegna una polilinea: fai clic sui punti
header-text = Testo
header-text-hover = Scrivi un testo: fai clic dove inizia, poi digitalo in Proprietà
header-image = Immagine
header-image-hover = Vettorizza un'immagine in forme, per un'incisione a V o un segui percorso
header-add-dxf = Aggiungi DXF
header-add-dxf-hover = Un altro pezzo: un DXF posizionato e aggiornato per conto suo
# "FILE > Import": the ribbon group FILE and its Import button.
header-add-dxf-first-hover = Importa prima un DXF (FILE > Importa)
header-import-image-menu = Importa immagine…
header-add-dxf-menu = Aggiungi DXF al progetto…

# OPERATION: "Follow" is the short word under the icon for a Follow path operation.
header-follow = Segui
# $operation is what the operation does (a hover text of its own).
header-needs-selection-hover = { $operation }. Seleziona prima le forme.
header-select-shapes-first = Seleziona prima le forme
header-new-profile = Nuova contornatura
header-new-pocket = Nuova tasca
header-new-vcarve = Nuova incisione a V
header-new-follow-path = Nuovo segui percorso
header-new-holes = Nuovi fori
header-new-surface = Nuova spianatura

# LIBRARY
header-tools = Utensili
header-tools-hover = La tua libreria utensili, sotto Questo computer nel Browser
header-machines = Macchine
header-machines-hover = I tuoi profili macchina, sotto Questo computer nel Browser
header-tool-library = Libreria utensili

# VIEW
header-measure = Misura
header-measure-hover = Due clic danno distanza, ΔX, ΔY e angolo. Ciò che viene mostrato è sulla barra dell'area di disegno.

# Generate and Export at the ribbon's right end.
header-export = Esporta
header-cancel = Annulla
header-cancel-hover = Interrompi il calcolo
header-generate = Calcola
header-generate-hover = Pianifica i percorsi utensile, scrivi e verifica il G-code
header-arc = Arco
header-arc-hover = Disegna un arco: inizio, fine, poi un punto per cui passa
header-slot = Asola
header-slot-hover = Disegna un'asola: i centri delle estremità, poi la larghezza
header-polygon = Poligono
header-polygon-hover = Disegna un poligono regolare: il centro, poi un vertice (il numero di lati nella barra di stato)
header-group-modify = Modifica
header-offset = Offset
header-offset-hover = Una forma a distanza da quelle selezionate: spostati dal lato voluto, clicca o digita la distanza
header-fillet = Raccordo
header-fillet-hover = Arrotonda un angolo della forma selezionata con un arco: clicca l'angolo; Maiusc+clic li arrotonda tutti
header-chamfer = Smusso
header-chamfer-hover = Taglia dritto un angolo della forma selezionata: clicca l'angolo; Maiusc+clic li taglia tutti
header-trim = Taglia
header-trim-hover = Togli il pezzo di una forma disegnata sotto il cursore, fin dove altre forme la incrociano: clicca il pezzo
header-extend = Estendi
header-extend-hover = Allunga l'estremità diritta di una forma aperta fino alla forma successiva: clicca vicino all'estremità
header-break = Spezza
header-break-hover = Spezza una forma disegnata nel punto cliccato: una aperta in due, una chiusa si apre lì
