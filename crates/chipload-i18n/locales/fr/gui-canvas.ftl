# The drawing canvas: its right-click menu, hover texts and notices (crates/chipload-gui/src/app/canvas.rs).

# The right-click menu. A ring's start: where a profile or pocket lap begins.
canvas-reset-start = Réinitialiser le départ
canvas-reset-start-disabled-hover = Ce tour démarre là où Chipload l'a choisi
canvas-turn-axis = ⟲ Axe de retournement
# $id: the contour's number.
canvas-select-contour = Sélectionner #{ $id }
canvas-select-layer = Sélectionner le calque { $layer }
canvas-select-similar = Sélectionner les semblables
canvas-select-similar-hover = Cercles de même diamètre sur tout calque, ou formes de même taille sur ce calque. Ctrl : ajouter à la sélection
canvas-select-inside = Sélectionner l’intérieur
canvas-select-inside-hover = Les formes fermées à l’intérieur de celle-ci : poches et trous d’une pièce. Ctrl : ajouter à la sélection
canvas-right-click-hint = Clic droit sur un contour pour sélectionner son calque.

# Hovering a ring's start marker.
canvas-start-moved-hover = Départ de ce tour : glisser le long du tour ; clic droit pour réinitialiser
canvas-start-hover = Départ de ce tour : glisser le long du tour pour le déplacer
canvas-tab-hover = Attache : la glisser le long du tour ; clic droit pour la retirer ou répartir régulièrement les attaches du tour
canvas-tab-hover-placed = Attache placée à la main : la glisser le long du tour ; clic droit pour la retirer ou répartir régulièrement les attaches du tour
# A dropped tab or ring start that the planner refuses goes back; $reason says why.
canvas-drop-refused = Non déplacé : { $reason }
canvas-tab-remove = Retirer l'attache
canvas-tab-even = Replacer les attaches de ce tour
canvas-tab-add = Ajouter une attache ici
canvas-corner-hover = Coin en { $at }, { $angle }° : { $gets }. Cliquer pour choisir à part
canvas-corner-hover-own = Coin en { $at }, { $angle }° : { $gets }, choisi à part. Cliquer pour changer
canvas-corner-hover-unreached = Coin en { $at }, { $angle }° : { $gets }, mais l'outil ne peut pas l'atteindre, il reste arrondi. Cliquer pour choisir
canvas-corner-as-operation = Comme l'opération ({ $gets })
canvas-corner-round = Arrondi
canvas-corner-dogbone = Dogbone
canvas-corner-tbone-longer = T-bone le long du plus long côté
canvas-corner-tbone-shorter = T-bone le long du plus court côté
canvas-corner-gets-round = arrondi
canvas-corner-gets-dogbone = un dogbone
canvas-corner-gets-tbone-longer = un T-bone le long du plus long côté
canvas-corner-gets-tbone-shorter = un T-bone le long du plus court côté

# Hovering plunges: "Operation 2 (Pocket 1): plunges 3, 13, 23 of 30". $operation is the
# operation's label, $numbers the list of plunge numbers, $total how many it has.
canvas-plunges = { $operation } : { $count ->
        [one] plongée
       *[other] plongées
    } { $numbers } sur { $total }
canvas-rejected-toolpath = Parcours d'outil rejeté

# The notice at the canvas's top.
canvas-updating-toolpath = Mise à jour du parcours d'outil…
canvas-toolpath-out-of-date = Parcours d'outil périmé
# A button: generate the toolpath again.
canvas-generate = Calculer
canvas-generate-hover = Le projet a changé depuis le dernier calcul (F5)
canvas-updating-simulation = Mise à jour de la simulation…

# The legend in the bottom left corner.
canvas-legend = Légende
canvas-legend-hover = Ce que montre le canevas
canvas-show-toolpath = Afficher le parcours
canvas-show-toolpath-hover = Masquer le parcours d'outil pour voir le dessin ou le brut simulé

## La boussole (docs/specs/compass.md).
compass-menu-item = Boussole
compass-hover = Déplacer le long d’une flèche, tourner avec l’anneau, glisser le centre sur un point ; glisser avec le bouton droit pour copier ou faire un réseau ; Espace la libère pour la placer
compass-select-first = Sélectionnez d’abord des formes dessinées, un groupe, une image, un texte, un modèle 3D ou un dessin entier, puis Q
compass-unlocked = Boussole libérée : glissez-la en place, Espace la bloque
compass-locked = Boussole bloquée : glisser déplace la sélection
compass-typed-unreadable = Illisible : tapez une longueur, un angle en degrés, ou ΔX et ΔY
compass-move-here = Déplacer ici
compass-turn-here = Tourner ici
compass-copy-here = Copier ici
compass-linear-array = Réseau linéaire…
compass-circular-array = Réseau circulaire…
compass-copies-drawn-only = Seules les formes dessinées se copient
compass-status = Boussole : glissez une flèche, l’anneau ou le centre ; bouton droit pour copier ; Espace la libère ; Q ou Échap la masque
compass-status-arrow = Glisser déplace le long de cette flèche ; double-clic pour taper la distance ; bouton droit pour copier ou faire un réseau linéaire ; clic droit pour aller à un point
compass-status-ring = Glisser tourne autour du centre par pas de 15° (Alt : 0,1°) ; double-clic pour taper l’angle ; bouton droit pour un réseau circulaire
compass-status-centre = Glisser déplace librement, le centre accroché aux points ; double-clic pour taper ΔX et ΔY ; clic droit pour aller à un point
compass-status-place = Glisser pose la boussole sur un point ; Espace la bloque
compass-status-aim = Glisser tourne les axes de la boussole, ou en couche un le long d’une ligne ; Espace la bloque
compass-to-point = Vers un point
compass-to-point-hover = Cliquez ensuite un point : la sélection y est déplacée (accrochage comme en glissant)
compass-to-centre = Vers un centre
compass-to-centre-hover = Cliquez ensuite une forme : la sélection va à son centre (celui d'un cercle ou d'un arc, sinon le milieu de son cadre)
compass-no-centre-here = Aucune forme ici : cliquez une forme pour aller à son centre (Échap annule)
compass-status-pick-point = Cliquez le point où déplacer la sélection (Alt : sans accrochage ; Échap annule)
compass-status-pick-point-axis = Cliquez un point : la sélection y va le long de la flèche (Alt : sans accrochage ; Échap annule)
compass-status-pick-centre = Cliquez une forme : la sélection va à son centre (Échap annule)
compass-status-pick-centre-axis = Cliquez une forme : la sélection va le long de la flèche jusqu’à son centre (Échap annule)
compass-value-distance = Distance
compass-value-angle = Angle
compass-value-delta = ΔX ΔY
# A part cut elsewhere than drawn: the label on the dashed box round its shapes
# (docs/specs/part-placement.md §4).
canvas-not-cut-here = { $part } : pas usinée ici
