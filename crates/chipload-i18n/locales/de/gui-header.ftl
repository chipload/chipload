# The header: the top bar and the ribbon of grouped buttons (crates/chipload-gui/src/app/header.rs).

# Export's hover text. $count is the number of parts that are cut (always more than one),
# $part the name of the part the preview shows.
header-export-hover = Den geprüften G-Code speichern
header-export-hover-parts = Den geprüften G-Code aller gefrästen Teile speichern ({ $count } Teile)
header-export-hover-parts-shown = Den geprüften G-Code aller gefrästen Teile speichern ({ $count } Teile); die Vorschau zeigt nur „{ $part }“
header-export-generating-hover = Den G-Code speichern, sobald er berechnet ist
header-export-first-hover = Zuerst berechnen

# The top bar's icon buttons.
header-open-hover = Ein Projekt öffnen
header-save-hover = Das Projekt speichern
header-undo = Rückgängig
header-redo = Wiederholen
header-help = Hilfe
# The colour theme button: what is set now, a click goes on to the next.
header-theme-system-hover = Farbschema: wie im System eingestellt. Klicken für das nächste.
header-theme-light-hover = Farbschema: hell. Klicken für das nächste.
header-theme-dark-hover = Farbschema: dunkel. Klicken für das nächste.

# The tab with the open project's file name.
header-new-project = Neues Projekt
header-tab-unsaved-hover =
    { $path }
    Ungespeicherte Änderungen
header-tab-not-saved-hover = Noch nicht gespeichert

# The ribbon's groups: their captions (shown in capitals) open the menu of the same name.
header-group-file = Datei
header-group-draw = Zeichnen
header-group-operation = Bearbeitung
header-group-library = Bibliothek
header-group-view = Ansicht
header-group-hover = Alles unter { $group }

# FILE: short words under icons.
header-new = Neu
header-new-hover = Neues Projekt
header-open = Öffnen
header-import = Import
header-import-hover = Neues Projekt aus einer DXF
header-save = Speichern

# DRAW: short words under icons, and what each does.
header-circle = Kreis
header-circle-hover = Einen Kreis zeichnen: Mittelpunkt, dann ein Punkt darauf
header-rectangle = Rechteck
header-rectangle-hover = Ein Rechteck zeichnen: zwei gegenüberliegende Ecken
header-polyline = Polylinie
header-polyline-hover = Eine Polylinie zeichnen: Punkte anklicken
header-text = Text
header-text-hover = Einen Text schreiben: klicken, wo er beginnt, dann unter Eigenschaften eintippen
header-image = Bild
header-image-hover = Ein Bild in Formen nachzeichnen, für V-Gravur oder Pfad folgen
header-add-dxf = DXF hinzu
header-add-dxf-hover = Ein weiteres Teil: eine DXF, eigenständig platziert und aktualisiert
# "FILE > Import": the ribbon group FILE and its Import button.
header-add-dxf-first-hover = Zuerst eine DXF importieren (DATEI > Import)
header-import-image-menu = Bild importieren…
header-add-dxf-menu = DXF zum Projekt hinzufügen…

# OPERATION: "Follow" is the short word under the icon for a Follow path operation.
header-follow = Pfad
# $operation is what the operation does (a hover text of its own).
header-needs-selection-hover = { $operation }. Zuerst Formen auswählen.
header-select-shapes-first = Zuerst Formen auswählen
header-new-profile = Neu: Konturfräsen
header-new-pocket = Neue Tasche
header-new-vcarve = Neue V-Gravur
header-new-follow-path = Neu: Pfad folgen
header-new-holes = Neue Bohrungen
header-new-surface = Neu: Planfräsen

# LIBRARY
header-tools = Werkzeuge
header-tools-hover = Ihre Werkzeugbibliothek, unter Dieser Computer im Browser
header-machines = Maschinen
header-machines-hover = Ihre Maschinenprofile, unter Dieser Computer im Browser
header-tool-library = Werkzeugbibliothek

# VIEW
header-measure = Messen
header-measure-hover = Zwei Klicks ergeben Abstand, ΔX, ΔY und Winkel. Was gezeigt wird, steht in der Leiste der Zeichenfläche.

# Generate and Export at the ribbon's right end.
header-export = Export
header-cancel = Abbrechen
header-cancel-hover = Berechnung stoppen
header-generate = Berechnen
header-generate-hover = Werkzeugwege planen, G-Code schreiben und prüfen
header-arc = Bogen
header-arc-hover = Einen Bogen zeichnen: Anfang, Ende, dann ein Punkt, durch den er geht
header-slot = Langloch
header-slot-hover = Ein Langloch zeichnen: die Mittelpunkte der Enden, dann die Breite
header-polygon = Vieleck
header-polygon-hover = Ein regelmäßiges Vieleck zeichnen: Mittelpunkt, dann eine Ecke (die Seitenzahl in der Statusleiste)
header-group-modify = Ändern
header-offset = Versatz
header-offset-hover = Eine Form im Abstand zu den ausgewählten: auf die Seite fahren, klicken oder den Abstand eingeben
header-fillet = Abrunden
header-fillet-hover = Eine Ecke der ausgewählten Form mit einem Bogen runden: die Ecke klicken; Shift+Klick rundet alle Ecken
header-chamfer = Fase
header-chamfer-hover = Eine Ecke der ausgewählten Form gerade abschneiden: die Ecke klicken; Shift+Klick schneidet alle Ecken
header-trim = Stutzen
header-trim-hover = Das Stück einer gezeichneten Form unter dem Cursor entfernen, bis dahin, wo andere Formen sie kreuzen: das Stück anklicken
header-extend = Verlängern
header-extend-hover = Das gerade Ende einer offenen gezeichneten Form bis zur nächsten Form verlängern: nahe dem Ende klicken
header-break = Teilen
header-break-hover = Eine gezeichnete Form am geklickten Punkt teilen: eine offene in zwei, eine geschlossene öffnet sich dort
header-union = Vereinigen
header-union-hover = Die gewählten geschlossenen Formen zu einer verbinden: alles, was eine von ihnen bedeckt
header-subtract = Abziehen
header-subtract-hover = Die gewählten geschlossenen Formen aus der Form ausschneiden, die Sie danach klicken
header-intersect = Schneiden
header-intersect-hover = Behalten, was alle gewählten geschlossenen Formen bedecken
