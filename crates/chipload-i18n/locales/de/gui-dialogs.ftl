# File dialogs, typed-path prompts, the About window and the window title
# (crates/chipload-gui/src/app/dialogs.rs).

# Help > User guide, until the guide is published online. Keep the file names as they are.
dialogs-user-guide = Das Benutzerhandbuch liegt jeder Version als chipload-guide-<version>.zip bei (index.html öffnen). Aus dem Quellcode: packaging/docs/build-book.sh.

# Names of file types in the system's file dialogs.
dialogs-filter-projects = Chipload-Projekte
dialogs-filter-dxf = Zeichnungen (DXF, SVG)

dialogs-selector-copied = Selektor in die Zwischenablage kopiert

dialogs-about-title = Über Chipload
dialogs-about-appimage = Das Linux-AppImage enthält außerdem libxkbcommon und libxcb-xkb (MIT-artige Lizenzen); ihre Copyright-Dateien liegen im AppImage unter usr/share/doc/chipload/bundled.
dialogs-third-party = Lizenzen Dritter

# Asked when a DXF doesn't say whether it was drawn in millimetres or inches.
dialogs-units-title = Einheiten der Zeichnung
dialogs-units-unknown = { $file } gibt keine Einheiten an.
dialogs-units-question = In welchen Einheiten wurde sie gezeichnet?
# A button: the drawing was drawn in inches.
dialogs-inch = Zoll
dialogs-units-cancelled = Zeichnung nicht importiert: ihre Einheiten sind unbekannt.
dialogs-px-title = Pixel pro Zoll
dialogs-px-unknown = { $file } gibt seine Größe nur in Pixeln an.
dialogs-px-question = Wie viele Pixel ergeben einen Zoll?
dialogs-px-96 = 96 (Inkscape, Browser)
dialogs-px-72 = 72 (älteres Illustrator)
dialogs-px-other = Andere:
dialogs-px-use = Verwenden
dialogs-px-cancelled = Zeichnung nicht importiert: ihre Größe ist unbekannt.
dialogs-export-dxf-title = Zeichnung als DXF exportieren
dialogs-export-dxf-inch = In Zoll (sonst mm)
dialogs-export-dxf-hidden = Auch ausgeblendete Layer und Gruppen
dialogs-export-dxf-construction = Auch Hilfsgeometrie (auf eigenen Layern)
dialogs-export-dxf-selection = Nur die gewählten Formen
dialogs-export-dxf-save = Exportieren…
dialogs-export-dxf-path = In Pfad exportieren…
dialogs-path-export-dxf = Die zu schreibende DXF:
dialogs-filter-dxf-file = DXF-Zeichnung

dialogs-unsaved-title = Ungespeicherte Änderungen
dialogs-unsaved-body = Das Projekt hat ungespeicherte Änderungen. Zuerst speichern?
dialogs-save-hover = Speichern, dann fortfahren
dialogs-dont-save = Nicht speichern
dialogs-dont-save-hover = Änderungen verwerfen und fortfahren

# Another program saved the open project's file.
dialogs-changed-title = { $file } wurde auf der Festplatte geändert
dialogs-changed-body = Ein anderes Programm hat das Projekt gespeichert. Dessen Version laden? Ihre ungespeicherten Änderungen bleiben mit Ctrl+Z erreichbar.
dialogs-load-it = Laden
dialogs-load-it-hover = Die Version der Datei zeigen; Ctrl+Z holt Ihre zurück
dialogs-keep-mine = Meine behalten
dialogs-keep-mine-hover = Ihre Änderungen behalten; das nächste Speichern überschreibt die Datei

# Windows where a file's path is typed in: a title and what to type.
dialogs-path-open-title = Pfad öffnen
dialogs-path-open = Ein .chipload-Projekt, oder eine .dxf oder .svg:
dialogs-path-save-title = Projekt speichern
dialogs-path-save = Projekt speichern unter:
dialogs-path-export-title = G-Code exportieren
dialogs-path-export = Den geprüften G-Code exportieren nach:
dialogs-path-update-title = Zeichnung aktualisieren
dialogs-path-update = Die DXF oder SVG:
dialogs-path-image-title = Bild importieren
dialogs-path-image = Ein Bild (PNG, JPEG oder BMP):
dialogs-path-add-dxf-title = DXF oder SVG hinzufügen
dialogs-path-add-dxf = Die DXF oder SVG, die zum Projekt hinzukommt:
dialogs-path-update-dxf-title = Zeichnungsdatei aktualisieren
dialogs-path-update-dxf = Die neue Version der DXF oder SVG:

# In the window's title bar, for a project never saved.
dialogs-title-new-project = neues Projekt

## Explode a DXF, join close ends (docs/specs/explode-and-join.md §5)

# $name is the DXF's name in the Browser.
dialogs-explode-title = { $name } in Formen auflösen
dialogs-explode-body = Ihre Formen werden zu gezeichneten Formen, die Sie bearbeiten, verschieben und löschen können. Die Bearbeitungen fräsen weiter dieselben Formen. Das Projekt kennt die DXF-Datei nicht mehr: Aktualisieren aus der DXF ist dann nicht möglich.
dialogs-explode-image-body = Seine Formen werden zu gezeichneten Formen, die Sie bearbeiten, verschieben und löschen können. Die Bearbeitungen fräsen weiter dieselben Formen. Das Bild selbst verschwindet aus dem Projekt: Es kann nicht erneut nachgezeichnet werden.
dialogs-explode-text-body = Seine Buchstaben werden zu gezeichneten Formen, die Sie bearbeiten, verschieben und löschen können. Die Bearbeitungen fräsen weiter dieselben Formen. Die Wörter können nicht mehr bearbeitet werden.
dialogs-explode-join = Enden verbinden, die näher liegen als
dialogs-explode = Auflösen
dialogs-join-title = Enden verbinden
dialogs-join-body = Offene Polylinien unter { $count ->
    [one] der ausgewählten Form
   *[other] den { $count } ausgewählten Formen
}, deren Enden näher als der Abstand liegen, werden zu einer, auf demselben Layer. Eine Kette, die zu ihrem Anfang zurückkehrt, wird geschlossen.
dialogs-join-body-all = Offene gezeichnete Polylinien, deren Enden näher als der Abstand liegen, werden zu einer, auf demselben Layer. Eine Kette, die zu ihrem Anfang zurückkehrt, wird geschlossen.
dialogs-join-gap = Enden verbinden, die näher liegen als
dialogs-join = Verbinden

# V-carve inlays (docs/specs/vcarve-inlay.md §4)
inlay-ribbon = Intarsie…
inlay-ribbon-hover = Eine V-Carving-Intarsie der gewählten Formen: hier eine Tasche, und ein gespiegelter Stopfen in einem eigenen Brett, der hineingeleimt wird
inlay-menu = Neue Intarsie…
inlay-title = V-Carving-Intarsie
inlay-body = Die gewählten Formen werden als Tasche in dieses Brett graviert und als Stopfen, gespiegelt, in einer neuen Aufspannung: einem eigenen Brett. Der Stopfen wird mit der Stirnseite nach unten in die Tasche geleimt und seine Rückseite abgehobelt.
inlay-vbit = V-Fräser
inlay-none = Keiner
inlay-depth = Taschentiefe
inlay-depth-hover = Wie tief die Tasche ins Grundbrett gefräst wird
inlay-plug-start = Stopfen-Starttiefe
inlay-plug-start-hover = Wie tief in seinem Brett der Umriss des Stopfens liegt: so tief sitzt seine Stirnseite in der Tasche, der Rest der Taschentiefe bleibt für den Leim
inlay-gap = Leimspalt
inlay-gap-hover = Wie viel tiefer das Stopfenbrett gefräst wird, damit seine Rückseite so weit über dem Grundbrett steht und ausgequetschter Leim Platz hat
inlay-floor = Schaftfräser für den Boden
inlay-floor-hover = Ein Schaftfräser für die Bodendurchgänge beider. Der Kanal des Stopfens ist größtenteils Boden
inlay-thickness = Dicke des Stopfenbretts
inlay-thickness-hover = Das Brett, aus dem der Stopfen gefräst wird. Was unter seinem Kanal bleibt, hält ihn zusammen, bis es abgehobelt wird
inlay-make = Intarsie anlegen
inlay-no-vbit = Das Projekt hat keinen V-Fräser: füge zuerst einen zu seinen Werkzeugen hinzu
inlay-made = Intarsie angelegt: Operation { $pocket } ist die Tasche, Operation { $plug } der Stopfen, in Aufspannung { $setup }, sein Teil von links nach rechts gespiegelt.
inlay-refused = Die Intarsie konnte nicht angelegt werden.
sign-title = Neues Schild
sign-body = Die Wörter, ein Rahmen um sie, der ihnen folgt, und jede Operation, die das Schild schneidet, in einem eigenen Teil. Alles lässt sich danach ändern.
sign-words = Wörter
sign-words-hint = Was auf dem Schild steht; Enter beginnt eine neue Zeile
sign-around = Um die Auswahl
sign-around-body = { $count ->
    [one] Das Schild geht um die ausgewählte Form, die mit dem V-Fräser geschnitzt wird. Der Rahmen folgt ihr, auch einem neu nachgezeichneten Bild.
   *[other] Das Schild geht um die { $count } ausgewählten Formen, die mit dem V-Fräser geschnitzt werden. Der Rahmen folgt ihnen, auch einem neu nachgezeichneten Bild.
}
sign-around-words = Stattdessen Wörter schreiben
sign-around-words-hover = Das Schild um neue Wörter machen, wie ohne Auswahl
sign-font = Schrift
sign-height = Höhe
sign-style = Stil
sign-style-hover = Graviert: die Buchstaben V-geschnitzt. Erhaben: der Hintergrund innerhalb des innersten Randes um die Buchstaben abgesenkt. Intarsie: die Buchstaben aus einem zweiten Brett eingelegt.
sign-style-engraved = Graviert
sign-style-raised = Erhaben
sign-style-inlay = Intarsie
sign-relief = Tiefe des Hintergrunds
sign-relief-hover = Wie tief der Hintergrund um die erhabenen Buchstaben mit dem Schaftfräser abgesenkt wird
sign-edge = Kante
sign-edge-chamfer = Fase
sign-edge-hover = Eine so breite Fase um die Außenlinie, mit dem V-Fräser geschnitten, bevor das Schild ausgeschnitten wird
sign-vbit = V-Fräser
sign-end-mill = Schaftfräser
sign-board = Brett
sign-fit-board = Um das Schild anpassen
sign-fit-board-hover = Das Rohteil um die Zeichnung mit diesem Rand und dieser Dicke. Aus: das Rohteil bleibt, wie es ist.
sign-thickness = Dicke
sign-board-margin = Rand
sign-adds = Fügt hinzu: { $operations }
sign-make = Schild erstellen
sign-no-tools = Wählen Sie einen V-Fräser und einen Schaftfräser (zuerst zu den Werkzeugen des Projekts hinzufügen).
sign-made = Schild erstellt: { $count } Operationen in einem eigenen Teil. Ändern Sie die Wörter, und der Rahmen folgt.
sign-refused = Das Schild konnte nicht erstellt werden.
inlay-mark-gap = Rückseite, { $gap } darüber
inlay-mark-surface = Oberfläche
inlay-mark-face = Stirnseite, { $depth }
inlay-mark-pocket = Tasche, { $depth }
