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
