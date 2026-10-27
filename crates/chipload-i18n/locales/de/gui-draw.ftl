# Drawing shapes with the mouse: the status bar while drawing, and snapping
# (crates/chipload-gui/src/draw.rs). Esc, Enter and Backspace are keyboard keys; the
# grid is the step clicks are rounded to, e.g. "1 mm".

draw-status-circle-start = Kreis zeichnen: Mittelpunkt anklicken (Raster { $grid }, Esc bricht ab)
draw-status-circle = Kreis zeichnen, Radius { $radius }: einen Punkt auf dem Kreis anklicken (Raster { $grid }, Esc bricht ab)
draw-status-rectangle-start = Rechteck zeichnen: eine Ecke anklicken (Raster { $grid }, Esc bricht ab)
# The rectangle's size so far: along X, then along Y.
draw-status-rectangle = Rechteck zeichnen, { $length } x { $width }: die gegenüberliegende Ecke anklicken (Raster { $grid }, Esc bricht ab)
draw-status-polyline-start = Polylinie zeichnen, { $count ->
        [one] { $count } Punkt
       *[other] { $count } Punkte
    }: den ersten Punkt anklicken (Raster { $grid }, Esc bricht ab)
draw-status-polyline = Polylinie zeichnen, { $count ->
        [one] { $count } Punkt
       *[other] { $count } Punkte
    }: den nächsten Punkt anklicken; Enter oder Doppelklick beendet sie offen, der erste Punkt schließt sie, Backspace entfernt den letzten Punkt (Raster { $grid }, Esc bricht ab)

# Why a click did nothing.
draw-no-size-circle = der Kreis hätte keine Größe
draw-no-size-rectangle = das Rechteck hätte keine Größe
draw-polyline-too-short = eine Polylinie braucht mindestens 2 Punkte

# The point the cursor snapped to, on contour #4.
draw-snap-end = Ende von #{ $contour }
draw-snap-centre = Mittelpunkt von #{ $contour }
draw-snap-middle = Mitte von #{ $contour }
draw-snap-intersection = Schnittpunkt von #{ $contour } und #{ $other }
draw-snap-intersection-self = Schnittpunkt auf #{ $contour }
draw-snap-quadrant = Quadrantenpunkt von #{ $contour }
draw-snap-perpendicular = Lot auf #{ $contour }
draw-snap-tangent = Tangente an #{ $contour }
draw-snap-nearest = auf #{ $contour }
