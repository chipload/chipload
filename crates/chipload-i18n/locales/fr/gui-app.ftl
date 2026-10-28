# The main window: opening, importing, saving and exporting, parts, rest passes, the
# selection (crates/chipload-gui/src/app/mod.rs).

# The buttons of a folded side panel; P is the key that folds and unfolds both panels.
app-show-browser-hover = Afficher le Navigateur (P : les deux panneaux)
app-show-properties-hover = Afficher les Propriétés (P : les deux panneaux)
# The heading of the right-hand panel.
app-properties = Propriétés

app-start-hint = Importez un DXF ou SVG (Ctrl+I) pour démarrer un projet, ou ouvrez-en un (Ctrl+O).

app-cannot-open = Impossible d'ouvrir { $file } :
app-project = Projet : { $file }
app-old-schema = Ce fichier est au schéma version { $version } ; l'enregistrement écrit la version { $current }.

app-new-project = Nouveau projet.
# $what: a machine or tool from the user's library, e.g. machine "Router".
app-started-with = Démarré avec { $what }.
app-new-project-next = Dessinez des formes depuis le menu Dessiner, ou ajoutez un dessin avec Ajouter DXF. Vérifiez le brut et la machine dans Paramètres d'usinage, et l'outil dans Outils.
app-units-missing = { $file } n'indique pas ses unités ; choisissez mm ou pouce.
app-px-missing = { $file } ne donne sa taille qu'en pixels ; choisissez ses pixels par pouce.
app-cannot-import = Impossible d'importer { $file } :
app-new-project-from = Nouveau projet à partir de { $file } ({ $shapes } formes).
app-import-next = Sélectionnez des contours, puis Nouveau contour ou Nouvelle poche. Vérifiez le brut et la machine dans Paramètres d'usinage, et l'outil dans Outils.

# Followed by the list of problems.
app-project-incomplete = Le projet n'est pas complet :
app-lost-geometry = Des opérations ont perdu leur géométrie : choisissez-la à nouveau
app-could-not-update-drawing = Impossible de mettre à jour le dessin :
app-could-not-add = Impossible d'ajouter { $file } :
app-could-not-move-drawing = Impossible de déplacer le dessin :

# The distance between copies of a part; $dy and $gap carry their unit.
app-pitch-set = Pas { $dx } × { $dy } (écart { $gap })
app-pitch-needs-plan = Impossible de régler le pas d'après un écart : la pièce doit d'abord être calculée.

app-deleted-part = { $count ->
    [0] Pièce { $name } supprimée (Ctrl+Z la rétablit)
    [one] Pièce { $name } et son opération supprimées (Ctrl+Z les rétablit)
   *[other] Pièce { $name } et ses { $count } opérations supprimées (Ctrl+Z les rétablit)
}
app-part-needed = Un projet doit avoir au moins une pièce

# A rest pass clears what a larger tool left in a pocket.
app-could-not-add-rest-pass = Impossible d'ajouter une passe de reprise : { $error }
app-rest-with-corners = Cette poche a des coins dogbone ou T-bone, et les passes de reprise ne peuvent pas encore s'y combiner. Les coins atteignent déjà ce qu'une passe de reprise atteindrait.
# $tool: a tool number such as T3.
app-rest-tool-missing = L'outil { $tool } n'est pas dans le projet : donnez d'abord un outil à la poche.
app-rest-no-smaller = Aucune fraise plus petite que { $tool } ({ $diameter }) dans le projet ni dans votre bibliothèque d'outils. Ajoutez-en une dans Outils, puis refaites un clic droit sur la poche.
app-now-rest-pass = L'opération { $op } est maintenant une passe de reprise de l'opération { $parent } ; elle est usinée après la suite de poches dont elle fait partie

app-generate-before-export = Calculez (F5) avant d'exporter le G-code.
app-planning-then-export = Calcul de chaque pièce, puis export
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
app-selection-dxf = DXF { $id }
app-selection-image = image { $id }
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
