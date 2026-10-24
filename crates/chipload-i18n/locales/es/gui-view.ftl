# The canvas's legend: what each kind of line on the canvas means
# (crates/chipload-gui/src/view.rs).

# A closed contour that is not inside another one.
view-legend-outline = contorno exterior
# A closed contour inside another one.
view-legend-hole = agujero
view-legend-open = contorno abierto (no se corta)
view-legend-selected = seleccionado
# The contours the selected operation cuts.
view-legend-operation = contornos de la operación
# The material block the part is cut from.
view-legend-stock = material en bruto
# Copies of the part laid out in a grid.
view-legend-copies = copias
view-legend-cutting = movimiento de corte
# A rapid move: the tool travels fast above the material.
view-legend-rapid = rápido, por encima del material
view-legend-plunge = donde baja la herramienta
# A small bridge left uncut that holds the part in the stock.
view-legend-tab = puente
# Where a closed toolpath ring starts; the user can drag it.
view-legend-ring-start = inicio de anillo (arrastra para moverlo)
