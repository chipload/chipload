# The drawing canvas: its right-click menu, hover texts and notices (crates/chipload-gui/src/app/canvas.rs).

# The right-click menu. A ring's start: where a profile or pocket lap begins.
canvas-reset-start = Ripristina inizio
canvas-reset-start-disabled-hover = Questo anello inizia dove ha scelto Chipload
canvas-turn-axis = ⟲ Asse di capovolgimento
# $id: the contour's number.
canvas-select-contour = Seleziona #{ $id }
canvas-select-layer = Seleziona layer { $layer }
canvas-select-similar = Seleziona simili
canvas-select-similar-hover = Cerchi dello stesso diametro su qualsiasi layer, o forme della stessa misura su questo layer. Ctrl: aggiungi alla selezione
canvas-select-inside = Seleziona interne
canvas-select-inside-hover = Le forme chiuse dentro questa: tasche e fori di un pezzo. Ctrl: aggiungi alla selezione
canvas-right-click-hint = Clic destro su un contorno per selezionarne il layer.

# Hovering a ring's start marker.
canvas-start-moved-hover = Inizio di questo anello: trascinalo lungo l'anello; clic destro per ripristinarlo
canvas-start-hover = Inizio di questo anello: trascinalo lungo l'anello per spostarlo
canvas-tab-hover = Ponticello: trascinalo lungo l'anello; clic destro per rimuoverlo o distribuire uniformemente i ponticelli dell'anello
canvas-tab-hover-placed = Ponticello posto a mano: trascinalo lungo l'anello; clic destro per rimuoverlo o distribuire uniformemente i ponticelli dell'anello
# A dropped tab or ring start that the planner refuses goes back; $reason says why.
canvas-drop-refused = Non spostato: { $reason }
canvas-tab-remove = Rimuovi il ponticello
canvas-tab-even = Riposiziona i ponticelli di questo anello
canvas-tab-add = Aggiungi un ponticello qui
canvas-corner-hover = Angolo in { $at }, { $angle }°: { $gets }. Clicca per scegliere a parte
canvas-corner-hover-own = Angolo in { $at }, { $angle }°: { $gets }, scelto a parte. Clicca per cambiarlo
canvas-corner-hover-unreached = Angolo in { $at }, { $angle }°: { $gets }, ma l'utensile non ci arriva, quindi resta arrotondato. Clicca per scegliere
canvas-corner-as-operation = Come la lavorazione ({ $gets })
canvas-corner-round = Arrotondato
canvas-corner-dogbone = Dogbone
canvas-corner-tbone-longer = T-bone lungo la parete più lunga
canvas-corner-tbone-shorter = T-bone lungo la parete più corta
canvas-corner-gets-round = arrotondato
canvas-corner-gets-dogbone = un dogbone
canvas-corner-gets-tbone-longer = un T-bone lungo la parete più lunga
canvas-corner-gets-tbone-shorter = un T-bone lungo la parete più corta

# Hovering plunges: "Operation 2 (Pocket 1): plunges 3, 13, 23 of 30". $operation is the
# operation's label, $numbers the list of plunge numbers, $total how many it has.
canvas-plunges = { $operation }: { $count ->
        [one] affondamento
       *[other] affondamenti
    } { $numbers } di { $total }
canvas-rejected-toolpath = Percorso utensile rifiutato

# The notice at the canvas's top.
canvas-updating-toolpath = Aggiornamento del percorso utensile…
canvas-toolpath-out-of-date = Percorso utensile non aggiornato
# A button: generate the toolpath again.
canvas-generate = Calcola
canvas-generate-hover = Il progetto è cambiato dall'ultimo calcolo (F5)
canvas-updating-simulation = Aggiornamento della simulazione…

# The legend in the bottom left corner.
canvas-legend = Legenda
canvas-legend-hover = Cosa mostra l'area di disegno
canvas-show-toolpath = Mostra percorso utensile
canvas-show-toolpath-hover = Nascondi il percorso utensile per vedere il disegno o il grezzo simulato

## La bussola (docs/specs/compass.md).
compass-menu-item = Bussola
compass-hover = Sposta lungo una freccia, ruota con l’anello, trascina il centro su un punto; trascina col tasto destro per copiare o fare una serie; Spazio la sblocca per posizionarla
compass-select-first = Seleziona prima forme disegnate, un gruppo, un’immagine, un testo, un modello 3D o un disegno intero, poi Q
compass-unlocked = Bussola sbloccata: trascinala in posizione, Spazio la blocca
compass-locked = Bussola bloccata: trascinare sposta la selezione
compass-typed-unreadable = Non leggibile: scrivi una lunghezza, un angolo in gradi, oppure ΔX e ΔY
compass-move-here = Sposta qui
compass-turn-here = Ruota qui
compass-copy-here = Copia qui
compass-linear-array = Serie lineare…
compass-circular-array = Serie circolare…
compass-copies-drawn-only = Si possono copiare solo forme disegnate
compass-status = Bussola: trascina una freccia, l’anello o il centro; tasto destro per copiare; Spazio la sblocca; Q o Esc la nasconde
compass-status-arrow = Trascina per spostare lungo questa freccia; doppio clic per scrivere la distanza; tasto destro per copiare o fare una serie lineare; clic destro per spostare a un punto
compass-status-ring = Trascina per ruotare attorno al centro a passi di 15° (Alt: 0,1°); doppio clic per scrivere l’angolo; tasto destro per una serie circolare
compass-status-centre = Trascina per spostare liberamente, il centro aggancia i punti; doppio clic per scrivere ΔX e ΔY; clic destro per spostare a un punto
compass-status-place = Trascina per mettere la bussola su un punto; Spazio la blocca
compass-status-aim = Trascina per ruotare gli assi della bussola, o allinearne uno a una linea; Spazio la blocca
compass-to-point = Al punto
compass-to-point-hover = Fai clic su un punto: la selezione si sposta lì (con aggancio come trascinando)
compass-to-centre = Al centro
compass-to-centre-hover = Fai clic su una forma: la selezione va al suo centro (di un cerchio o di un arco, altrimenti il centro del suo riquadro)
compass-no-centre-here = Lì non c’è nessuna forma: fai clic su una forma per andare al suo centro (Esc annulla)
compass-status-pick-point = Fai clic sul punto in cui spostare la selezione (Alt: senza aggancio; Esc annulla)
compass-status-pick-point-axis = Fai clic su un punto: la selezione ci va lungo la freccia (Alt: senza aggancio; Esc annulla)
compass-status-pick-centre = Fai clic su una forma: la selezione va al suo centro (Esc annulla)
compass-status-pick-centre-axis = Fai clic su una forma: la selezione va lungo la freccia fino al suo centro (Esc annulla)
