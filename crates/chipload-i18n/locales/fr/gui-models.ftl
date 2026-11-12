# 3D models in the window: File > Add 3D model…, their rows in the Browser and their
# properties (crates/chipload-gui/src/app/models.rs, sources.rs, forms/sources.rs).

models-stl-filter = Modèles STL
models-cannot-read = Impossible de lire { $path } : { $error }
models-cannot-add = Impossible d'ajouter { $path } :
models-cannot-place = Impossible de placer le modèle ainsi :
models-removed = Modèle { $id } supprimé
dialogs-path-model-title = Ajouter un modèle 3D
dialogs-path-model = Un fichier STL (binaire ou texte) :
menu-add-model = Ajouter un modèle 3D…
menu-add-model-hover = Un fichier STL, usiné dans la planche par le dessus en relief
menu-add-model-path = Ajouter un modèle 3D depuis un chemin…
sources-model-title = { $name } (modèle { $id })
sources-add-model = Ajouter un modèle 3D…
sources-add-model-hover = Un fichier STL, usiné dans la planche par le dessus en relief
sources-model-hover = Un modèle 3D : son contour sur la planche. Double-clic pour sa position, sa taille et sa profondeur
sources-model-new-relief = Nouveau relief
sources-model-new-relief-hover = Usiner ce modèle : une opération Relief dans son contour, dans la pièce active
sources-model-properties = Propriétés
sources-remove-model-hover = Supprimer le modèle et son contour ; ses reliefs restent, chacun avec un problème jusqu'à ce qu'il ait un autre modèle
forms-model-file = Fichier
forms-model-size = Sur la planche
forms-model-size-hover = Largeur × hauteur × profondeur du modèle placé
forms-model-position-hover = Le coin inférieur gauche du contour du modèle, avant rotation
forms-model-reset-position = Retour à l'origine du dessin
forms-model-rotation-hover = Tourné dans le sens antihoraire autour du centre du contour
forms-model-reset-rotation = Pas tourné
forms-model-width-hover = La largeur du modèle sur la planche ; la hauteur suit
forms-model-depth = Profondeur
forms-model-depth-hover = La profondeur du relief, du haut du modèle à son point le plus bas. Les reliefs sont souvent plus plats que le modèle
forms-model-proportional = Garder les proportions
forms-model-proportional-hover = La profondeur suit la largeur, dans la proportion du modèle
forms-model-up = Haut
forms-model-up-hover = Quel axe du modèle pointe vers le haut depuis la planche. Beaucoup de fichiers sont modélisés avec Y vers le haut
forms-model-layer-hover = Le calque sur lequel le contour du modèle est dessiné
forms-model-new-relief = Nouveau relief
forms-model-new-relief-hover = Usiner ce modèle : une opération Relief dans son contour, dans la pièce active
forms-remove-model-hover = Supprimer le modèle et son contour ; ses reliefs restent, chacun avec un problème jusqu'à ce qu'il ait un autre modèle

# Reliefs and their roughing pass (docs/specs/stl-relief.md §9, §14).
models-could-not-add-roughing = Impossible d'ajouter la passe d'ébauche : { $error }
models-no-roughing-tool = Aucune fraise droite ni hémisphérique simple dans le projet ou votre bibliothèque
