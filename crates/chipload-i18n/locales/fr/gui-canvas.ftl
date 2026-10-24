# The drawing canvas: its right-click menu, hover texts and notices (crates/chipload-gui/src/app/canvas.rs).

# The right-click menu. A ring's start: where a profile or pocket lap begins.
canvas-reset-start = Réinitialiser le départ
canvas-reset-start-disabled-hover = Ce tour démarre là où Chipload l'a choisi
# $id: the contour's number.
canvas-select-contour = Sélectionner #{ $id }
canvas-select-layer = Sélectionner le calque { $layer }
canvas-right-click-hint = Clic droit sur un contour pour sélectionner son calque.

# Hovering a ring's start marker.
canvas-start-moved-hover = Départ de ce tour : glisser le long du tour ; clic droit pour réinitialiser
canvas-start-hover = Départ de ce tour : glisser le long du tour pour le déplacer

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
