# The drawing canvas: its right-click menu, hover texts and notices (crates/chipload-gui/src/app/canvas.rs).

# The right-click menu. A ring's start: where a profile or pocket lap begins.
canvas-reset-start = Start zurücksetzen
canvas-reset-start-disabled-hover = Dieser Ring beginnt, wo Chipload es gewählt hat
canvas-turn-axis = ⟲ Wendeachse
# $id: the contour's number.
canvas-select-contour = #{ $id } auswählen
canvas-select-layer = Layer { $layer } auswählen
canvas-select-similar = Ähnliche auswählen
canvas-select-similar-hover = Kreise gleichen Durchmessers auf jedem Layer oder gleich große Formen auf diesem Layer. Strg: zur Auswahl hinzufügen
canvas-select-inside = Innenliegende auswählen
canvas-select-inside-hover = Die geschlossenen Formen innerhalb dieser: Taschen und Bohrungen eines Teils. Strg: zur Auswahl hinzufügen
canvas-right-click-hint = Rechtsklick auf eine Kontur wählt ihren Layer.

# Hovering a ring's start marker.
canvas-start-moved-hover = Start dieses Rings: entlang des Rings ziehen; Rechtsklick setzt zurück
canvas-start-hover = Start dieses Rings: entlang des Rings ziehen, um ihn zu verschieben
canvas-tab-hover = Haltesteg: entlang des Rings ziehen; Rechtsklick, um ihn zu entfernen oder die Haltestege des Rings gleichmäßig zu verteilen
canvas-tab-hover-placed = Von Hand gesetzter Haltesteg: entlang des Rings ziehen; Rechtsklick, um ihn zu entfernen oder die Haltestege des Rings gleichmäßig zu verteilen
# A dropped tab or ring start that the planner refuses goes back; $reason says why.
canvas-drop-refused = Nicht verschoben: { $reason }
canvas-tab-remove = Haltesteg entfernen
canvas-tab-even = Haltestege dieses Rings neu setzen
canvas-tab-add = Hier einen Haltesteg hinzufügen
canvas-corner-hover = Ecke bei { $at }, { $angle }°: { $gets }. Klicken, um für sie eigens zu wählen
canvas-corner-hover-own = Ecke bei { $at }, { $angle }°: { $gets }, eigens gewählt. Klicken, um es zu ändern
canvas-corner-hover-unreached = Ecke bei { $at }, { $angle }°: { $gets }, aber das Werkzeug erreicht sie nicht, also bleibt sie rund. Klicken, um zu wählen
canvas-corner-as-operation = Wie die Bearbeitung ({ $gets })
canvas-corner-round = Rund
canvas-corner-dogbone = Dogbone
canvas-corner-tbone-longer = T-Bone entlang der längeren Wand
canvas-corner-tbone-shorter = T-Bone entlang der kürzeren Wand
canvas-corner-gets-round = rund
canvas-corner-gets-dogbone = ein Dogbone
canvas-corner-gets-tbone-longer = ein T-Bone entlang der längeren Wand
canvas-corner-gets-tbone-shorter = ein T-Bone entlang der kürzeren Wand

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

## Der Kompass (docs/specs/compass.md).
compass-menu-item = Kompass
compass-hover = Entlang eines Pfeils verschieben, mit dem Ring drehen, die Mitte auf einen Punkt ziehen; mit rechts ziehen zum Kopieren oder für eine Reihe; Leertaste löst ihn, um ihn zu platzieren
compass-select-first = Erst gezeichnete Formen, eine Gruppe, ein Bild, einen Text, ein 3D-Modell oder eine ganze Zeichnung auswählen, dann Q
compass-unlocked = Kompass gelöst: an seinen Platz ziehen, Leertaste fixiert ihn
compass-locked = Kompass fixiert: Ziehen verschiebt die Auswahl
compass-typed-unreadable = Nicht lesbar: eine Länge, einen Winkel in Grad oder ΔX und ΔY eingeben
compass-move-here = Hierher verschieben
compass-turn-here = Hierher drehen
compass-copy-here = Hierher kopieren
compass-linear-array = Lineare Reihe…
compass-circular-array = Kreisförmige Reihe…
compass-copies-drawn-only = Nur gezeichnete Formen lassen sich kopieren
compass-status = Kompass: einen Pfeil, den Ring oder die Mitte ziehen; mit rechts ziehen zum Kopieren; Leertaste löst ihn; Q oder Esc blendet ihn aus
compass-status-arrow = Ziehen verschiebt entlang dieses Pfeils; Doppelklick zum Eingeben des Abstands; mit rechts ziehen zum Kopieren oder für eine lineare Reihe; Rechtsklick, um zu einem Punkt zu verschieben
compass-status-ring = Ziehen dreht um die Mitte in 15°-Schritten (Alt: 0,1°); Doppelklick zum Eingeben des Winkels; mit rechts ziehen für eine kreisförmige Reihe
compass-status-centre = Ziehen verschiebt frei, die Mitte rastet an Punkten ein; Doppelklick zum Eingeben von ΔX und ΔY; Rechtsklick, um zu einem Punkt zu verschieben
compass-status-place = Ziehen setzt den Kompass auf einen Punkt; Leertaste fixiert ihn
compass-status-aim = Ziehen dreht die Achsen des Kompasses oder legt eine an eine Linie; Leertaste fixiert ihn
compass-to-point = Zu Punkt
compass-to-point-hover = Als Nächstes einen Punkt klicken: die Auswahl wird dorthin verschoben (mit Fang wie beim Ziehen)
compass-to-centre = Zu Mittelpunkt
compass-to-centre-hover = Als Nächstes eine Form klicken: die Auswahl wird zu ihrem Mittelpunkt verschoben (eines Kreises oder Bogens, sonst die Mitte ihres Rahmens)
compass-no-centre-here = Dort ist keine Form: eine Form klicken, um zu ihrem Mittelpunkt zu verschieben (Esc bricht ab)
compass-status-pick-point = Den Punkt klicken, zu dem die Auswahl verschoben wird (Alt: ohne Fang; Esc bricht ab)
compass-status-pick-point-axis = Einen Punkt klicken: die Auswahl wird entlang des Pfeils dorthin verschoben (Alt: ohne Fang; Esc bricht ab)
compass-status-pick-centre = Eine Form klicken: die Auswahl wird zu ihrem Mittelpunkt verschoben (Esc bricht ab)
compass-status-pick-centre-axis = Eine Form klicken: die Auswahl wird entlang des Pfeils zu ihrem Mittelpunkt verschoben (Esc bricht ab)
compass-value-distance = Abstand
compass-value-angle = Winkel
compass-value-delta = ΔX ΔY
