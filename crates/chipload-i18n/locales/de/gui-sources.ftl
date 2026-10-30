# The Browser's Drawing folder: the project's DXFs with their layers, its pictures, texts
# and drawn shapes (crates/chipload-gui/src/app/sources.rs).

# The folder's name: everything that is drawn, a noun.
sources-drawing = Zeichnung
sources-drawing-hover = Zeichnungsdateien, Bilder, Texte und gezeichnete Formen; Rechtsklick zum Hinzufügen
sources-add-dxf = DXF oder SVG hinzufügen…
sources-add-dxf-hover = Ein weiteres Teil: eine DXF oder SVG, eigenständig platziert und aktualisiert
# "File > Import DXF" names a menu item.
sources-add-dxf-disabled-hover = Datei > DXF oder SVG importieren beginnt damit ein Projekt
sources-import-image = Bild importieren…
sources-import-image-hover = Ein Bild in Formen nachzeichnen, für V-Gravur
sources-add-text = Text hinzufügen
sources-add-text-hover = Einen Text schreiben, dann klicken, wo er beginnt
sources-no-shapes = Noch keine Formen.
# The name of drawing files (DXF, SVG) in the file dialog's filter.
sources-dxf-filter = Zeichnungen (DXF, SVG)

# Rows of the folder. $file: the picture's file name; $text: what the text says.
sources-image-title = { $file } (Bild { $id })
sources-text-title = { $text } (Text { $id })
# Rows for the shapes drawn in the app, by kind.
sources-rectangles = Rechtecke
sources-circles = Kreise
sources-polylines = Polylinien
sources-other-paths = Andere Pfade

sources-dxf-hover = Klick wählt die ganze Datei: eine daraus erstellte Bearbeitung folgt ihren Aktualisierungen. Position und Maßstab unter Eigenschaften; Rechtsklick für mehr
sources-image-hover = Klick wählt das ganze Bild: eine daraus erstellte Bearbeitung folgt dem erneuten Nachzeichnen. Größe und Lage unter Eigenschaften; Doppelklick für die Einstellungen zum Nachzeichnen
sources-text-hover = Klick wählt den ganzen Text: eine daraus erstellte Bearbeitung folgt seinen Änderungen. Wortlaut, Schrift, Größe und Lage stehen unter Eigenschaften
sources-drawn-hover = Klick wählt diese Formen; Ctrl+Klick fügt sie hinzu
sources-layer-hover = Klick wählt diesen Layer dieser Datei: eine daraus erstellte Bearbeitung fräst nur diesen Layer dieser Datei

# Right-click menu items, verbs.
sources-select = Auswählen
sources-show = Einblenden
sources-hide = Ausblenden
sources-place = Platzieren…
sources-place-hover = Diese Zeichnungsdatei verschieben, drehen oder skalieren, in ihren Eigenschaften
sources-update-from-dxf = Aus einer DXF oder SVG aktualisieren…
sources-update-from-path = Aus einem Pfad aktualisieren…
sources-remove-dxf-hover = Diese Zeichnungsdatei und ihre Formen entfernen
sources-trace-settings = Nachzeichnen…
sources-trace-settings-hover = Schwellwert, Glättung, Umrisse oder Mittellinien; Größe und Lage stehen in seinen Eigenschaften
sources-remove-image-hover = Das Bild und seine Formen entfernen
sources-remove-text-hover = Den Text und seine Buchstaben entfernen

# Followed by the project's problems, one per line.
sources-project-incomplete = Das Projekt ist nicht vollständig:
# Followed by the error on its own line.
sources-remove-dxf-failed = Die Zeichnungsdatei konnte nicht entfernt werden:
sources-explode = In Formen auflösen…
sources-explode-hover = Die Formen dieser Datei in gezeichnete Formen umwandeln, die Sie bearbeiten können; die Verknüpfung zur Datei geht verloren
sources-explode-failed = Die Zeichnungsdatei konnte nicht aufgelöst werden:
sources-join-failed = Die Formen konnten nicht verbunden werden:
sources-drawn-layer-hover = Klicken wählt diese Formen aus; das Auge blendet sie aus
sources-shape-hover = Klicken wählt sie aus; Strg+Klick fügt hinzu oder entfernt; Rechtsklick zum Umbenennen, Kopieren oder Löschen
sources-rename = Umbenennen…
sources-copy-shape = Kopieren
sources-delete-shape = Löschen
sources-polygons = Vielecke
