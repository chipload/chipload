# The Browser's Drawing folder: the project's DXFs with their layers, its pictures, texts
# and drawn shapes (crates/chipload-gui/src/app/sources.rs).

# The folder's name: everything that is drawn, a noun.
sources-drawing = Dessin
sources-drawing-hover = DXF, images, textes et formes dessinées ; clic droit pour en ajouter
sources-add-dxf = Ajouter un DXF…
sources-add-dxf-hover = Une autre pièce : un DXF placé et mis à jour séparément
# "File > Import DXF" names a menu item.
sources-add-dxf-disabled-hover = Fichier > Importer un DXF démarre un projet à partir d'un DXF
sources-import-image = Importer une image…
sources-import-image-hover = Vectoriser une image en formes, pour la gravure en V
sources-add-text = Ajouter un texte
sources-add-text-hover = Écrire un texte, puis cliquer là où il commence
sources-no-shapes = Pas encore de formes.
# The name of DXF files in the file dialog's filter.
sources-dxf-filter = Dessins DXF

# Rows of the folder. $file: the picture's file name; $text: what the text says.
sources-image-title = { $file } (image { $id })
sources-text-title = { $text } (texte { $id })
# Rows for the shapes drawn in the app, by kind.
sources-rectangles = Rectangles
sources-circles = Cercles
sources-polylines = Polylignes
sources-other-paths = Autres tracés

sources-dxf-hover = Un clic sélectionne tout le DXF : une opération créée à partir de lui suit ses mises à jour. Sa position et son échelle sont dans Propriétés ; clic droit pour plus
sources-image-hover = Un clic sélectionne toute l'image : une opération créée à partir d'elle suit ses nouvelles vectorisations. Sa taille et sa position sont dans Propriétés ; double-clic pour ses réglages de vectorisation
sources-text-hover = Un clic sélectionne tout le texte : une opération créée à partir de lui suit ses modifications. Son contenu, sa police, sa taille et sa position sont dans Propriétés
sources-drawn-hover = Un clic sélectionne ces formes ; Ctrl+clic les ajoute
sources-layer-hover = Un clic sélectionne ce calque de ce DXF : une opération créée à partir de lui n'usine que ce calque de ce DXF

# Right-click menu items, verbs.
sources-select = Sélectionner
sources-show = Afficher
sources-hide = Masquer
sources-place = Placer…
sources-place-hover = Déplacer, tourner ou mettre à l'échelle ce DXF, dans ses propriétés
sources-update-from-dxf = Mettre à jour depuis un DXF…
sources-update-from-path = Mettre à jour depuis un chemin…
sources-remove-dxf-hover = Retirer ce DXF et ses formes
sources-trace-settings = Réglages de vectorisation…
sources-trace-settings-hover = Seuil, lissage, contours ou lignes médianes ; sa taille et sa position sont dans ses propriétés
sources-remove-image-hover = Retirer l'image et ses formes
sources-remove-text-hover = Retirer le texte et ses lettres

# Followed by the project's problems, one per line.
sources-project-incomplete = Le projet n'est pas complet :
# Followed by the error on its own line.
sources-remove-dxf-failed = Impossible de retirer le DXF :
sources-explode = Décomposer en formes…
sources-explode-hover = Transformer les formes de ce DXF en formes dessinées modifiables ; le lien avec le fichier DXF disparaît
sources-explode-failed = Impossible de décomposer le DXF :
sources-join-failed = Impossible de joindre les formes :
sources-drawn-layer-hover = Cliquez pour sélectionner ces formes ; l'œil les masque
sources-shape-hover = Cliquez pour la sélectionner ; Ctrl+clic ajoute ou retire ; clic droit pour renommer, copier ou supprimer
sources-rename = Renommer…
sources-copy-shape = Copier
sources-delete-shape = Supprimer
sources-polygons = Polygones
