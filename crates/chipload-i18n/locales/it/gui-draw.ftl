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
modify-offset-pick = Offset: seleziona le forme (clicca su una; Ctrl aggiunge), Esc termina
modify-offset-status = Offset { $distance } { $side }: clicca, o digita la distanza (Alt: fuori griglia, Esc termina)
modify-offset-done = { $count ->
    [one] Aggiunto un offset
   *[other] Aggiunti { $count } offset
} a { $distance }.
modify-side-outside = all'esterno
modify-side-inside = all'interno
modify-side-left = a sinistra
modify-side-right = a destra
modify-corners-round = Angoli arrotondati
modify-corners-sharp = Angoli vivi
modify-corners-hover = Arrotondati: l'offset esatto, archi attorno agli angoli. Vivi: i lati prolungati fino a incontrarsi, come una cornice.
modify-offset-count = Numero
modify-corner-pick = Seleziona una forma disegnata (cliccala), poi clicca i suoi angoli; Esc termina
modify-fillet-status = Raccordo: clicca un angolo; Maiusc+clic li arrotonda tutti (Esc termina)
modify-chamfer-status = Smusso: clicca un angolo; Maiusc+clic li taglia tutti (Esc termina)
modify-fillet-radius = Raggio
modify-chamfer-distance = Distanza
modify-trim-status = Taglia: clicca il pezzo di una forma disegnata da togliere; si vede prima del clic (Esc termina)
modify-extend-status = Estendi: clicca vicino all'estremità di una forma aperta per allungarla fino alla forma successiva (Esc termina)
modify-break-status = Spezza: clicca una forma disegnata dove deve dividersi (Esc termina)
modify-extend-closed = una forma chiusa non ha estremità da estendere
modify-combine-pick = Seleziona prima due o più forme disegnate chiuse
modify-subtract-status = Sottrai: clicca la forma da cui ritagliare quelle selezionate (Esc termina)
modify-array-pick = Seleziona prima le forme disegnate da copiare
modify-array-status = Serie: imposta le copie sotto Selezione, poi Crea (Esc termina)
modify-polar-status = Serie polare: clicca il centro, imposta le copie sotto Selezione, poi Crea (Esc termina)
array-columns = Colonne
array-rows = Righe
array-dx = Spaziatura colonne
array-dy = Spaziatura righe
array-count = Numero
array-center = Centro
array-angle = Angolo
array-angle-hover = 360° distribuisce le copie in modo uniforme tutt'attorno; meno mette la prima e l'ultima ai suoi estremi; negativo va in senso orario
array-rotate = Ruota le copie
array-create = Crea
