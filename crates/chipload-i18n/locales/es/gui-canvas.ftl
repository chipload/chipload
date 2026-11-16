# The drawing canvas: its right-click menu, hover texts and notices (crates/chipload-gui/src/app/canvas.rs).

# The right-click menu. A ring's start: where a profile or pocket lap begins.
canvas-reset-start = Restablecer inicio
canvas-reset-start-disabled-hover = Este anillo empieza donde lo eligió Chipload
canvas-turn-axis = ⟲ Eje de volteo
# $id: the contour's number.
canvas-select-contour = Seleccionar #{ $id }
canvas-select-layer = Seleccionar capa { $layer }
canvas-select-similar = Seleccionar similares
canvas-select-similar-hover = Círculos del mismo diámetro en cualquier capa, o formas del mismo tamaño en esta capa. Ctrl: añadir a la selección
canvas-select-inside = Seleccionar interior
canvas-select-inside-hover = Las formas cerradas dentro de esta: cajeados y agujeros de una pieza. Ctrl: añadir a la selección
canvas-right-click-hint = Haz clic derecho en un contorno para seleccionar su capa.

# Hovering a ring's start marker.
canvas-start-moved-hover = Inicio de este anillo: arrástralo a lo largo del anillo; clic derecho para restablecerlo
canvas-start-hover = Inicio de este anillo: arrástralo a lo largo del anillo para moverlo
canvas-tab-hover = Puente: arrástralo a lo largo del anillo; clic derecho para quitarlo o repartir por igual los puentes del anillo
canvas-tab-hover-placed = Puente colocado a mano: arrástralo a lo largo del anillo; clic derecho para quitarlo o repartir por igual los puentes del anillo
# A dropped tab or ring start that the planner refuses goes back; $reason says why.
canvas-drop-refused = No movido: { $reason }
canvas-tab-remove = Quitar el puente
canvas-tab-even = Volver a colocar los puentes de este anillo
canvas-tab-add = Añadir un puente aquí
canvas-corner-hover = Esquina en { $at }, { $angle }°: { $gets }. Haz clic para elegir aparte
canvas-corner-hover-own = Esquina en { $at }, { $angle }°: { $gets }, elegida aparte. Haz clic para cambiarla
canvas-corner-hover-unreached = Esquina en { $at }, { $angle }°: { $gets }, pero la herramienta no llega a ella, así que queda redondeada. Haz clic para elegir
canvas-corner-as-operation = Como la operación ({ $gets })
canvas-corner-round = Redondeada
canvas-corner-dogbone = Dogbone
canvas-corner-tbone-longer = T-bone a lo largo de la pared más larga
canvas-corner-tbone-shorter = T-bone a lo largo de la pared más corta
canvas-corner-gets-round = redondeada
canvas-corner-gets-dogbone = un dogbone
canvas-corner-gets-tbone-longer = un T-bone a lo largo de la pared más larga
canvas-corner-gets-tbone-shorter = un T-bone a lo largo de la pared más corta

# Hovering plunges: "Operation 2 (Pocket 1): plunges 3, 13, 23 of 30". $operation is the
# operation's label, $numbers the list of plunge numbers, $total how many it has.
canvas-plunges = { $operation }: { $count ->
        [one] penetración
       *[other] penetraciones
    } { $numbers } de { $total }
canvas-rejected-toolpath = Trayectoria rechazada

# The notice at the canvas's top.
canvas-updating-toolpath = Actualizando la trayectoria…
canvas-toolpath-out-of-date = Trayectoria desactualizada
# A button: generate the toolpath again.
canvas-generate = Calcular
canvas-generate-hover = El proyecto cambió desde que se calculó (F5)
canvas-updating-simulation = Actualizando la simulación…

# The legend in the bottom left corner.
canvas-legend = Leyenda
canvas-legend-hover = Qué muestra el lienzo
canvas-show-toolpath = Mostrar trayectoria
canvas-show-toolpath-hover = Oculta la trayectoria para ver el dibujo o el material simulado

## La brújula (docs/specs/compass.md).
compass-menu-item = Brújula
compass-hover = Mover a lo largo de una flecha, girar con el anillo, arrastrar el centro a un punto; arrastrar con el botón derecho para copiar o hacer una matriz; Espacio la libera para colocarla
compass-select-first = Selecciona primero formas dibujadas, un grupo, una imagen, un texto, un modelo 3D o un dibujo entero, luego Q
compass-unlocked = Brújula libre: arrástrala a su sitio, Espacio la fija
compass-locked = Brújula fija: arrastrar mueve la selección
compass-typed-unreadable = No se puede leer: escribe una longitud, un ángulo en grados, o ΔX y ΔY
compass-move-here = Mover aquí
compass-turn-here = Girar aquí
compass-copy-here = Copiar aquí
compass-linear-array = Matriz lineal…
compass-circular-array = Matriz circular…
compass-copies-drawn-only = Solo se pueden copiar formas dibujadas
compass-status = Brújula: arrastra una flecha, el anillo o el centro; botón derecho para copiar; Espacio la libera; Q o Esc la oculta
compass-status-arrow = Arrastrar mueve a lo largo de esta flecha; doble clic para escribir la distancia; botón derecho para copiar o hacer una matriz lineal; clic derecho para mover a un punto
compass-status-ring = Arrastrar gira alrededor del centro en pasos de 15° (Alt: 0,1°); doble clic para escribir el ángulo; botón derecho para una matriz circular
compass-status-centre = Arrastrar mueve libremente, el centro se ajusta a puntos; doble clic para escribir ΔX y ΔY; clic derecho para mover a un punto
compass-status-place = Arrastrar pone la brújula sobre un punto; Espacio la fija
compass-status-aim = Arrastrar gira los ejes de la brújula, o alinea uno con una línea; Espacio la fija
compass-to-point = A un punto
compass-to-point-hover = Haz clic en un punto a continuación: la selección se mueve allí (con forzado como al arrastrar)
compass-to-centre = Al centro
compass-to-centre-hover = Haz clic en una forma a continuación: la selección va a su centro (el de un círculo o un arco, si no el centro de su caja)
compass-no-centre-here = Ahí no hay ninguna forma: haz clic en una forma para ir a su centro (Esc cancela)
compass-status-pick-point = Haz clic en el punto al que mover la selección (Alt: sin forzado; Esc cancela)
compass-status-pick-point-axis = Haz clic en un punto: la selección va hasta él a lo largo de la flecha (Alt: sin forzado; Esc cancela)
compass-status-pick-centre = Haz clic en una forma: la selección va a su centro (Esc cancela)
compass-status-pick-centre-axis = Haz clic en una forma: la selección va a lo largo de la flecha hasta su centro (Esc cancela)
compass-value-distance = Distancia
compass-value-angle = Ángulo
compass-value-delta = ΔX ΔY
# A part cut elsewhere than drawn: the label on the dashed box round its shapes
# (docs/specs/part-placement.md §4).
canvas-not-cut-here = { $part }: no se corta aquí
