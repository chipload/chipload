# The menu bar and its menus (crates/chipload-gui/src/app/menu.rs).

menu-language = Langue
menu-language-system = Système
menu-language-system-hover = La langue du système d'exploitation si Chipload l'a, sinon l'anglais
menu-language-review = { $language } (relecture souhaitée)
menu-language-review-hover = Traduit, pas encore relu par un locuteur natif qui usine. Les corrections sont les bienvenues : voir le guide de l'utilisateur, Languages.

# File menu.
menu-new-project = Nouveau projet
menu-open = Ouvrir…
menu-open-recent = Ouvrir un récent
# Empties the list of recent projects.
menu-clear-list = Vider la liste
menu-import-dxf = Importer un DXF…
menu-import-image = Importer une image…
menu-import-image-hover = Vectoriser une image en formes, pour la gravure en V
menu-open-path = Ouvrir un chemin…
menu-import-image-path = Importer une image depuis un chemin…
menu-several-dxfs = Plusieurs DXF : mettez-en un à jour depuis le Navigateur (clic droit dessus)
menu-add-dxf = Ajouter un DXF au projet…
menu-add-dxf-hover = Une autre pièce : un DXF placé et mis à jour séparément
menu-add-dxf-path = Ajouter un DXF depuis un chemin…
menu-save = Enregistrer
menu-save-as = Enregistrer sous…
menu-save-path = Enregistrer vers un chemin…
menu-export = Exporter le G-code…
menu-export-path = Exporter le G-code vers un chemin…
menu-quit = Quitter

# Edit: undo, and the selected drawn shapes.
menu-undo = Annuler
menu-redo = Rétablir
menu-only-drawn = Formes dessinées seulement ; les formes importées ne changent qu'avec le DXF
menu-copy = Copier
menu-paste = Coller
menu-mirror-left-right = Miroir gauche–droite
menu-mirror-up-down = Miroir haut–bas
menu-mirror-hover = Retourner les formes dessinées sélectionnées autour du milieu de leur boîte
menu-delete-shapes = Supprimer les formes
menu-delete-shapes-hover = Supprime les formes dessinées sélectionnées ; les formes importées ne changent qu'avec le DXF
# A selector: the selection written as JSON, for scripts.
menu-copy-selector = Copier la sélection

# Job menu.
menu-cancel-generation = Annuler le calcul
menu-generate = Calculer

# View menu.
menu-fit-view = Ajuster la vue
menu-top-view = Vue de dessus
menu-measure = Mesurer
menu-measure-hover = Deux clics donnent la distance, ΔX, ΔY et l'angle
menu-3d-view = Vue 3D
menu-z-scale = Échelle Z en 3D
menu-show-grid = Afficher la grille
menu-show-grid-hover = Lignes sur la grille d'accrochage, une plus marquée toutes les dix
menu-show-legend = Afficher la légende
menu-show-browser = Afficher le Navigateur
menu-show-properties = Afficher les Propriétés
menu-fold-panels = Replier les deux panneaux
menu-open-panels = Déplier les deux panneaux
menu-panels-hover = Le plus grand canevas : le Navigateur et les Propriétés repliés en bandes
menu-auto-update = Mettre à jour le parcours automatiquement
menu-auto-update-hover = Calculer tout seul une demi-seconde après chaque modification
menu-show-toolpath = Afficher le parcours
menu-show-toolpath-hover = Masquer le parcours d'outil pour voir le dessin ou le brut simulé
menu-show-drawing = Afficher le dessin
menu-show-drawing-hover = Masquer le dessin pour voir le brut ou le parcours d'outil seul
menu-show-images = Afficher les images
menu-show-images-hover = Les images vectorisées en formes, en transparence sur le brut
# The stock: the material being cut, as the simulation shows it.
menu-show-simulation = Afficher le brut (simulation)
menu-simulation-hint = Afficher le brut tel que le programme le laisse : chaque mouvement usiné avec la forme de l'outil (plus foncé = plus profond ; rouge = sous le brut). Affichage seulement.
menu-units = Unités
menu-units-mm-hover = Longueurs en mm, avances en mm/min
# The unit's name, as a choice next to "mm".
menu-units-inch = pouce
menu-units-inch-hover = Longueurs en pouces, avances en in/min ; les projets restent en mm
menu-theme = Thème de couleurs
menu-theme-system = Système
menu-theme-system-hover = Clair ou sombre, selon le réglage du système d'exploitation
menu-theme-light = Clair
menu-theme-light-hover = Panneaux et canevas clairs
menu-theme-dark = Sombre
menu-theme-dark-hover = Panneaux et canevas sombres

# Draw menu.
menu-draw-circle = Cercle
menu-draw-rectangle = Rectangle
menu-draw-polyline = Polyligne
menu-draw-text = Texte
menu-draw-text-hover = Écrire un texte, puis cliquer là où il commence

# Help menu.
menu-user-guide = Guide de l'utilisateur
menu-shortcuts = Raccourcis clavier
menu-about = À propos et licences

# The status bar under the canvas. What the toolpath is of, with several parts:
menu-status-part-of = pièce { $part } sur { $count }
menu-status-all-parts = toutes les pièces
# $time: how long the program runs, e.g. "12 min 30 s".
menu-status-cycle = cycle { $time }
menu-status-time-hover = Temps de cycle estimé (le rapport donne le détail)
menu-status-active-part = pièce active : { $name }
menu-status-active-part-hover = Les nouvelles opérations vont dans la pièce active (Navigateur, clic droit sur une pièce)
menu-status-units-hover = Les longueurs sont affichées dans ces unités (Vue > Unités)
# $step: the grid's step with its unit, e.g. "10 mm".
menu-status-grid = grille { $step }
menu-status-grid-hover = Le pas de la grille : les clics en dessinant s'y accrochent (G la masque)
menu-status-post-hover = Le post-processeur (Paramètres d'usinage > Machine)
# Pieces in all, from every part that is cut.
menu-status-pieces = { $count ->
    [one] { $count } exemplaire
   *[other] { $count } exemplaires
}
menu-status-copies = { $count ->
    [one] { $count } copie
   *[other] { $count } copies
}
# The cursor in program coordinates (from the work zero), after the drawing's X and Y.
menu-status-work = (coord. pièce X { $x }  Y { $y })
# $target: what the cursor snapped to, e.g. "end of #4".
menu-snapped-to = — accroché à { $target }
menu-generating = Calcul…
# $snapped: empty, or "— snapped to …" with a space before it.
menu-dragging = Glissement vers ({ $x }, { $y }){ $snapped } (Alt : sans accrochage ; Shift sur un milieu courbe le segment)
menu-status-hint = Cliquer un contour pour le sélectionner · Ctrl+clic ajoute ou retire · clic droit sélectionne son calque · Esc désélectionne
menu-join-ends = Joindre les extrémités…
menu-join-ends-hover = Joindre en une seule les polylignes dessinées ouvertes dont les extrémités se touchent presque : celles sélectionnées, ou toutes si aucune ne l'est
menu-join-ends-disabled-hover = Il n'y a aucune forme dessinée. Les formes d'un DXF peuvent être jointes une fois celui-ci décomposé (clic droit dessus dans le Navigateur).
menu-draw-arc = Arc
menu-draw-slot = Rainure
menu-draw-polygon = Polygone
menu-polygon-sides = Côtés
menu-modify-offset = Décaler
menu-modify-fillet = Congé
menu-modify-chamfer = Chanfrein
menu-modify-trim = Ajuster
menu-modify-extend = Prolonger
menu-modify-break = Couper
menu-modify-union = Union
menu-modify-subtract = Soustraire
menu-modify-intersect = Intersection
