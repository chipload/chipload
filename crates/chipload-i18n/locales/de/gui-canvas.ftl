# The drawing canvas: its right-click menu, hover texts and notices (crates/chipload-gui/src/app/canvas.rs).

# The right-click menu. A ring's start: where a profile or pocket lap begins.
canvas-reset-start = Start zurücksetzen
canvas-reset-start-disabled-hover = Dieser Ring beginnt, wo Chipload es gewählt hat
# $id: the contour's number.
canvas-select-contour = #{ $id } auswählen
canvas-select-layer = Layer { $layer } auswählen
canvas-right-click-hint = Rechtsklick auf eine Kontur wählt ihren Layer.

# Hovering a ring's start marker.
canvas-start-moved-hover = Start dieses Rings: entlang des Rings ziehen; Rechtsklick setzt zurück
canvas-start-hover = Start dieses Rings: entlang des Rings ziehen, um ihn zu verschieben

# Hovering plunges: "Operation 2 (Pocket 1): plunges 3, 13, 23 of 30". $operation is the
# operation's label, $numbers the list of plunge numbers, $total how many it has.
canvas-plunges = { $operation }: { $count ->
        [one] Eintauchen
       *[other] Eintauchen
    } { $numbers } von { $total }
canvas-rejected-toolpath = Verworfener Werkzeugweg

# The notice at the canvas's top.
canvas-updating-toolpath = Werkzeugweg wird aktualisiert…
canvas-toolpath-out-of-date = Werkzeugweg veraltet
# A button: generate the toolpath again.
canvas-generate = Berechnen
canvas-generate-hover = Das Projekt hat sich seit der Berechnung geändert (F5)
canvas-updating-simulation = Simulation wird aktualisiert…

# The legend in the bottom left corner.
canvas-legend = Legende
canvas-legend-hover = Was die Zeichenfläche zeigt
canvas-show-toolpath = Werkzeugweg zeigen
canvas-show-toolpath-hover = Werkzeugweg ausblenden, um die Zeichnung oder das simulierte Rohteil zu sehen
