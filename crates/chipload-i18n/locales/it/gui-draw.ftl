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

# A value typed while drawing that can't be used.
draw-entry-unreadable = illeggibile: digita x,y  @dx,dy  una lunghezza, o lunghezza<angolo
draw-entry-needs-point = il primo punto: digita x,y
draw-entry-no-direction = allontana il cursore dall'ultimo punto per dare la direzione
draw-status-arc-start = Disegno di un arco: clicca dove inizia (griglia { $grid }, Esc annulla)
draw-status-arc-end = Disegno di un arco, corda { $length }: clicca dove finisce (griglia { $grid }, Esc annulla)
draw-status-arc = Disegno di un arco di raggio { $radius }: clicca un punto per cui passa, o digita il raggio (griglia { $grid }, Esc annulla)
draw-status-slot-start = Disegno di un'asola: clicca il centro di un'estremità (griglia { $grid }, Esc annulla)
draw-status-slot-end = Disegno di un'asola, { $length } tra le estremità: clicca il centro dell'altra estremità (griglia { $grid }, Esc annulla)
draw-status-slot = Disegno di un'asola larga { $width }: clicca per darne la larghezza, o digitala (griglia { $grid }, Esc annulla)
draw-arc-on-a-line = i tre punti sono allineati: usa lo strumento polilinea
draw-no-size-slot = l'asola non avrebbe lunghezza o larghezza
draw-entry-radius-too-small = il raggio deve essere almeno metà della distanza tra le estremità
draw-entry-no-side = sposta il cursore dal lato verso cui l'arco deve curvare
draw-status-polygon-start = Disegno di un poligono: clicca il centro (griglia { $grid }, Esc annulla)
draw-status-polygon = Disegno di un poligono di raggio { $radius }: clicca un vertice (griglia { $grid }, Esc annulla)
