# The canvas's legend: what each kind of line on the canvas means
# (crates/chipload-gui/src/view.rs).

# A closed contour that is not inside another one.
view-legend-outline = ytterkontur
# A closed contour inside another one.
view-legend-hole = hål
view-legend-open = öppen kontur (fräses inte)
view-legend-selected = markerad
# The contours the selected operation cuts.
view-legend-operation = operationens konturer
# The material block the part is cut from.
view-legend-stock = ämne
# Copies of the part laid out in a grid.
view-legend-copies = kopior
view-legend-cutting = skärande rörelse
# A rapid move: the tool travels fast above the material.
view-legend-rapid = snabbförflyttning, ovanför ämnet
view-legend-plunge = där verktyget går ned
# A small bridge left uncut that holds the part in the stock.
view-legend-tab = brygga
# Where a closed toolpath ring starts; the user can drag it.
view-legend-ring-start = slingstart (dra för att flytta)
