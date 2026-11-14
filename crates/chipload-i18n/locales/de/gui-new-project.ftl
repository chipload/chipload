# The new project dialog (crates/chipload-gui/src/app/new_project.rs, docs/specs/new-project-dialog.md).

new-project-title = Neues Projekt
new-project-material = Material
new-project-material-none = Keins
new-project-thickness = Dicke
# The board: fitted round the drawing with a margin, or a size.
new-project-board = Platte
new-project-fit = An die Zeichnung anpassen, Rand
new-project-size = Größe
new-project-z-zero = Z-Nullpunkt
new-project-z-zero-hover = Wo der Z0 der Maschine gesetzt wird: auf der Oberseite der Platte oder auf der Opferplatte darunter
new-project-z-top = Oberseite der Platte
new-project-z-spoilboard = Opferplatte
new-project-machine = Maschine
new-project-machine-none = Chiploads eigene Einstellungen (keine Maschine in der Bibliothek)
# Under the machine: its post-processor, G-code units (mm or inch) and how tools are changed.
new-project-machine-summary = { $post } · { $units } · { $change }
new-project-tools = Werkzeuge
new-project-search = Suchen
# Under the tool list: how many are ticked and their numbers ("T1, T2").
new-project-tools-count = { $count ->
    [0] Keine Werkzeuge: vor dem Erzeugen eines hinzufügen
    [one] 1 Werkzeug: { $list }
   *[other] { $count } Werkzeuge: { $list }
}
new-project-no-library-tools = Die Werkzeugbibliothek ist leer: das Projekt beginnt mit einem 6-mm-Schaftfräser. Werkzeuge unter Dieser Computer > Werkzeuge hinzufügen.
new-project-create = Erstellen
