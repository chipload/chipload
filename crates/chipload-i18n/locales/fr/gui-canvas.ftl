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
canvas-tab-hover = Attache : la glisser le long du tour ; clic droit pour la retirer ou répartir régulièrement les attaches du tour
canvas-tab-hover-placed = Attache placée à la main : la glisser le long du tour ; clic droit pour la retirer ou répartir régulièrement les attaches du tour
canvas-tab-remove = Retirer l'attache
canvas-tab-even = Répartir régulièrement les attaches de ce tour
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
