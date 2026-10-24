# The drawing canvas: its right-click menu, hover texts and notices (crates/chipload-gui/src/app/canvas.rs).

# The right-click menu. A ring's start: where a profile or pocket lap begins.
canvas-reset-start = Ripristina inizio
canvas-reset-start-disabled-hover = Questo anello inizia dove ha scelto Chipload
# $id: the contour's number.
canvas-select-contour = Seleziona #{ $id }
canvas-select-layer = Seleziona layer { $layer }
canvas-right-click-hint = Clic destro su un contorno per selezionarne il layer.

# Hovering a ring's start marker.
canvas-start-moved-hover = Inizio di questo anello: trascinalo lungo l'anello; clic destro per ripristinarlo
canvas-start-hover = Inizio di questo anello: trascinalo lungo l'anello per spostarlo

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
