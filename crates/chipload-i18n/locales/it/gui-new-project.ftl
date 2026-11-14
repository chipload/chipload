# The new project dialog (crates/chipload-gui/src/app/new_project.rs, docs/specs/new-project-dialog.md).

new-project-title = Nuovo progetto
new-project-material = Materiale
new-project-material-none = Nessuno
new-project-thickness = Spessore
# The board: fitted round the drawing with a margin, or a size.
new-project-board = Pannello
new-project-fit = Adattato al disegno, margine
new-project-size = Dimensioni
new-project-z-zero = Zero Z
new-project-z-zero-hover = Dove si imposta lo Z0 della macchina: sul lato superiore del pannello o sul piano di sacrificio sotto
new-project-z-top = Lato superiore del pannello
new-project-z-spoilboard = Piano di sacrificio
new-project-machine = Macchina
new-project-machine-none = Impostazioni di Chipload (nessuna macchina nella libreria)
# Under the machine: its post-processor, G-code units (mm or inch) and how tools are changed.
new-project-machine-summary = { $post } · { $units } · { $change }
new-project-tools = Utensili
new-project-search = Cerca
# Under the tool list: how many are ticked and their numbers ("T1, T2").
new-project-tools-count = { $count ->
    [0] Nessun utensile: aggiungine uno prima di generare
    [one] 1 utensile: { $list }
   *[other] { $count } utensili: { $list }
}
new-project-no-library-tools = La libreria utensili è vuota: il progetto parte con una fresa da 6 mm. Aggiungi i tuoi utensili in Libreria > Utensili.
new-project-create = Crea
