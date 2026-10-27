# Drawing shapes with the mouse: the status bar while drawing, and snapping
# (crates/chipload-gui/src/draw.rs). Esc, Enter and Backspace are keyboard keys; the
# grid is the step clicks are rounded to, e.g. "1 mm".

draw-status-circle-start = Dibujando un círculo: haz clic en el centro (cuadrícula { $grid }, Esc cancela)
draw-status-circle = Dibujando un círculo de radio { $radius }: haz clic en un punto del círculo (cuadrícula { $grid }, Esc cancela)
draw-status-rectangle-start = Dibujando un rectángulo: haz clic en una esquina (cuadrícula { $grid }, Esc cancela)
# The rectangle's size so far: along X, then along Y.
draw-status-rectangle = Dibujando un rectángulo de { $length } x { $width }: haz clic en la esquina opuesta (cuadrícula { $grid }, Esc cancela)
draw-status-polyline-start = Dibujando una polilínea de { $count ->
        [one] { $count } punto
       *[other] { $count } puntos
    }: haz clic en el primer punto (cuadrícula { $grid }, Esc cancela)
draw-status-polyline = Dibujando una polilínea de { $count ->
        [one] { $count } punto
       *[other] { $count } puntos
    }: haz clic en el siguiente punto; Enter o doble clic la termina abierta, el primer punto la cierra, Backspace quita el último punto (cuadrícula { $grid }, Esc cancela)

# Why a click did nothing.
draw-no-size-circle = el círculo no tendría tamaño
draw-no-size-rectangle = el rectángulo no tendría tamaño
draw-polyline-too-short = una polilínea necesita al menos 2 puntos

# The point the cursor snapped to, on contour #4.
draw-snap-end = extremo de #{ $contour }
draw-snap-centre = centro de #{ $contour }
draw-snap-middle = punto medio de #{ $contour }
draw-snap-intersection = intersección de #{ $contour } y #{ $other }
draw-snap-intersection-self = intersección en #{ $contour }
draw-snap-quadrant = cuadrante de #{ $contour }
draw-snap-perpendicular = perpendicular a #{ $contour }
draw-snap-tangent = tangente a #{ $contour }
draw-snap-nearest = sobre #{ $contour }

# A value typed while drawing that can't be used.
draw-entry-unreadable = no se puede leer: escriba x,y  @dx,dy  una longitud, o longitud<ángulo
draw-entry-needs-point = el primer punto: escriba x,y
draw-entry-no-direction = aleje el cursor del último punto para dar la dirección
draw-status-arc-start = Dibujando un arco: haga clic donde empieza (rejilla { $grid }, Esc cancela)
draw-status-arc-end = Dibujando un arco, cuerda { $length }: haga clic donde termina (rejilla { $grid }, Esc cancela)
draw-status-arc = Dibujando un arco de radio { $radius }: haga clic en un punto por el que pasa, o escriba su radio (rejilla { $grid }, Esc cancela)
draw-status-slot-start = Dibujando una ranura: haga clic en el centro de un extremo (rejilla { $grid }, Esc cancela)
draw-status-slot-end = Dibujando una ranura, { $length } entre los extremos: haga clic en el centro del otro extremo (rejilla { $grid }, Esc cancela)
draw-status-slot = Dibujando una ranura de ancho { $width }: haga clic para dar su ancho, o escríbalo (rejilla { $grid }, Esc cancela)
draw-arc-on-a-line = los tres puntos están en una línea: use la herramienta polilínea
draw-no-size-slot = la ranura no tendría longitud o ancho
draw-entry-radius-too-small = el radio debe ser al menos la mitad de la distancia entre los extremos
draw-entry-no-side = mueva el cursor hacia el lado al que debe curvarse el arco
draw-status-polygon-start = Dibujando un polígono: haga clic en el centro (rejilla { $grid }, Esc cancela)
draw-status-polygon = Dibujando un polígono de radio { $radius }: haga clic en un vértice (rejilla { $grid }, Esc cancela)
