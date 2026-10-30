# The menu bar and its menus (crates/chipload-gui/src/app/menu.rs).

menu-language = Lingua
menu-language-system = Sistema
menu-language-system-hover = La lingua del sistema operativo se Chipload la ha, altrimenti l'inglese
menu-language-review = { $language } (revisione cercata)
menu-language-review-hover = Tradotto, non ancora letto da un madrelingua che fa lavorazioni meccaniche. Le correzioni sono benvenute: vedi la guida utente, Lingue.

# File menu.
menu-new-project = Nuovo progetto
menu-open = Apri…
menu-open-recent = Apri recenti
# Empties the list of recent projects.
menu-clear-list = Svuota elenco
menu-import-dxf = Importa DXF o SVG…
menu-import-image = Importa immagine…
menu-import-image-hover = Vettorizza un'immagine in forme, per l'incisione a V
menu-open-path = Apri percorso…
menu-import-image-path = Importa immagine da percorso…
menu-several-dxfs = Più file di disegno: aggiornane uno dal Browser (clic destro su di esso)
menu-add-dxf = Aggiungi DXF o SVG al progetto…
menu-add-dxf-hover = Un DXF o SVG in questo progetto, accanto a ciò che c'è: posizionato e aggiornato per conto suo
menu-add-dxf-path = Aggiungi DXF o SVG da percorso…
menu-save = Salva
menu-save-as = Salva come…
menu-save-path = Salva in percorso…
menu-export = Esporta G-code…
menu-export-path = Esporta G-code in percorso…
menu-export-dxf = Esporta disegno come DXF…
menu-export-dxf-hover = Le forme mostrate nell'area di disegno, sui loro layer, per un laser, un programma CAD o un preventivo
menu-construction = Costruzione
menu-construction-hover = Geometria di costruzione: mostrata a puntini e agganciabile, ma mai lavorata. Per linee e cerchi guida
menu-toggle-construction = Geometria di costruzione sì o no
menu-quit = Esci

# Edit: undo, and the selected drawn shapes.
menu-undo = Annulla
menu-redo = Ripeti
menu-only-drawn = Solo forme disegnate; quelle importate cambiano solo con il loro file
menu-copy = Copia
menu-paste = Incolla
menu-mirror-left-right = Specchia sinistra–destra
menu-mirror-up-down = Specchia alto–basso
menu-mirror-hover = Ribalta le forme disegnate selezionate attorno al centro del loro riquadro
menu-delete-shapes = Elimina forme
menu-delete-shapes-hover = Elimina le forme disegnate selezionate; quelle importate cambiano solo con il loro file
# A selector: the selection written as JSON, for scripts.
menu-copy-selector = Copia come selettore

# Job menu.
menu-cancel-generation = Annulla calcolo
menu-generate = Calcola

# View menu.
menu-fit-view = Adatta vista
menu-top-view = Vista dall'alto
menu-measure = Misura
menu-measure-hover = Due clic danno distanza, ΔX, ΔY e angolo
menu-3d-view = Vista 3D
menu-z-scale = Scala Z in 3D
menu-show-grid = Mostra griglia
menu-show-grid-hover = Linee sulla griglia di aggancio, una più marcata ogni dieci
menu-show-legend = Mostra legenda
menu-show-browser = Mostra Browser
menu-show-properties = Mostra Proprietà
menu-fold-panels = Chiudi entrambi i pannelli
menu-open-panels = Apri entrambi i pannelli
menu-panels-hover = L'area di disegno più grande: Browser e Proprietà ridotti a strisce
menu-auto-update = Aggiorna percorso utensile automaticamente
menu-auto-update-hover = Calcola da solo mezzo secondo dopo ogni modifica
menu-show-toolpath = Mostra percorso utensile
menu-show-toolpath-hover = Nascondi il percorso utensile per vedere il disegno o il grezzo simulato
menu-show-drawing = Mostra disegno
menu-show-drawing-hover = Nascondi il disegno per vedere solo il grezzo o il percorso utensile
menu-show-images = Mostra immagini
menu-show-images-hover = Le immagini vettorizzate in forme, tenui sopra il grezzo
# The stock: the material being cut, as the simulation shows it.
menu-show-simulation = Mostra grezzo (simulazione)
menu-simulation-hint = Mostra il grezzo come lo lascia il programma: ogni movimento tagliato con la forma dell'utensile (più scuro è più profondo; rosso è sotto il grezzo). Solo visualizzazione.
menu-units = Unità
menu-units-mm-hover = Lunghezze in mm, avanzamenti in mm/min
# The unit's name, as a choice next to "mm".
menu-units-inch = pollici
menu-units-inch-hover = Lunghezze in pollici, avanzamenti in in/min; i progetti restano in mm
menu-theme = Tema colori
menu-theme-system = Sistema
menu-theme-system-hover = Chiaro o scuro, come è impostato il sistema operativo
menu-theme-light = Chiaro
menu-theme-light-hover = Pannelli e area di disegno chiari
menu-theme-dark = Scuro
menu-theme-dark-hover = Pannelli e area di disegno scuri

# Draw menu.
menu-draw-circle = Cerchio
menu-draw-rectangle = Rettangolo
menu-draw-polyline = Polilinea
menu-draw-text = Testo
menu-draw-text-hover = Scrivi un testo, poi fai clic dove inizia

# Help menu.
menu-user-guide = Guida utente
menu-shortcuts = Scorciatoie da tastiera
menu-about = Informazioni e licenze

# The status bar under the canvas. What the toolpath is of, with several parts:
menu-status-part-of = pezzo { $part } di { $count }
menu-status-all-parts = tutti i pezzi
# $time: how long the program runs, e.g. "12 min 30 s".
menu-status-cycle = ciclo { $time }
menu-status-time-hover = Tempo ciclo stimato (il report ha i dettagli)
menu-status-active-part = pezzo attivo: { $name }
menu-status-active-part-hover = Le nuove lavorazioni vanno nel pezzo attivo (Browser, clic destro su un pezzo)
menu-status-units-hover = Le lunghezze sono mostrate in queste unità (Vista > Unità)
# $step: the grid's step with its unit, e.g. "10 mm".
menu-status-grid = griglia { $step }
menu-status-grid-hover = Il passo della griglia: i clic durante il disegno vi si agganciano (G la nasconde)
menu-status-post-hover = Il post-processore (Impostazioni lavoro > Macchina)
# Pieces in all, from every part that is cut.
menu-status-pieces = { $count ->
    [one] { $count } pezzo
   *[other] { $count } pezzi
}
menu-status-copies = { $count ->
    [one] { $count } copia
   *[other] { $count } copie
}
# The cursor in program coordinates (from the work zero), after the drawing's X and Y.
menu-status-work = (pezzo X { $x }  Y { $y })
# $target: what the cursor snapped to, e.g. "end of #4".
menu-snapped-to = — agganciato a { $target }
menu-generating = Calcolo…
# $snapped: empty, or "— snapped to …" with a space before it.
menu-dragging = Trascinamento a ({ $x }, { $y }){ $snapped } (Alt: niente aggancio; su un segmento Ctrl inserisce un punto, Shift lo curva)
menu-status-hint = Clic su un contorno per selezionarlo · Ctrl+clic aggiunge o toglie · clic destro seleziona il suo layer · Esc annulla la selezione
menu-join-ends = Unisci estremi…
menu-join-ends-hover = Unisci in una sola le polilinee disegnate aperte i cui estremi quasi si toccano: quelle selezionate, o tutte se nessuna è selezionata
menu-join-ends-disabled-hover = Non ci sono forme disegnate. Le forme di un DXF si possono unire dopo averlo esploso (clic destro su di esso nel Browser).
menu-draw-arc = Arco
menu-draw-slot = Asola
menu-draw-polygon = Poligono
menu-polygon-sides = Lati
menu-modify-offset = Offset
menu-modify-fillet = Raccordo
menu-modify-chamfer = Smusso
menu-modify-trim = Taglia
menu-modify-extend = Estendi
menu-modify-break = Spezza
menu-modify-union = Unisci
menu-modify-subtract = Sottrai
menu-modify-intersect = Interseca
menu-modify-array = Serie
menu-modify-polar-array = Serie polare
