# 3D models in the window: File > Add 3D model…, their rows in the Browser and their
# properties (crates/chipload-gui/src/app/models.rs, sources.rs, forms/sources.rs).

models-stl-filter = STL-Modelle
models-cannot-read = { $path } kann nicht gelesen werden: { $error }
models-reading = { $file } wird gelesen…
models-cannot-add = { $path } kann nicht hinzugefügt werden:
models-cannot-place = Das Modell lässt sich so nicht platzieren:
models-removed = Modell { $id } entfernt
dialogs-path-model-title = 3D-Modell hinzufügen
dialogs-path-model = Eine STL-Datei (binär oder Text):
menu-add-model = 3D-Modell hinzufügen…
menu-add-model-hover = Eine STL-Datei, von oben als Relief ins Brett gefräst
menu-add-model-path = 3D-Modell aus Pfad hinzufügen…
sources-model-title = { $name } (Modell { $id })
sources-add-model = 3D-Modell hinzufügen…
sources-add-model-hover = Eine STL-Datei, von oben als Relief ins Brett gefräst
sources-model-hover = Ein 3D-Modell: sein Umriss auf dem Brett. Doppelklick für Lage, Größe und Tiefe
sources-model-new-relief = Neues Relief
sources-model-new-relief-hover = Dieses Modell fräsen: eine Relief-Bearbeitung in seinem Umriss, im aktiven Teil
sources-model-properties = Eigenschaften
sources-remove-model-hover = Modell und Umriss entfernen; seine Reliefs bleiben, jedes mit einem Problem, bis es ein anderes Modell bekommt
forms-model-file = Datei
forms-model-size = Auf dem Brett
forms-model-size-hover = Breite × Höhe × Tiefe des platzierten Modells
forms-model-position-hover = Die untere linke Ecke des Modellumrisses, vor dem Drehen
forms-model-reset-position = Zurück zum Ursprung der Zeichnung
forms-model-rotation-hover = Gegen den Uhrzeigersinn um die Mitte des Umrisses gedreht
forms-model-reset-rotation = Nicht gedreht
forms-model-width-hover = Die Breite des Modells auf dem Brett; die Höhe folgt
forms-model-depth = Tiefe
forms-model-depth-hover = Wie tief das Relief geht, von der Oberseite des Modells bis zu seinem tiefsten Punkt. Reliefs sind meist flacher als das Modell
forms-model-proportional = Proportionen beibehalten
forms-model-proportional-hover = Die Tiefe folgt der Breite, im Verhältnis des Modells
forms-model-up = Oben
forms-model-up-hover = Welche Achse des Modells vom Brett nach oben zeigt. Viele Dateien sind mit Y nach oben modelliert
forms-model-keep = Behalten von oben
forms-model-keep-hover = Wie viel des Modells von oben behalten wird: der Rest, seine Rückseite, wird abgeschnitten. Breite und Tiefe gelten für den behaltenen Teil.
forms-model-layer-hover = Die Ebene, auf der der Umriss des Modells gezeichnet wird
forms-model-new-relief = Neues Relief
forms-model-new-relief-hover = Dieses Modell fräsen: eine Relief-Bearbeitung in seinem Umriss, im aktiven Teil
forms-remove-model-hover = Modell und Umriss entfernen; seine Reliefs bleiben, jedes mit einem Problem, bis es ein anderes Modell bekommt

# Reliefs and their roughing pass (docs/specs/stl-relief.md §9, §14).
models-could-not-add-roughing = Der Schruppdurchgang konnte nicht hinzugefügt werden: { $error }
models-no-roughing-tool = Kein Schaftfräser und kein einfacher Kugelfräser im Projekt oder in Ihrer Bibliothek

# Turn and cut… (docs/specs/relief-wizard.md §4)
turn-cut-title = Drehen und schneiden
turn-cut-body = Wähle, welche Seite des Modells nach oben zeigt und wie viel davon von oben behalten wird. Ziehe am Modell, um es von allen Seiten zu sehen; das Relief daraus steht daneben.
turn-cut-model = Das Modell (ziehen zum Drehen)
turn-cut-model-hover = Farbig: der behaltene Teil. Grau: der abgeschnittene Teil. Die Platte ist die Schnitthöhe.
turn-cut-relief = Das Relief daraus, von oben
turn-cut-size = { $width } × { $height }, { $depth } tief
turn-cut-face-up = Nach oben
turn-cut-turn = Auf der Platte drehen
turn-cut-turn-left = Eine Vierteldrehung gegen den Uhrzeigersinn
turn-cut-turn-right = Eine Vierteldrehung im Uhrzeigersinn
forms-model-turn-cut = Drehen und schneiden…
forms-model-turn-cut-hover = Welche Seite nach oben zeigt und wie viel behalten wird, gewählt mit Blick auf Modell und Relief
