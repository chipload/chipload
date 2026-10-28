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
modify-offset-pick = Desfase: seleccione las formas (haga clic en una; Ctrl añade), Esc termina
modify-offset-status = Desfase { $distance } { $side }: haga clic, o escriba la distancia (Alt: fuera de la rejilla, Esc termina)
modify-offset-done = { $count ->
    [one] Se añadió un desfase
   *[other] Se añadieron { $count } desfases
} a { $distance }.
modify-side-outside = por fuera
modify-side-inside = por dentro
modify-side-left = a la izquierda
modify-side-right = a la derecha
modify-corners-round = Esquinas redondeadas
modify-corners-sharp = Esquinas vivas
modify-corners-hover = Redondeadas: el desfase exacto, arcos en las esquinas. Vivas: los lados prolongados hasta encontrarse, como un marco.
modify-offset-count = Cantidad
modify-corner-pick = Seleccione una forma dibujada (haga clic en ella) y luego sus esquinas; Esc termina
modify-fillet-status = Redondeo: haga clic en una esquina; Mayús+clic redondea todas (Esc termina)
modify-chamfer-status = Chaflán: haga clic en una esquina; Mayús+clic corta todas (Esc termina)
modify-fillet-radius = Radio
modify-chamfer-distance = Distancia
modify-trim-status = Recortar: haga clic en el trozo de una forma dibujada que se quita; se muestra antes del clic (Esc termina)
modify-extend-status = Alargar: haga clic cerca del extremo de una forma abierta para alargarla hasta la forma siguiente (Esc termina)
modify-break-status = Partir: haga clic en una forma dibujada donde deba partirse (Esc termina)
modify-extend-closed = una forma cerrada no tiene extremo que alargar
modify-combine-pick = Seleccione primero dos o más formas dibujadas cerradas
modify-subtract-status = Restar: haga clic en la forma de la que recortar las seleccionadas (Esc termina)
modify-array-pick = Seleccione primero las formas dibujadas que copiar
modify-array-status = Matriz: ajuste las copias en Selección y luego Crear (Esc termina)
modify-polar-status = Matriz polar: haga clic en el centro, ajuste las copias en Selección y luego Crear (Esc termina)
array-columns = Columnas
array-rows = Filas
array-dx = Separación de columnas
array-dy = Separación de filas
array-count = Número
array-center = Centro
array-angle = Ángulo
array-angle-hover = 360° reparte las copias por igual alrededor; menos pone la primera y la última en sus extremos; negativo gira en sentido horario
array-rotate = Girar las copias
array-create = Crear
