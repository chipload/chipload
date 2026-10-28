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
dialogs-path-open = Ein .chipload-Projekt oder eine .dxf:
dialogs-path-save-title = Projekt speichern
dialogs-path-save = Projekt speichern unter:
dialogs-path-export-title = G-Code exportieren
dialogs-path-export = Den geprüften G-Code exportieren nach:
dialogs-path-update-title = Zeichnung aktualisieren
dialogs-path-update = Die DXF:
dialogs-path-image-title = Bild importieren
dialogs-path-image = Ein Bild (PNG, JPEG oder BMP):
dialogs-path-add-dxf-title = DXF hinzufügen
dialogs-path-add-dxf = Die DXF oder SVG, die zum Projekt hinzukommt:
dialogs-path-update-dxf-title = DXF aktualisieren
dialogs-path-update-dxf = Die neue Version der DXF oder SVG:

# In the window's title bar, for a project never saved.
dialogs-title-new-project = neues Projekt

## Explode a DXF, join close ends (docs/specs/explode-and-join.md §5)

# $name is the DXF's name in the Browser.
dialogs-explode-title = { $name } in Formen auflösen
dialogs-explode-body = Ihre Formen werden zu gezeichneten Formen, die Sie bearbeiten, verschieben und löschen können. Die Bearbeitungen fräsen weiter dieselben Formen. Das Projekt kennt die DXF-Datei nicht mehr: Aktualisieren aus der DXF ist dann nicht möglich.
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
