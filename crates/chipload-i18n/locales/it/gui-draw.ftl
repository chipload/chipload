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
draw-arc-behind = nessun arco tangente raggiunge un punto proprio dietro: clicca di lato, o passa alle linee (L)

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
modify-fillet-status = Raccordo: clicca un angolo (Maiusc+clic: tutti gli angoli della sua forma), o clicca una forma e poi un'altra per raccordarle; Esc termina
modify-chamfer-status = Smusso: clicca un angolo (Maiusc+clic: tutti gli angoli della sua forma), o clicca una linea e poi un'altra per smussare tra loro; Esc termina
modify-between-second = Ora clicca la seconda forma, sulla parte da tenere; Esc dimentica la prima
modify-fillet-radius = Raggio
modify-chamfer-distance = Distanza
modify-trim-status = Taglia: clicca il pezzo di una forma disegnata da togliere; si vede prima del clic (Esc termina)
modify-extend-status = Estendi: clicca vicino all'estremità di una forma aperta per allungarla fino alla forma successiva (Esc termina)
modify-break-status = Spezza: clicca una forma disegnata dove deve dividersi (Esc termina)
modify-extend-closed = una forma chiusa non ha estremità da estendere
modify-combine-pick = Seleziona prima due o più forme disegnate chiuse
modify-subtract-status = Sottrai: clicca la forma da cui ritagliare quelle selezionate (Esc termina)
modify-construction-on = Le prossime forme disegnate sono geometria di costruzione (K la disattiva).
modify-construction-off = Le prossime forme disegnate sono forme normali.
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
array-step = Passo
array-step-hover = Da una copia alla successiva, in X e in Y
array-rotate = Ruota le copie
array-create = Crea
arrange-align = Allinea
arrange-left = Lati sinistri allineati
arrange-centre = Centri allineati, in orizzontale
arrange-right = Lati destri allineati
arrange-top = Sommità allineate
arrange-middle = Centri allineati, in verticale
arrange-bottom = Basi allineate
arrange-stock = Al grezzo
arrange-stock-hover = Allinea le forme ai lati o al centro del grezzo invece che tra loro
arrange-first-hover = La prima forma scelta resta ferma, le altre si allineano a essa
arrange-all-hover = Allineate al riquadro che le racchiude tutte: nessuna forma è stata scelta per prima
arrange-distribute = Distribuisci
arrange-across = Spazi uguali in orizzontale; la più a sinistra e la più a destra restano ferme
arrange-up = Spazi uguali in verticale; la più bassa e la più alta restano ferme

# Disegna > Cornice… (docs/specs/shape-library.md §5)
modify-frame-status = Cornice attorno alla selezione: impostala sotto Selezione, poi Aggiungi (Esc termina)
modify-frame-size-status = Cornice di una misura: impostala sotto Selezione, o seleziona ciò che deve circondare (Esc termina)
frame-title = Cornice
frame-around = Attorno alla selezione, centrata su di essa.
frame-to-size = Di una misura, centrata sul grezzo.
frame-margin = Margine
frame-margin-hover = Dal rettangolo della selezione alla linea più interna
frame-width = Larghezza
frame-height = Altezza
frame-corner = Angolo
frame-corner-hover = La misura degli angoli: l'arrotondamento, l'intaglio, il gradino o lo smusso, quanto sale la parte alta o bassa, la bombatura della targa, la V dello stendardo
frame-corner-auto = auto ({ $size })
frame-corner-auto-hover = Impostala tu; disattivata: un decimo del lato più corto
frame-borders = Bordi
frame-gap = Distanza
frame-gap-hover = Dal contorno al primo bordo, e tra i bordi
frame-layer = Layer
frame-border-layer = Layer dei bordi
frame-size = { $width } × { $height }
frame-add = Aggiungi
frame-kind-rectangle = Rettangolo
frame-kind-rounded = Arrotondata
frame-kind-notched = Intagliata: angoli rientranti a quarto di cerchio
frame-kind-chamfered = Smussata
frame-kind-oval = Ovale
frame-kind-arched = Ad arco in alto
frame-kind-plaque = Targa: i lati corti bombati verso l'esterno
frame-kind-stepped = A gradino: angoli intagliati a quadrato
frame-kind-double-arch = Ad arco sopra e sotto
frame-kind-gable = Timpano: in alto a punta
frame-kind-shield = Scudo
frame-kind-banner = Stendardo: estremità a coda di rondine
frame-kind-octagon = Ottagono
frame-corners = Angoli
frame-corners-sharp = Vivi
frame-corners-round = Arrotondati
frame-corners-notch = Intaglio
frame-corners-chamfer = Smusso
frame-corners-step = Gradino
frame-border-corners = Angoli dei bordi
frame-border-corners-same = Come il contorno
frame-border-corners-hover = Gli angoli propri delle linee di bordo, alla stessa distanza; come il contorno: ne seguono la forma
frame-double = Linee doppie
frame-double-hover = Ogni bordo due scanalature, con una costa larga quanto una scanalatura in mezzo
frame-border-width = Larghezza scanalatura
frame-border-width-hover = Quanto è larga la scanalatura di un bordo in superficie (la larghezza della fresa a V alla profondità di taglio). Il margine è misurato fino al suo bordo interno; 0: fino alla linea.
frame-live-title = Cornice
frame-live-follows = { $texts ->
    [0] Segue { $others ->
        [one] una forma
       *[other] { $others } forme
    }: ridisegnata quando cambiano.
    [one] Segue il suo testo: ridisegnata quando cambiano le parole, il carattere o l'altezza.
   *[other] Segue { $texts } testi: ridisegnata quando cambiano.
}
frame-live-fixed = Disegnata a una misura.
frame-detach = Stacca
frame-detach-hover = Rende la cornice forme semplici che non seguono nulla. Modificare a mano una delle sue forme fa lo stesso.
draw-status-rectangle-centre-start = Disegno di un rettangolo dal centro: fai clic sul centro (griglia { $grid }, Esc annulla)
draw-status-rectangle-centre = Disegno di un rettangolo { $length } x { $width }: fai clic su un angolo (griglia { $grid }, Esc annulla)
draw-status-rectangle-aligned-start = Disegno di un rettangolo allineato: fai clic su un estremo di un lato (griglia { $grid }, Esc annulla)
draw-status-rectangle-aligned-edge = Disegno di un rettangolo allineato, lato { $length }: fai clic sull'altro estremo del lato (griglia { $grid }, Esc annulla)
draw-status-rectangle-aligned = Disegno di un rettangolo allineato { $length } x { $width }: clicca per darne la larghezza, o digitala (griglia { $grid }, Esc annulla)
draw-variant-rectangle-corners = Due angoli
draw-variant-rectangle-centre = Centro e angolo
draw-variant-rectangle-aligned = Allineato (tre punti)
draw-circle-on-a-line = i tre punti sono allineati: nessun cerchio passa per essi
draw-status-circle-two-start = Disegno di un cerchio per due punti: fai clic su un estremo di un diametro (griglia { $grid }, Esc annulla)
draw-status-circle-two = Disegno di un cerchio di diametro { $diameter }: fai clic sull'altro estremo del diametro (griglia { $grid }, Esc annulla)
draw-status-circle-three-start = Disegno di un cerchio per tre punti: fai clic su un primo punto del cerchio (griglia { $grid }, Esc annulla)
draw-status-circle-three-second = Disegno di un cerchio per tre punti: fai clic su un secondo punto del cerchio (griglia { $grid }, Esc annulla)
draw-status-circle-three = Disegno di un cerchio di raggio { $radius }: fai clic su un terzo punto del cerchio (griglia { $grid }, Esc annulla)
draw-variant-circle-centre = Centro e raggio
draw-variant-circle-two = Due punti (un diametro)
draw-variant-circle-three = Tre punti
draw-no-size-arc = l'arco non avrebbe dimensione: gira a partire dall'inizio
draw-status-arc-centre-start = Disegno di un arco dal centro: fai clic sul centro (griglia { $grid }, Esc annulla)
draw-status-arc-centre-radius = Disegno di un arco di raggio { $radius }: fai clic sul suo inizio (griglia { $grid }, Esc annulla)
draw-status-arc-centre = Disegno di un arco di raggio { $radius }, { $angle }°: fai clic sulla sua fine, o digita <angolo; gira nell'altro verso per l'altro lato (griglia { $grid }, Esc annulla)
draw-status-polygon-side-start = Disegno di un poligono da un lato: fai clic sul centro (griglia { $grid }, Esc annulla)
draw-status-polygon-side = Disegno di un poligono di { $flats } in chiave: fai clic sul punto medio di un lato (griglia { $grid }, Esc annulla)
draw-status-polygon-edge-start = Disegno di un poligono su un bordo: fai clic su un estremo di un lato (griglia { $grid }, Esc annulla)
draw-status-polygon-edge = Disegno di un poligono, lato { $length }: fai clic sull'altro estremo del lato; il poligono sta alla sua sinistra (griglia { $grid }, Esc annulla)
draw-variant-arc-three = Tre punti
draw-variant-arc-centre = Centro, inizio, fine
draw-variant-polygon-corner = Centro e vertice
draw-variant-polygon-side = Centro e lato
draw-variant-polygon-edge = Su un bordo
draw-status-spline-start = Disegno di una spline di { $count ->
        [one] { $count } punto
       *[other] { $count } punti
    }: fai clic sul primo punto (griglia { $grid }, Esc annulla)
draw-status-spline = Disegno di una spline di { $count ->
        [one] { $count } punto
       *[other] { $count } punti
    }: fai clic sul punto successivo; la curva passa per ognuno. Enter o doppio clic la termina aperta, il primo punto la chiude, Backspace rimuove l'ultimo punto (griglia { $grid }, Esc annulla)
draw-spline-too-short = una spline richiede almeno 2 punti
modify-mirror-pick = Specchia: clicca le forme disegnate da specchiare (Ctrl aggiunge), poi l'asse (Esc termina)
modify-mirror-first = Specchia: clicca il primo punto dell'asse, o una linea retta da usare come asse (Esc termina)
modify-mirror-second = Specchia: clicca il secondo punto dell'asse; Maiusc lo tiene a passi di 15° (Esc termina)
modify-mirror-keep = Mantieni l'originale
modify-mirror-keep-hover = Attivo: aggiunge copie speculari, e unisce una metà con le estremità sull'asse alla sua immagine in una sola forma. Disattivo: capovolge le forme sul posto
modify-scale-pick = Scala: clicca ciò che va scalato (Ctrl aggiunge), poi due punti (Esc termina)
modify-scale-first = Scala: clicca il primo punto di una lunghezza; resta dov'è (Esc termina)
modify-scale-second = Scala: clicca il secondo punto della lunghezza; Maiusc la tiene a passi di 15° (Esc torna indietro)
modify-scale-third = Scala: scrivi la nuova lunghezza e premi Invio, o clicca un punto a quella distanza dal primo (Esc torna indietro)
modify-scale-done = Scalato ×{ $factor }.
draw-no-size-slot-arc = l'asola ad arco non avrebbe larghezza, sarebbe più larga di quanto il raggio permette, o le sue estremità si toccherebbero
draw-status-slot-arc-start = Disegno di un'asola ad arco: fai clic sul centro del suo arco (griglia { $grid }, Esc annulla)
draw-status-slot-arc-radius = Disegno di un'asola ad arco di raggio { $radius }: fai clic sull'inizio della sua linea mediana (griglia { $grid }, Esc annulla)
draw-status-slot-arc-end = Disegno di un'asola ad arco di raggio { $radius }, { $angle }°: fai clic sulla fine, o digita <angolo; gira nell'altro verso per l'altro lato (griglia { $grid }, Esc annulla)
draw-status-slot-arc = Disegno di un'asola ad arco larga { $width }: clicca per darne la larghezza, o digitala (griglia { $grid }, Esc annulla)
draw-variant-slot-straight = Dritta (due estremità)
draw-variant-slot-arc = Ad arco (centro, estremità, larghezza)

# Drawing a polyline's tangent arcs (docs/specs/polyline-tangent-arcs.md).
draw-status-polyline-arc = Disegno di una polilinea, { $count ->
        [one] { $count } punto
       *[other] { $count } punti
    }, un arco di raggio { $radius } che prosegue dall'ultimo lato: clicca dove finisce; L disegna linee, Invio o doppio clic la termina aperta, il primo punto la chiude, Backspace toglie l'ultimo punto (griglia { $grid }, Esc annulla)
draw-status-polyline-arc-first = Disegno di una polilinea, { $count ->
        [one] { $count } punto
       *[other] { $count } punti
    }: un arco ha bisogno di un lato da cui proseguire, quindi il primo lato è una linea (inizia sull'estremità di una forma per proseguirla); clicca il punto successivo (griglia { $grid }, Esc annulla)
