# Help > Keyboard shortcuts (crates/chipload-gui/src/app/shortcuts.rs).

shortcuts-window = Tastenkürzel

# The window's group headings: menus (File, Edit, View, Help), the Generate button, and
# what is done on the canvas or while drawing a shape.
shortcuts-group-file = Datei
shortcuts-group-edit = Bearbeiten
shortcuts-group-generate = Berechnen
shortcuts-group-view = Ansicht
shortcuts-group-help = Hilfe
shortcuts-group-canvas = Zeichenfläche
shortcuts-group-drawing = Zeichnen

# What each keyboard shortcut does.
shortcuts-copy-selector = Die Auswahl als Selektor kopieren (JSON)
shortcuts-save-as = Speichern unter
shortcuts-redo = Wiederholen
shortcuts-copy = Die ausgewählten gezeichneten Formen kopieren
shortcuts-paste = Sie einfügen (am Cursor, über der Zeichenfläche)
shortcuts-new = Neues Projekt
shortcuts-open = Ein Projekt öffnen
shortcuts-import = Neues Projekt aus einer DXF oder SVG
shortcuts-save = Speichern
shortcuts-export = G-Code exportieren (sobald berechnet)
shortcuts-quit = Beenden
shortcuts-undo = Rückgängig
shortcuts-move-up = Die Bearbeitung nach oben schieben
shortcuts-move-down = Die Bearbeitung nach unten schieben
shortcuts-generate = Werkzeugweg und G-Code berechnen
shortcuts-fit = Die Ansicht einpassen
shortcuts-top-view = Draufsicht
shortcuts-grid = Das Raster zeigen oder ausblenden
shortcuts-measure = Messen: zwei Klicks ergeben Abstand und Winkel
shortcuts-panels = Die Seitenbereiche ein- oder ausklappen, für eine größere Zeichenfläche
shortcuts-this-list = Diese Liste

# Mouse actions and keys that aren't menu shortcuts: the keys column (key names as on the
# keyboard), then what they do.
shortcuts-click = Klick
shortcuts-click-select = Eine Kontur auswählen
shortcuts-ctrl-click = Ctrl+Klick
shortcuts-ctrl-click-what = Eine Kontur zur Auswahl hinzufügen oder herausnehmen
shortcuts-right-click = Rechtsklick
shortcuts-right-click-what = Eine Kontur oder ihren Layer auswählen; einen verschobenen Ringstart zurücksetzen
shortcuts-scroll = Scrollen
shortcuts-scroll-what = Am Cursor zoomen
shortcuts-drag = Ziehen
shortcuts-drag-what = Mit einem Rahmen auswählen: von links nach rechts, was ganz darin liegt, von rechts nach links, was er berührt; Shift fügt hinzu, Strg fügt hinzu oder entfernt
shortcuts-pan-drag = Ziehen mit rechts oder Mitte, Leertaste+Ziehen
shortcuts-pan-drag-what = Verschieben
shortcuts-delete = Delete
shortcuts-delete-what = Die ausgewählten gezeichneten Formen löschen
shortcuts-escape = Escape
shortcuts-escape-clear = Die Auswahl aufheben
# A ring: a profile's closed path; its start can be dragged on the canvas.
shortcuts-drag-start = Start eines Rings ziehen
shortcuts-drag-start-what = Verschieben, wo der Ring beginnt
shortcuts-drop = Datei ablegen
shortcuts-drop-what = Ein .chipload-Projekt öffnen oder eines aus einer .dxf oder .svg beginnen
shortcuts-click-point = Einen Punkt setzen (fängt an Enden, Mittelpunkten und Mitten)
shortcuts-alt = Alt
shortcuts-alt-what = Halten: nicht fangen
shortcuts-enter = Enter, Doppelklick
shortcuts-enter-what = Polylinie beenden; ohne Werkzeug: mit dem letzten wieder zeichnen
shortcuts-backspace = Backspace
shortcuts-backspace-what = Den letzten Punkt zurücknehmen
shortcuts-escape-stop = Zeichnen beenden
shortcuts-drag-handle = Einen Griff ziehen
shortcuts-drag-handle-what = Einen Punkt, den Mittelpunkt oder eine Ecke verschieben; ein Rechteck drehen
shortcuts-drag-middle = Eine Linie einer Polylinie ziehen
shortcuts-drag-middle-what = Dieses Segment verschieben
shortcuts-ctrl-drag-middle = Strg+Ziehen an einer Linie einer Polylinie
shortcuts-ctrl-drag-middle-what = Dort einen Punkt einfügen
shortcuts-shift-drag-middle = Shift+Ziehen an einer Linie einer Polylinie
shortcuts-shift-drag-middle-what = Das Segment zu einem Bogen biegen
shortcuts-draw-polyline = Polylinie zeichnen
shortcuts-draw-circle = Kreis zeichnen
shortcuts-draw-rectangle = Rechteck zeichnen
shortcuts-type-value = Zahl eintippen
shortcuts-type-value-what = Ein genauer Punkt: x,y  @dx,dy  Länge  Länge<Winkel
shortcuts-shift = Umschalt
shortcuts-shift-what = Halten: die Linie in 15°-Schritten
shortcuts-drag-shape = Eine gezeichnete Form ziehen
shortcuts-drag-shape-what = Sie verschieben, oder die ausgewählten Formen mit ihr; sie rastet ein, Shift hält 15°-Schritte
shortcuts-orbit-drag = Ziehen in der 3D-Ansicht
shortcuts-orbit-drag-what = Umkreisen; Shift+Ziehen verschiebt
shortcuts-draw-arc = Bogen zeichnen
shortcuts-draw-slot = Langloch zeichnen
shortcuts-draw-polygon = Vieleck zeichnen
shortcuts-offset = Die ausgewählten Formen versetzen
shortcuts-trim = Gezeichnete Formen stutzen
shortcuts-construction = Hilfsgeometrie: für die nächsten gezeichneten Formen oder die Auswahl
