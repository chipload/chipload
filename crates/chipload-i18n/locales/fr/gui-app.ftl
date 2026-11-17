# The main window: opening, importing, saving and exporting, parts, rest passes, the
# selection (crates/chipload-gui/src/app/mod.rs).

# The heading of the right-hand panel.
app-properties = Propriétés

app-start-hint = Importez un DXF ou SVG (Ctrl+I) pour démarrer un projet, ou ouvrez-en un (Ctrl+O).

app-cannot-open = Impossible d'ouvrir { $file } :
app-project = Projet : { $file }
app-old-schema = Ce fichier est au schéma version { $version } ; l'enregistrement écrit la version { $current }.

app-new-project = Nouveau projet.
# $what: a machine or tool from the user's library, e.g. machine "Router".
app-started-with = Démarré avec { $what }.
app-new-project-next = Dessinez des formes depuis le menu Dessiner, ou ajoutez un dessin avec Ajouter un DXF ou SVG. Vérifiez le panneau sous le montage, la machine sous Machine, et l'outil dans Outils.
app-units-missing = { $file } n'indique pas ses unités ; choisissez mm ou pouce.
app-px-missing = { $file } ne donne sa taille qu'en pixels ; choisissez ses pixels par pouce.
app-cannot-import = Impossible d'importer { $file } :
app-new-project-from = Nouveau projet à partir de { $file } ({ $shapes } formes).
app-import-next = Sélectionnez des contours, puis Nouveau contour ou Nouvelle poche. Vérifiez le panneau sous le montage, la machine sous Machine, et l'outil dans Outils.

# Followed by the list of problems.
app-project-incomplete = Le projet n'est pas complet :
app-lost-geometry = Des opérations ont perdu leur géométrie : choisissez-la à nouveau
app-could-not-update-drawing = Impossible de mettre à jour le dessin :
app-could-not-add = Impossible d'ajouter { $file } :
app-could-not-move-drawing = Impossible de déplacer le dessin :

# The distance between a grid's cells; $dy and $gap carry their unit.
app-pitch-set = Pas { $dx } × { $dy } (écart { $gap })
app-pitch-needs-plan = Impossible de régler le pas d'après un écart : la pièce doit d'abord être calculée.
app-instance-needs-plan = Impossible de placer la nouvelle instance : la pièce doit d'abord être calculée.

app-deleted-part = { $count ->
    [0] Pièce { $name } supprimée (Ctrl+Z la rétablit)
    [one] Pièce { $name } et son opération supprimées (Ctrl+Z les rétablit)
   *[other] Pièce { $name } et ses { $count } opérations supprimées (Ctrl+Z les rétablit)
}
app-part-needed = Un projet doit avoir au moins une pièce
app-board-surface-stays = Le surfaçage de toute la planche appartient au montage et y reste en premier

# A rest pass clears what a larger tool left in a pocket.
app-could-not-add-rest-pass = Impossible d'ajouter une passe de reprise : { $error }
app-could-not-add-floor-pass = Impossible d'ajouter une passe de fond : { $error }
app-setup-refused = Impossible de modifier les montages : { $error }
app-dowels-placed = Trous de goupille replacés, à l'écart des usinages des deux faces. Le zéro a suivi la première goupille s'il y était.
app-setup-has-operations = Le montage a des pièces avec des opérations : déplacez-les d'abord vers un autre montage, ou supprimez-les.
app-exported-setup = Montage « { $name } » seulement : rendez un autre montage actif et exportez-le aussi.
app-floor-no-endmill = Aucune fraise droite dans le projet ni dans votre bibliothèque d'outils
app-rest-with-corners = Cette poche a des coins dogbone ou T-bone, et les passes de reprise ne peuvent pas encore s'y combiner. Les coins atteignent déjà ce qu'une passe de reprise atteindrait.
# $tool: a tool number such as T3.
app-rest-tool-missing = L'outil { $tool } n'est pas dans le projet : donnez d'abord un outil à la poche.
app-rest-no-smaller = Aucune fraise plus petite que { $tool } ({ $diameter }) dans le projet ni dans votre bibliothèque d'outils. Ajoutez-en une dans Outils, puis refaites un clic droit sur la poche.
app-now-rest-pass = L'opération { $op } est maintenant une passe de reprise de l'opération { $parent } ; elle est usinée après la suite de poches dont elle fait partie

app-generate-before-export = Calculez (F5) avant d'exporter le G-code.
app-planning-then-export = Calcul de chaque pièce, puis export
app-planning-part-then-export = Planification de la pièce { $name }, puis export
app-could-not-export = Impossible d'exporter { $path } :
app-dxf-exported = { $path } écrit.
app-could-not-export-dxf = Le dessin n'a pas été exporté :
app-sheet-failed = G-code exporté, mais impossible d'écrire la fiche de réglage { $path } :
app-exported-n-files = { $count } fichiers exportés : changez d'outil et refaites le zéro Z entre eux, en gardant les zéros X et Y.
app-sheet-for-operator = Fiche de réglage pour l'opérateur : { $path }
app-exported-files = { $count } fichiers exportés à côté de { $path }
app-exported-files-sheet = { $count } fichiers et la fiche de réglage { $sheet } exportés à côté de { $path }
app-exported = { $path } exporté
app-exported-sheet = { $path } et la fiche de réglage { $sheet } exportés

app-cannot-save-yet = Le projet ne peut pas encore être enregistré :
app-could-not-save = Impossible d'enregistrer { $path } :
app-saved = { $path } enregistré

# What is selected on the canvas.
app-nothing-selected = Rien de sélectionné.
# $count contours selected; $what says how (one of the texts below); $list their numbers.
app-selected = { $count } sélectionnés ({ $what }) : { $list }
app-selection-layer = calque { $layer }
app-selection-dxf = dessin { $id }
app-selection-image = image { $id }
app-selection-model = modèle { $id }
app-selection-text = texte { $id }
# $source: one of the three texts above.
app-selection-left-out = { $source }, { $count } exclus
app-selection-contours = contours
# Open contours (not closed) are not cut.
app-open-not-cut = Ouverts (non usinés) : { $list }

# File menu, for a project started without a drawing.
app-import-dxf-into-project = Importer un DXF ou SVG dans le projet…
app-import-dxf-into-project-path = Importer un DXF ou SVG dans le projet depuis un chemin…
# File menu, a new version of the project's drawing.
app-update-drawing-from-dxf = Mettre à jour le dessin depuis un DXF ou SVG…
app-update-drawing-from-path = Mettre à jour le dessin depuis un chemin…

# The Nest window (docs/specs/nesting.md §7)
nest-menu = Imbriquer…
nest-menu-hover = Imbriquer… : chaque instance des pièces du montage placée sur sa planche aussi serrée que possible, séparée par l’écart ; d’abord un aperçu, dans une boîte de dialogue
nest-again = Imbriquer à nouveau
nest-again-hover = Imbriquer le montage à nouveau avec l’écart, la marge et les rotations de la dernière fois, sans boîte de dialogue
nest-title = Imbriquer : { $setup }
nest-title-one = Imbriquer
nest-board-follows = La planche suit le dessin : l’imbrication lui donne cette taille, son coin à l’origine du dessin.
nest-board = Planche
nest-gap = Écart
nest-gap-hover = La plus petite distance entre ce qu’usinent deux instances : une bande de planche qui garde le panneau rigide et les attaches solides
nest-margin = Marge
nest-margin-hover = La place laissée au bord de la planche, pour les vis ou les serre-joints
nest-part = Pièce
nest-count = Fois
nest-count-hover = Combien de fois la pièce est usinée : des instances sont d’abord ajoutées ou retirées, jamais la pièce elle-même
nest-turns = Rotations
nest-turns-none = Telle quelle
nest-turns-half = Demi-tours
nest-turns-quarter = Quarts de tour
nest-turns-every-15 = Tous les 15°
nest-running = Imbrication…
nest-stale = Réglages modifiés : Aperçu pour voir l’imbrication
nest-apply = Appliquer
nest-preview = Aperçu
nest-stopped-unexpectedly = L’imbrication s’est arrêtée de façon inattendue
