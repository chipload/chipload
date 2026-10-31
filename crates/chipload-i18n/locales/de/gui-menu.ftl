# The menu bar and its menus (crates/chipload-gui/src/app/menu.rs).

menu-language = Sprache
menu-language-system = System
menu-language-system-hover = Die Sprache des Betriebssystems, wenn Chipload sie hat, sonst Englisch
menu-language-review = { $language } (Prüfung erwünscht)
menu-language-review-hover = Übersetzt, aber noch nicht von einem Muttersprachler aus der Zerspanung gelesen. Korrekturen sind willkommen: siehe Benutzerhandbuch, Sprachen.

# File menu.
menu-new-project = Neues Projekt
menu-open = Öffnen…
menu-open-recent = Zuletzt geöffnet
# Empties the list of recent projects.
menu-clear-list = Liste leeren
menu-import-dxf = DXF oder SVG importieren…
menu-import-image = Bild importieren…
menu-import-image-hover = Ein Bild in Formen nachzeichnen, für V-Gravur
menu-open-path = Pfad öffnen…
menu-import-image-path = Bild aus Pfad importieren…
menu-several-dxfs = Mehrere Zeichnungsdateien: eine im Browser aktualisieren (Rechtsklick)
menu-add-dxf = DXF oder SVG zum Projekt hinzufügen…
menu-add-dxf-hover = Eine DXF oder SVG in dieses Projekt, neben das Vorhandene: eigenständig platziert und aktualisiert
menu-add-dxf-path = DXF oder SVG aus Pfad hinzufügen…
menu-save = Speichern
menu-save-as = Speichern unter…
menu-save-path = In Pfad speichern…
menu-export = G-Code exportieren…
menu-export-path = G-Code in Pfad exportieren…
menu-export-dxf = Zeichnung als DXF exportieren…
menu-export-dxf-hover = Die Formen, die die Zeichenfläche zeigt, auf ihren Layern, für einen Laser, ein CAD-Programm oder ein Angebot
menu-construction = Hilfsgeometrie
menu-construction-hover = Hilfsgeometrie: gepunktet gezeigt und zum Fangen da, aber nie gefräst. Für Hilfslinien und -kreise beim Zeichnen
menu-toggle-construction = Hilfsgeometrie ein oder aus
menu-quit = Beenden

# Edit: undo, and the selected drawn shapes.
menu-undo = Rückgängig
menu-redo = Wiederholen
menu-only-drawn = Nur gezeichnete Formen; importierte ändern sich nur mit ihrer Datei
menu-copy = Kopieren
menu-paste = Einfügen
menu-mirror-left-right = Links–rechts spiegeln
menu-mirror-up-down = Oben–unten spiegeln
menu-mirror-hover = Die ausgewählten gezeichneten Formen um die Mitte ihres Rahmens spiegeln
menu-delete-shapes = Formen löschen
menu-delete-shapes-hover = Löscht die ausgewählten gezeichneten Formen; importierte ändern sich nur mit ihrer Datei
# A selector: the selection written as JSON, for scripts.
menu-copy-selector = Als Selektor kopieren

# Job menu.
menu-cancel-generation = Berechnung abbrechen
menu-generate = Berechnen

# View menu.
menu-fit-view = Ansicht einpassen
menu-top-view = Draufsicht
menu-measure = Messen
menu-measure-hover = Zwei Klicks ergeben Abstand, ΔX, ΔY und Winkel
menu-3d-view = 3D-Ansicht
menu-z-scale = Z-Maßstab in 3D
menu-show-grid = Raster zeigen
menu-show-grid-hover = Linien auf dem Fangraster, jede zehnte kräftiger
menu-show-rulers = Lineale zeigen
menu-show-rulers-hover = Lineale am oberen und linken Rand der Zeichenfläche in der Draufsicht: Zeichnungskoordinaten, der Mauszeiger markiert
menu-show-legend = Legende zeigen
menu-show-browser = Browser zeigen
menu-show-properties = Eigenschaften zeigen
menu-fold-panels = Beide Bereiche einklappen
menu-open-panels = Beide Bereiche ausklappen
menu-panels-hover = Größte Zeichenfläche: Browser und Eigenschaften zu Leisten eingeklappt
menu-auto-update = Werkzeugweg automatisch aktualisieren
menu-auto-update-hover = Eine halbe Sekunde nach jeder Änderung selbst berechnen
menu-show-toolpath = Werkzeugweg zeigen
menu-show-toolpath-hover = Werkzeugweg ausblenden, um die Zeichnung oder das simulierte Rohteil zu sehen
menu-show-drawing = Zeichnung zeigen
menu-show-drawing-hover = Zeichnung ausblenden, um nur das Rohteil oder den Werkzeugweg zu sehen
menu-show-images = Bilder zeigen
menu-show-images-hover = Die in Formen nachgezeichneten Bilder, blass über dem Rohteil
# The stock: the material being cut, as the simulation shows it.
menu-simulate = Simulieren
menu-simulation-hint = Das Rohteil zeigen, wie das Programm es hinterlässt: jede Bewegung mit der Form des Werkzeugs gefräst (dunkler ist tiefer; rot ist unterhalb des Rohteils). Nur Anzeige.
menu-units = Einheiten
menu-units-mm-hover = Längen in mm, Vorschübe in mm/min
# The unit's name, as a choice next to "mm".
menu-units-inch = Zoll
menu-units-inch-hover = Längen in Zoll, Vorschübe in in/min; Projekte bleiben in mm
menu-theme = Farbschema
menu-theme-system = System
menu-theme-system-hover = Hell oder dunkel, wie im Betriebssystem eingestellt
menu-theme-light = Hell
menu-theme-light-hover = Helle Bereiche und Zeichenfläche
menu-theme-dark = Dunkel
menu-theme-dark-hover = Dunkle Bereiche und Zeichenfläche

# Draw menu.
menu-draw-circle = Kreis
menu-draw-rectangle = Rechteck
menu-draw-polyline = Polylinie
menu-draw-text = Text
menu-draw-text-hover = Einen Text schreiben, dann klicken, wo er beginnt
menu-draw-frame = Rahmen…
menu-draw-frame-hover = Ein Rahmen für ein Schild: um die Auswahl oder in einer Größe, mit Randlinien

# Help menu.
menu-user-guide = Benutzerhandbuch
menu-shortcuts = Tastenkürzel
menu-about = Über und Lizenzen

# The status bar under the canvas. What the toolpath is of, with several parts:
menu-status-part-of = Teil { $part } von { $count }
menu-status-all-parts = alle Teile
# $time: how long the program runs, e.g. "12 min 30 s".
menu-status-cycle = Zeit { $time }
menu-status-time-hover = Geschätzte Bearbeitungszeit (Details im Bericht)
menu-status-active-part = aktives Teil: { $name }
menu-status-active-part-hover = Neue Bearbeitungen kommen in das aktive Teil (Browser, Rechtsklick auf ein Teil)
menu-status-units-hover = Längen werden in diesen Einheiten gezeigt (Ansicht > Einheiten)
# $step: the grid's step with its unit, e.g. "10 mm".
menu-status-grid = Raster { $step }
menu-status-grid-hover = Die Rasterweite: Klicks beim Zeichnen rasten darauf ein (G blendet es aus)
menu-status-post-hover = Der Postprozessor (Maschine)
# Pieces in all, from every part that is cut.
menu-status-pieces = { $count ->
    [one] { $count } Stück
   *[other] { $count } Stück
}
menu-status-copies = { $count ->
    [one] { $count } Kopie
   *[other] { $count } Kopien
}
# The cursor in program coordinates (from the work zero), after the drawing's X and Y.
menu-status-work = (Werkstück X { $x }  Y { $y })
# $target: what the cursor snapped to, e.g. "end of #4".
menu-snapped-to = — gefangen an { $target }
menu-generating = Berechnung läuft…
# $snapped: empty, or "— snapped to …" with a space before it.
menu-dragging = Ziehen nach ({ $x }, { $y }){ $snapped } (Alt: ohne Fang; an einem Segment fügt Strg einen Punkt ein, Shift biegt es)
menu-status-hint = Kontur anklicken zum Auswählen · Ctrl+Klick fügt hinzu oder entfernt · Rechtsklick wählt ihren Layer · Esc hebt auf
menu-status-simulate-hint = Simulieren: ziehen verschiebt (3D: dreht) · Nummer eines Arbeitsgangs anklicken öffnet ihn · F6 zurück zu Bearbeiten
menu-join-ends = Enden verbinden…
menu-join-ends-hover = Offene gezeichnete Polylinien, deren Enden sich fast treffen, zu einer verbinden: die ausgewählten, oder alle, wenn keine ausgewählt ist
menu-join-ends-disabled-hover = Es gibt keine gezeichneten Formen. Die Formen einer DXF lassen sich verbinden, sobald sie aufgelöst ist (Rechtsklick auf sie im Browser).
menu-draw-arc = Bogen
menu-draw-slot = Langloch
menu-draw-polygon = Vieleck
menu-polygon-sides = Seiten
menu-modify-offset = Versatz
menu-modify-fillet = Abrunden
menu-modify-chamfer = Fase
menu-modify-trim = Stutzen
menu-modify-extend = Verlängern
menu-modify-break = Teilen
menu-modify-union = Vereinigen
menu-modify-subtract = Abziehen
menu-modify-intersect = Schneiden
menu-modify-array = Anordnung
menu-modify-polar-array = Kreisanordnung
