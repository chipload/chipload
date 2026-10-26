# The canvas's legend: what each kind of line on the canvas means
# (crates/chipload-gui/src/view.rs).

# A closed contour that is not inside another one.
view-legend-outline = outline
# A closed contour inside another one.
view-legend-hole = hole
view-legend-open = open contour (not cut)
view-legend-selected = selected
# The contours the selected operation cuts.
view-legend-operation = the operation's contours
# The material block the part is cut from.
view-legend-stock = stock
# Copies of the part laid out in a grid.
view-legend-copies = copies
view-legend-cutting = cutting move
# A rapid move: the tool travels fast above the material.
view-legend-rapid = rapid, above the stock
view-legend-plunge = where the tool goes down
# A small bridge left uncut that holds the part in the stock.
view-legend-tab = tab
# Where a closed toolpath ring starts; the user can drag it.
view-legend-ring-start = ring start (drag to move)
view-legend-corner = corner (click to choose its own)
