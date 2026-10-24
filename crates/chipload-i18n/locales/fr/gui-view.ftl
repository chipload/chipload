# The canvas's legend: what each kind of line on the canvas means
# (crates/chipload-gui/src/view.rs).

# A closed contour that is not inside another one.
view-legend-outline = contour extérieur
# A closed contour inside another one.
view-legend-hole = trou
view-legend-open = contour ouvert (non usiné)
view-legend-selected = sélectionné
# The contours the selected operation cuts.
view-legend-operation = les contours de l'opération
# The material block the part is cut from.
view-legend-stock = brut
# Copies of the part laid out in a grid.
view-legend-copies = copies
view-legend-cutting = mouvement de coupe
# A rapid move: the tool travels fast above the material.
view-legend-rapid = déplacement rapide, au-dessus du brut
view-legend-plunge = où l'outil descend
# A small bridge left uncut that holds the part in the stock.
view-legend-tab = attache
# Where a closed toolpath ring starts; the user can drag it.
view-legend-ring-start = départ du tour (glisser pour déplacer)
