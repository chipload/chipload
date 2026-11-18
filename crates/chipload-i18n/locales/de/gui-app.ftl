# The main window: opening, importing, saving and exporting, parts, rest passes, the
# selection (crates/chipload-gui/src/app/mod.rs).

# The heading of the right-hand panel.
app-properties = Eigenschaften

app-start-hint = Importieren Sie eine DXF oder SVG (Ctrl+I), um ein Projekt zu beginnen, oder öffnen Sie eines (Ctrl+O).

app-cannot-open = { $file } kann nicht geöffnet werden:
app-project = Projekt: { $file }
app-old-schema = Diese Datei hat Schemaversion { $version }; beim Speichern wird Version { $current } geschrieben.

app-new-project = Neues Projekt.
# $what: a machine or tool from the user's library, e.g. machine "Router".
app-started-with = Begonnen mit { $what }.
app-new-project-next = Zeichnen Sie Formen über das Menü Zeichnen, oder holen Sie eine Zeichnung mit „DXF oder SVG zum Projekt hinzufügen“ dazu. Prüfen Sie die Platte unter der Aufspannung, die Maschine unter Maschine und das Werkzeug unter Werkzeuge.
app-units-missing = { $file } gibt keine Einheiten an; wählen Sie mm oder Zoll.
app-px-missing = { $file } gibt seine Größe nur in Pixeln an; wählen Sie die Pixel pro Zoll.
app-cannot-import = { $file } kann nicht importiert werden:
app-new-project-from = Neues Projekt aus { $file } ({ $shapes } Formen).
app-import-next = Wählen Sie Konturen, dann Neu: Konturfräsen oder Neue Tasche. Prüfen Sie die Platte unter der Aufspannung, die Maschine unter Maschine und das Werkzeug unter Werkzeuge.

# Followed by the list of problems.
app-project-incomplete = Das Projekt ist nicht vollständig:
app-lost-geometry = Einige Bearbeitungen haben ihre Geometrie verloren: wählen Sie sie neu
app-could-not-update-drawing = Die Zeichnung konnte nicht aktualisiert werden:
app-could-not-add = { $file } konnte nicht hinzugefügt werden:
app-could-not-move-drawing = Die Zeichnung konnte nicht verschoben werden:

# The distance between a grid's cells; $dy and $gap carry their unit.
app-pitch-set = Teilung { $dx } × { $dy } (Abstand { $gap })
app-pitch-needs-plan = Die Teilung konnte nicht aus einem Abstand bestimmt werden: das Teil muss zuerst geplant werden.
app-instance-needs-plan = Die neue Instanz konnte nicht platziert werden: das Teil muss zuerst geplant werden.

app-deleted-part = { $count ->
    [0] Teil { $name } gelöscht (Ctrl+Z holt es zurück)
    [one] Teil { $name } und seine Bearbeitung gelöscht (Ctrl+Z holt sie zurück)
   *[other] Teil { $name } und seine { $count } Bearbeitungen gelöscht (Ctrl+Z holt sie zurück)
}
app-part-needed = Ein Projekt braucht mindestens ein Teil
app-board-surface-stays = Das Planen des ganzen Bretts gehört zur Aufspannung und bleibt darin zuerst

# A rest pass clears what a larger tool left in a pocket.
app-could-not-add-rest-pass = Restdurchgang konnte nicht hinzugefügt werden: { $error }
app-could-not-add-floor-pass = Bodendurchgang konnte nicht hinzugefügt werden: { $error }
app-setup-refused = Die Aufspannungen konnten nicht geändert werden: { $error }
app-dowels-placed = Passstiftbohrungen neu platziert, frei von den Schnitten beider Seiten. Der Nullpunkt ist mit dem ersten Stift mitgegangen, wenn er dort lag.
app-setup-has-operations = Die Aufspannung hat Teile mit Bearbeitungen: verschieben Sie sie zuerst in eine andere Aufspannung oder löschen Sie sie.
app-exported-setup = Nur Aufspannung „{ $name }“: machen Sie eine andere Aufspannung aktiv und exportieren Sie auch sie.
app-floor-no-endmill = Kein Schaftfräser im Projekt oder in Ihrer Werkzeugbibliothek
app-rest-with-corners = Diese Tasche hat Dogbone- oder T-Bone-Ecken, und Restdurchgänge lassen sich noch nicht damit kombinieren. Die Ecken reichen schon dorthin, wo ein Restdurchgang hinkäme.
# $tool: a tool number such as T3.
app-rest-tool-missing = Werkzeug { $tool } ist nicht im Projekt: geben Sie der Tasche zuerst ein Werkzeug.
app-rest-no-smaller = Kein Schaftfräser kleiner als { $tool } ({ $diameter }) im Projekt oder in Ihrer Werkzeugbibliothek. Fügen Sie einen unter Werkzeuge hinzu und klicken Sie dann erneut mit rechts auf die Tasche.
app-now-rest-pass = Bearbeitung { $op } ist jetzt ein Restdurchgang von Bearbeitung { $parent }; sie wird nach der Folge von Taschen gefräst, zu der sie gehört

app-generate-before-export = Vor dem Export des G-Codes berechnen (F5).
app-planning-then-export = Alle Teile werden geplant, dann exportiert
app-planning-part-then-export = Teil { $name } wird geplant, dann exportiert
app-could-not-export = { $path } konnte nicht exportiert werden:
app-dxf-exported = { $path } geschrieben.
app-could-not-export-dxf = Die Zeichnung wurde nicht exportiert:
app-sheet-failed = G-Code exportiert, aber das Einrichteblatt { $path } konnte nicht geschrieben werden:
app-exported-n-files = { $count } Dateien exportiert: zwischen ihnen das Werkzeug wechseln und Z-Null setzen, X- und Y-Null beibehalten.
app-sheet-for-operator = Einrichteblatt für den Bediener: { $path }
app-exported-files = { $count } Dateien neben { $path } exportiert
app-exported-files-sheet = { $count } Dateien und das Einrichteblatt { $sheet } neben { $path } exportiert
app-exported = { $path } exportiert
app-export-all-running = Jede Aufspannung wird berechnet und geschrieben…
app-exported-all = { $setups } Aufspannungen exportiert, { $count } Dateien, neben { $path }
app-export-all-failed = Die Aufspannungen konnten nicht exportiert werden; nichts wurde geschrieben
app-export-all-stopped = Der Export aller Aufspannungen wurde unerwartet abgebrochen; womöglich wurde nichts geschrieben
app-exported-sheet = { $path } und das Einrichteblatt { $sheet } exportiert

app-cannot-save-yet = Das Projekt kann noch nicht gespeichert werden:
app-could-not-save = { $path } konnte nicht gespeichert werden:
app-saved = { $path } gespeichert

# What is selected on the canvas.
app-nothing-selected = Nichts ausgewählt.
# $count contours selected; $what says how (one of the texts below); $list their numbers.
app-selected = { $count } ausgewählt ({ $what }): { $list }
app-selection-layer = Layer { $layer }
app-selection-dxf = Zeichnung { $id }
app-selection-image = Bild { $id }
app-selection-model = Modell { $id }
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

# The Nest window (docs/specs/nesting.md §7)
nest-menu = Verschachteln…
nest-menu-hover = Verschachteln…: jede Instanz der Teile der Aufspannung so dicht wie möglich auf ihr Brett gelegt, getrennt durch den Abstand; erst eine Vorschau, in einem Dialog
nest-again = Erneut verschachteln
nest-again-hover = Die Aufspannung erneut verschachteln, mit Abstand, Rand und Drehungen vom letzten Mal, ohne Dialog
nest-title = Verschachteln: { $setup }
nest-title-one = Verschachteln
nest-board-follows = Das Brett folgt der Zeichnung: Verschachteln gibt ihm diese Größe, seine Ecke im Ursprung der Zeichnung.
nest-board = Brett
nest-gap = Abstand
nest-gap-hover = Der kleinste Abstand zwischen dem, was zwei Instanzen fräsen: ein Steg, der die Platte steif hält und die Haltestege halten lässt
nest-margin = Rand
nest-margin-hover = Der Platz am Rand des Bretts, für Schrauben oder Zwingen
nest-goal = Ziel
nest-goal-hover = Was die Verschachtelung so klein wie möglich macht, nachdem sie so viel wie möglich platziert hat
nest-goal-compact = Kompakt
nest-goal-compact-hover = Der kleinste Rahmen um die Teile: ein Block in der linken unteren Ecke, der Rest des Bretts ein L darum
nest-goal-low = Oben frei lassen
nest-goal-low-hover = Die Teile so tief wie möglich, über die ganze Breite: oben bleibt ein Streifen frei
nest-goal-left = Rechts frei lassen
nest-goal-left-hover = Die Teile so weit links wie möglich, über die ganze Höhe: rechts bleibt ein Streifen frei
nest-overflow = Was nicht passt
nest-overflow-hover = Ausgeschaltet und genannt, oder auf weitere Rohteile gelegt: die nächste Aufspannung mit demselben Rohteil, sonst eine neue wie diese
nest-overflow-off = Ausschalten
nest-overflow-boards = Auf weitere Rohteile
nest-part = Teil
nest-count = Anzahl
nest-count-hover = Wie oft das Teil gefräst wird: zuerst werden Instanzen hinzugefügt oder entfernt, nie das Teil selbst
nest-turns = Drehen
nest-turns-none = Wie es liegt
nest-turns-half = Halbe Drehungen
nest-turns-quarter = Vierteldrehungen
nest-turns-every-15 = Alle 15°
nest-running = Verschachteln…
nest-stale = Einstellungen geändert: Vorschau zeigt das Ergebnis
nest-apply = Übernehmen
nest-preview = Vorschau
nest-stopped-unexpectedly = Das Verschachteln hat unerwartet aufgehört
