# The drawing canvas: its right-click menu, hover texts and notices (crates/chipload-gui/src/app/canvas.rs).

# The right-click menu. A ring's start: where a profile or pocket lap begins.
canvas-reset-start = Restablecer inicio
canvas-reset-start-disabled-hover = Este anillo empieza donde lo eligió Chipload
# $id: the contour's number.
canvas-select-contour = Seleccionar #{ $id }
canvas-select-layer = Seleccionar capa { $layer }
canvas-right-click-hint = Haz clic derecho en un contorno para seleccionar su capa.

# Hovering a ring's start marker.
canvas-start-moved-hover = Inicio de este anillo: arrástralo a lo largo del anillo; clic derecho para restablecerlo
canvas-start-hover = Inicio de este anillo: arrástralo a lo largo del anillo para moverlo
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
