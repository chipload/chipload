# Help > Keyboard shortcuts (crates/chipload-gui/src/app/shortcuts.rs).

shortcuts-window = Atajos de teclado

# The window's group headings: menus (File, Edit, View, Help), the Generate button, and
# what is done on the canvas or while drawing a shape.
shortcuts-group-file = Archivo
shortcuts-group-edit = Editar
shortcuts-group-generate = Calcular
shortcuts-group-view = Vista
shortcuts-group-help = Ayuda
shortcuts-group-canvas = Lienzo
shortcuts-group-drawing = Al dibujar

# What each keyboard shortcut does.
shortcuts-copy-selector = Copiar la selección como selector (JSON)
shortcuts-save-as = Guardar como
shortcuts-redo = Rehacer
shortcuts-copy = Copiar las formas dibujadas seleccionadas
shortcuts-paste = Pegarlas (en el cursor, sobre el lienzo)
shortcuts-new = Proyecto nuevo
shortcuts-open = Abrir un proyecto
shortcuts-import = Proyecto nuevo a partir de un DXF o SVG
shortcuts-save = Guardar
shortcuts-export = Exportar código G (una vez calculado)
shortcuts-quit = Salir
shortcuts-undo = Deshacer
shortcuts-move-up = Subir la operación
shortcuts-move-down = Bajar la operación
shortcuts-generate = Calcular la trayectoria y el código G
shortcuts-fit = Ajustar la vista
shortcuts-top-view = Vista superior
shortcuts-grid = Mostrar u ocultar la cuadrícula
shortcuts-simulate = Simular o editar
shortcuts-measure = Medir: dos clics dan la distancia y el ángulo
shortcuts-panels = Plegar o abrir los paneles laterales, para un lienzo más grande
shortcuts-this-list = Esta lista

# Mouse actions and keys that aren't menu shortcuts: the keys column (key names as on the
# keyboard), then what they do.
shortcuts-click = Clic
shortcuts-click-select = Seleccionar un contorno
shortcuts-ctrl-click = Ctrl+clic
shortcuts-ctrl-click-what = Añadir un contorno a la selección, o quitarlo
shortcuts-right-click = Clic derecho
shortcuts-right-click-what = Seleccionar un contorno o su capa; restablecer el inicio movido de un anillo
shortcuts-scroll = Rueda
shortcuts-scroll-what = Zoom en el cursor
shortcuts-drag = Arrastrar
shortcuts-drag-what = Seleccionar con un recuadro: de izquierda a derecha lo que queda dentro, de derecha a izquierda lo que toca; Mayús añade, Ctrl añade o quita
shortcuts-pan-drag = Arrastrar con botón derecho o central, Espacio+arrastrar
shortcuts-pan-drag-what = Desplazar la vista
shortcuts-delete = Delete
shortcuts-delete-what = Eliminar lo seleccionado: formas dibujadas, un texto, una imagen o un DXF en el lienzo; una operación o una pasada de restos en el Navegador
shortcuts-escape = Escape
shortcuts-escape-clear = Borrar la selección
# A ring: a profile's closed path; its start can be dragged on the canvas.
shortcuts-drag-start = Arrastrar el inicio de un anillo
shortcuts-drag-start-what = Mover dónde empieza el anillo
shortcuts-drop = Soltar un archivo
shortcuts-drop-what = Abrir un proyecto .chipload, o empezar uno desde un .dxf o .svg
shortcuts-click-point = Colocar un punto (se ajusta a extremos, centros y puntos medios)
shortcuts-alt = Alt
shortcuts-alt-what = Mantener: sin ajuste
shortcuts-enter = Enter, doble clic
shortcuts-enter-what = Terminar una polilínea; sin herramienta, dibujar de nuevo con la última
shortcuts-backspace = Backspace
shortcuts-backspace-what = Deshacer el último punto
shortcuts-escape-stop = Dejar de dibujar
shortcuts-drag-handle = Arrastrar un tirador
shortcuts-drag-handle-what = Mover un punto, el centro o una esquina; girar un rectángulo
shortcuts-drag-middle = Arrastrar una línea de una polilínea
shortcuts-drag-middle-what = Mover ese segmento
shortcuts-ctrl-drag-middle = Ctrl+arrastrar una línea de una polilínea
shortcuts-ctrl-drag-middle-what = Insertar ahí un punto
shortcuts-shift-drag-middle = Shift+arrastrar una línea de una polilínea
shortcuts-shift-drag-middle-what = Curvar el segmento en un arco
shortcuts-draw-polyline = Dibujar una polilínea
shortcuts-draw-circle = Dibujar un círculo
shortcuts-draw-rectangle = Dibujar un rectángulo
shortcuts-type-value = Escribir un número
shortcuts-type-value-what = Un punto exacto: x,y  @dx,dy  longitud  longitud<ángulo
shortcuts-shift = Mayús
shortcuts-shift-what = Mantener: la línea en pasos de 15°
shortcuts-drag-shape = Arrastrar una forma dibujada
shortcuts-drag-shape-what = Moverla, o las formas seleccionadas con ella; se ajusta, Mayús mantiene pasos de 15°
shortcuts-orbit-drag = Arrastrar en la vista 3D
shortcuts-orbit-drag-what = Orbitar; Mayús+arrastrar desplaza
shortcuts-draw-arc = Dibujar un arco
shortcuts-draw-slot = Dibujar una ranura
shortcuts-draw-polygon = Dibujar un polígono
shortcuts-offset = Desfasar las formas seleccionadas
shortcuts-trim = Recortar formas dibujadas
shortcuts-construction = Geometría auxiliar: para las próximas formas dibujadas, o la selección
