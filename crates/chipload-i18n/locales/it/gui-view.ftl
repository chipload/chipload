# The canvas's legend: what each kind of line on the canvas means
# (crates/chipload-gui/src/view.rs).

# A closed contour that is not inside another one.
view-legend-outline = contorno esterno
# A closed contour inside another one.
view-legend-hole = foro
view-legend-open = contorno aperto (non lavorato)
view-legend-selected = selezionato
# The contours the selected operation cuts.
view-legend-operation = i contorni della lavorazione
# The material block the part is cut from.
view-legend-stock = grezzo
# Copies of the part laid out in a grid.
view-legend-copies = copie
view-legend-cutting = movimento di taglio
# A rapid move: the tool travels fast above the material.
view-legend-rapid = rapido, sopra il grezzo
view-legend-plunge = dove l'utensile scende
# A small bridge left uncut that holds the part in the stock.
view-legend-tab = ponticello
# Where a closed toolpath ring starts; the user can drag it.
view-legend-ring-start = inizio anello (trascina per spostarlo)
