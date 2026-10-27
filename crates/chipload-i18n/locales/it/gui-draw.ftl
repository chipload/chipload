# Drawing shapes with the mouse: the status bar while drawing, and snapping
# (crates/chipload-gui/src/draw.rs). Esc, Enter and Backspace are keyboard keys; the
# grid is the step clicks are rounded to, e.g. "1 mm".

draw-status-circle-start = Disegno di un cerchio: fai clic sul centro (griglia { $grid }, Esc annulla)
draw-status-circle = Disegno di un cerchio di raggio { $radius }: fai clic su un punto del cerchio (griglia { $grid }, Esc annulla)
draw-status-rectangle-start = Disegno di un rettangolo: fai clic su un angolo (griglia { $grid }, Esc annulla)
# The rectangle's size so far: along X, then along Y.
draw-status-rectangle = Disegno di un rettangolo { $length } x { $width }: fai clic sull'angolo opposto (griglia { $grid }, Esc annulla)
draw-status-polyline-start = Disegno di una polilinea, { $count ->
        [one] { $count } punto
       *[other] { $count } punti
    }: fai clic sul primo punto (griglia { $grid }, Esc annulla)
draw-status-polyline = Disegno di una polilinea, { $count ->
        [one] { $count } punto
       *[other] { $count } punti
    }: fai clic sul punto successivo; Enter o doppio clic la termina aperta, il primo punto la chiude, Backspace rimuove l'ultimo punto (griglia { $grid }, Esc annulla)

# Why a click did nothing.
draw-no-size-circle = il cerchio non avrebbe dimensione
draw-no-size-rectangle = il rettangolo non avrebbe dimensione
draw-polyline-too-short = una polilinea richiede almeno 2 punti

# The point the cursor snapped to, on contour #4.
draw-snap-end = estremo di #{ $contour }
draw-snap-centre = centro di #{ $contour }
draw-snap-middle = punto medio di #{ $contour }
draw-snap-intersection = intersezione di #{ $contour } e #{ $other }
draw-snap-intersection-self = intersezione su #{ $contour }
draw-snap-quadrant = quadrante di #{ $contour }
draw-snap-perpendicular = perpendicolare a #{ $contour }
draw-snap-tangent = tangente a #{ $contour }
draw-snap-nearest = su #{ $contour }
