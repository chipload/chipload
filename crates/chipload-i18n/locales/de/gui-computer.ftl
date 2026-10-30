# "This computer" in the Browser: Preferences and post-processors (crates/chipload-gui/src/app/computer.rs).

computer-posts = Postprozessoren
computer-posts-hover = Was Werkzeugwege in G-Code für Ihre Maschine umsetzt: eingebaute und Ihre eigenen
# Notes at the right end of a post's row in the Browser.
computer-post-built-in = eingebaut
computer-post-yours = eigener
computer-post-this-project = dieses Projekt

computer-preferences = Einstellungen
computer-preferences-about = Für jedes Projekt auf diesem Computer.
computer-settings-folder = Einstellungsordner
# $var is the name of an environment variable.
computer-settings-folder-none = keiner: setzen Sie { $var }, um Einstellungen und eine Bibliothek zu behalten
computer-setup-sheet = Einrichteblatt
computer-setup-sheet-hover = Eine HTML-Seite für den Bediener neben den exportierten Dateien: Rohteil, Nullpunkt, Werkzeuge in Reihenfolge, Bearbeitungen, Zeiten und Warnungen
computer-setup-sheet-write = mit dem G-Code eines schreiben
computer-toolpath = Werkzeugweg
computer-toolpath-auto = automatisch aktualisieren (auch im Menü Ansicht)
computer-toolpath-auto-hover = Eine halbe Sekunde nach jeder Änderung selbst berechnen
computer-units = Einheiten
computer-units-hover = In welchen Einheiten Längen und Vorschübe gezeigt und eingegeben werden. Projekte bleiben in mm; die Einheiten des G-Codes sind die der Maschine (Maschine)
computer-theme = Farbschema
computer-theme-system = System
computer-theme-light = Hell
computer-theme-dark = Dunkel

computer-posts-count = { $builtin } eingebaut, { $yours } eigene.
computer-posts-folder = Eigene gehören in { $folder }
computer-posts-no-folder = Kein Einstellungsordner, daher nur die eingebauten Postprozessoren.
# NAME.toml is a file name: keep it as it is.
computer-posts-about = Ein eigener Postprozessor besteht aus zwei Dateien: NAME.toml mit seinen Einstellungen und die darin genannte Vorlage. Beginnen Sie mit einer Kopie eines eingebauten; die Seite Postprozessoren im Handbuch erklärt beide.
computer-posts-reload = Ordner neu einlesen
computer-posts-reload-hover = Nach dem Hinzufügen oder Ändern der Dateien eines Postprozessors
computer-post = Postprozessor
computer-post-gone = Dieser Postprozessor ist nicht mehr vorhanden.
computer-post-unusable = Nicht verwendbar: { $error }
computer-post-use = In diesem Projekt verwenden
computer-post-use-hover = Zum Postprozessor des Projekts machen
computer-post-use-copy-hover = Zum Postprozessor des Projekts machen: eine Kopie kommt ins Projekt
computer-post-in-use = Das Projekt verwendet ihn
computer-post-no-project = Zuerst ein Projekt öffnen
