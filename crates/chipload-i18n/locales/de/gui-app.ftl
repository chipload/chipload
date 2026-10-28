# The main window: opening, importing, saving and exporting, parts, rest passes, the
# selection (crates/chipload-gui/src/app/mod.rs).

# The buttons of a folded side panel; P is the key that folds and unfolds both panels.
app-show-browser-hover = Browser zeigen (P: beide Bereiche)
app-show-properties-hover = Eigenschaften zeigen (P: beide Bereiche)
# The heading of the right-hand panel.
app-properties = Eigenschaften

app-start-hint = Importieren Sie eine DXF oder SVG (Ctrl+I), um ein Projekt zu beginnen, oder öffnen Sie eines (Ctrl+O).

app-cannot-open = { $file } kann nicht geöffnet werden:
app-project = Projekt: { $file }
app-old-schema = Diese Datei hat Schemaversion { $version }; beim Speichern wird Version { $current } geschrieben.

app-new-project = Neues Projekt.
# $what: a machine or tool from the user's library, e.g. machine "Router".
app-started-with = Begonnen mit { $what }.
app-new-project-next = Zeichnen Sie Formen über das Menü Zeichnen, oder holen Sie eine Zeichnung mit Datei > DXF ins Projekt importieren. Prüfen Sie Rohteil, Maschine und Werkzeug unter Jobeinstellungen.
app-units-missing = { $file } gibt keine Einheiten an; wählen Sie mm oder Zoll.
app-px-missing = { $file } gibt seine Größe nur in Pixeln an; wählen Sie die Pixel pro Zoll.
app-cannot-import = { $file } kann nicht importiert werden:
app-new-project-from = Neues Projekt aus { $file } ({ $shapes } Formen).
app-import-next = Wählen Sie Konturen, dann Neu: Konturfräsen oder Neue Tasche. Prüfen Sie Rohteil, Maschine und Werkzeug unter Jobeinstellungen.

# Followed by the list of problems.
app-project-incomplete = Das Projekt ist nicht vollständig:
app-lost-geometry = Einige Bearbeitungen haben ihre Geometrie verloren: wählen Sie sie neu
app-could-not-update-drawing = Die Zeichnung konnte nicht aktualisiert werden:
app-could-not-add = { $file } konnte nicht hinzugefügt werden:
app-could-not-move-drawing = Die Zeichnung konnte nicht verschoben werden:

# The distance between copies of a part; $dy and $gap carry their unit.
app-pitch-set = Teilung { $dx } × { $dy } (Abstand { $gap })
app-pitch-needs-plan = Die Teilung konnte nicht aus einem Abstand bestimmt werden: das Teil muss zuerst geplant werden.

app-deleted-part = { $count ->
    [0] Teil { $name } gelöscht (Ctrl+Z holt es zurück)
    [one] Teil { $name } und seine Bearbeitung gelöscht (Ctrl+Z holt sie zurück)
   *[other] Teil { $name } und seine { $count } Bearbeitungen gelöscht (Ctrl+Z holt sie zurück)
}
app-part-needed = Ein Projekt braucht mindestens ein Teil

# A rest pass clears what a larger tool left in a pocket.
app-could-not-add-rest-pass = Restdurchgang konnte nicht hinzugefügt werden: { $error }
app-rest-with-corners = Diese Tasche hat Dogbone- oder T-Bone-Ecken, und Restdurchgänge lassen sich noch nicht damit kombinieren. Die Ecken reichen schon dorthin, wo ein Restdurchgang hinkäme.
# $tool: a tool number such as T3.
app-rest-tool-missing = Werkzeug { $tool } ist nicht im Projekt: geben Sie der Tasche zuerst ein Werkzeug.
app-rest-no-smaller = Kein Schaftfräser kleiner als { $tool } ({ $diameter }) im Projekt oder in Ihrer Werkzeugbibliothek. Fügen Sie einen unter Werkzeuge hinzu und klicken Sie dann erneut mit rechts auf die Tasche.
app-now-rest-pass = Bearbeitung { $op } ist jetzt ein Restdurchgang von Bearbeitung { $parent }; sie wird nach der Folge von Taschen gefräst, zu der sie gehört

app-generate-before-export = Vor dem Export des G-Codes berechnen (F5).
app-planning-then-export = Alle Teile werden geplant, dann exportiert
app-could-not-export = { $path } konnte nicht exportiert werden:
app-dxf-exported = { $path } geschrieben.
app-could-not-export-dxf = Die Zeichnung wurde nicht exportiert:
app-sheet-failed = G-Code exportiert, aber das Einrichteblatt { $path } konnte nicht geschrieben werden:
app-exported-n-files = { $count } Dateien exportiert: zwischen ihnen das Werkzeug wechseln und Z-Null setzen, X- und Y-Null beibehalten.
app-sheet-for-operator = Einrichteblatt für den Bediener: { $path }
app-exported-files = { $count } Dateien neben { $path } exportiert
app-exported-files-sheet = { $count } Dateien und das Einrichteblatt { $sheet } neben { $path } exportiert
app-exported = { $path } exportiert
app-exported-sheet = { $path } und das Einrichteblatt { $sheet } exportiert

app-cannot-save-yet = Das Projekt kann noch nicht gespeichert werden:
app-could-not-save = { $path } konnte nicht gespeichert werden:
app-saved = { $path } gespeichert

# What is selected on the canvas.
app-nothing-selected = Nichts ausgewählt.
# $count contours selected; $what says how (one of the texts below); $list their numbers.
app-selected = { $count } ausgewählt ({ $what }): { $list }
app-selection-layer = Layer { $layer }
app-selection-dxf = DXF { $id }
app-selection-image = Bild { $id }
app-selection-text = Text { $id }
# $source: one of the three texts above.
app-selection-left-out = { $source }, { $count } ausgelassen
app-selection-contours = Konturen
# Open contours (not closed) are not cut.
app-open-not-cut = Offen (nicht gefräst): { $list }

# File menu, for a project started without a drawing.
app-import-dxf-into-project = DXF oder SVG ins Projekt importieren…
app-import-dxf-into-project-path = DXF oder SVG ins Projekt aus Pfad importieren…
# File menu, a new version of the project's drawing.
app-update-drawing-from-dxf = Zeichnung aus DXF oder SVG aktualisieren…
app-update-drawing-from-path = Zeichnung aus Pfad aktualisieren…
