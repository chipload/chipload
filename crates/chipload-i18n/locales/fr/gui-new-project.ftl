# The new project dialog (crates/chipload-gui/src/app/new_project.rs, docs/specs/new-project-dialog.md).

new-project-title = Nouveau projet
new-project-material = Matériau
new-project-material-none = Aucun
new-project-thickness = Épaisseur
# The board: fitted round the drawing with a margin, or a size.
new-project-board = Planche
new-project-fit = Ajustée au dessin, marge
new-project-size = Taille
new-project-z-zero = Zéro Z
new-project-z-zero-hover = Où le Z0 de la machine est pris : sur le dessus de la planche, ou sur le martyr dessous
new-project-z-top = Dessus de la planche
new-project-z-spoilboard = Martyr
new-project-machine = Machine
new-project-machine-none = Réglages de Chipload (aucune machine dans votre bibliothèque)
# Under the machine: its post-processor, G-code units (mm or inch) and how tools are changed.
new-project-machine-summary = { $post } · { $units } · { $change }
new-project-tools = Outils
new-project-search = Rechercher
# Under the tool list: how many are ticked and their numbers ("T1, T2").
new-project-tools-count = { $count ->
    [0] Aucun outil : ajoutez-en un avant de générer
    [one] 1 outil : { $list }
   *[other] { $count } outils : { $list }
}
new-project-no-library-tools = Votre bibliothèque d'outils est vide : le projet commence avec une fraise de 6 mm. Ajoutez vos outils sous Bibliothèque > Outils.
new-project-create = Créer
