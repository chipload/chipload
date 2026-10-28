# Help > Keyboard shortcuts (crates/chipload-gui/src/app/shortcuts.rs).

shortcuts-window = Raccourcis clavier

# The window's group headings: menus (File, Edit, View, Help), the Generate button, and
# what is done on the canvas or while drawing a shape.
shortcuts-group-file = Fichier
shortcuts-group-edit = Édition
shortcuts-group-generate = Calculer
shortcuts-group-view = Vue
shortcuts-group-help = Aide
shortcuts-group-canvas = Canevas
shortcuts-group-drawing = Dessin de formes

# What each keyboard shortcut does.
shortcuts-copy-selector = Copier la sélection comme sélecteur (JSON)
shortcuts-save-as = Enregistrer sous
shortcuts-redo = Rétablir
shortcuts-copy = Copier les formes dessinées sélectionnées
shortcuts-paste = Les coller (au curseur, sur le canevas)
shortcuts-new = Nouveau projet
shortcuts-open = Ouvrir un projet
shortcuts-import = Nouveau projet à partir d'un DXF ou SVG
shortcuts-save = Enregistrer
shortcuts-export = Exporter le G-code (une fois calculé)
shortcuts-quit = Quitter
shortcuts-undo = Annuler
shortcuts-move-up = Monter l'opération
shortcuts-move-down = Descendre l'opération
shortcuts-generate = Calculer le parcours d'outil et le G-code
shortcuts-fit = Ajuster la vue
shortcuts-top-view = Vue de dessus
shortcuts-grid = Afficher ou masquer la grille
shortcuts-measure = Mesurer : deux clics donnent la distance et l'angle
shortcuts-panels = Replier ou déplier les panneaux latéraux, pour un canevas plus grand
shortcuts-this-list = Cette liste

# Mouse actions and keys that aren't menu shortcuts: the keys column (key names as on the
# keyboard), then what they do.
shortcuts-click = Clic
shortcuts-click-select = Sélectionner un contour
shortcuts-ctrl-click = Ctrl+clic
shortcuts-ctrl-click-what = Ajouter un contour à la sélection, ou l'en retirer
shortcuts-right-click = Clic droit
shortcuts-right-click-what = Sélectionner un contour ou son calque ; réinitialiser un départ de tour déplacé
shortcuts-scroll = Molette
shortcuts-scroll-what = Zoomer au curseur
shortcuts-drag = Glisser
shortcuts-drag-what = Sélectionner par un cadre : de gauche à droite ce qui est dedans, de droite à gauche ce qu'il touche ; Maj ajoute, Ctrl ajoute ou retire
shortcuts-pan-drag = Glisser droit ou milieu, Espace+glisser
shortcuts-pan-drag-what = Déplacer la vue
shortcuts-delete = Del
shortcuts-delete-what = Supprimer les formes dessinées sélectionnées
shortcuts-escape = Esc
shortcuts-escape-clear = Vider la sélection
# A ring: a profile's closed path; its start can be dragged on the canvas.
shortcuts-drag-start = Glisser le départ d'un tour
shortcuts-drag-start-what = Déplacer l'endroit où le tour commence
shortcuts-drop = Déposer un fichier
shortcuts-drop-what = Ouvrir un projet .chipload, ou en démarrer un depuis un .dxf
shortcuts-click-point = Placer un point (accroche aux extrémités, centres et milieux)
shortcuts-alt = Alt
shortcuts-alt-what = Maintenu : pas d'accrochage
shortcuts-enter = Enter, double-clic
shortcuts-enter-what = Terminer une polyligne ; sans outil, redessiner avec le dernier
shortcuts-backspace = Backspace
shortcuts-backspace-what = Retirer le dernier point
shortcuts-escape-stop = Arrêter de dessiner
shortcuts-drag-handle = Glisser une poignée
shortcuts-drag-handle-what = Déplacer un point, le centre ou un coin ; tourner un rectangle
shortcuts-drag-middle = Glisser une ligne d'une polyligne
shortcuts-drag-middle-what = Déplacer ce segment
shortcuts-ctrl-drag-middle = Ctrl+glisser une ligne d'une polyligne
shortcuts-ctrl-drag-middle-what = Y insérer un point
shortcuts-shift-drag-middle = Shift+glisser une ligne d'une polyligne
shortcuts-shift-drag-middle-what = Courber le segment en arc
shortcuts-draw-polyline = Dessiner une polyligne
shortcuts-draw-circle = Dessiner un cercle
shortcuts-draw-rectangle = Dessiner un rectangle
shortcuts-type-value = Taper un nombre
shortcuts-type-value-what = Un point exact : x,y  @dx,dy  longueur  longueur<angle
shortcuts-shift = Maj
shortcuts-shift-what = Maintenir : la ligne par pas de 15°
shortcuts-drag-shape = Glisser une forme dessinée
shortcuts-drag-shape-what = La déplacer, ou les formes sélectionnées avec elle ; elle s'accroche, Maj garde des pas de 15°
shortcuts-orbit-drag = Glisser dans la vue 3D
shortcuts-orbit-drag-what = Orbiter ; Maj+glisser déplace la vue
shortcuts-draw-arc = Dessiner un arc
shortcuts-draw-slot = Dessiner une rainure
shortcuts-draw-polygon = Dessiner un polygone
shortcuts-offset = Décaler les formes sélectionnées
shortcuts-trim = Ajuster des formes dessinées
shortcuts-construction = Géométrie de construction : pour les prochaines formes dessinées, ou la sélection
