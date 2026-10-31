# The header: the top bar and the ribbon of grouped buttons (crates/chipload-gui/src/app/header.rs).

# Export's hover text. $count is the number of parts that are cut (always more than one),
# $part the name of the part the preview shows.
header-export-hover = Guardar el código G comprobado
header-export-hover-parts = Guardar el código G comprobado de cada pieza que se corta ({ $count } piezas)
header-export-hover-parts-shown = Guardar el código G comprobado de cada pieza que se corta ({ $count } piezas); la vista previa solo muestra "{ $part }"
header-export-generating-hover = Guardar el código G cuando esté calculado
header-export-first-hover = Calcula primero

# The top bar's icon buttons.
header-open-hover = Abrir un proyecto
header-save-hover = Guardar el proyecto
header-undo = Deshacer
header-redo = Rehacer
header-help = Ayuda
# The colour theme button: what is set now, a click goes on to the next.
header-theme-system-hover = Tema de color: según el sistema. Haz clic para el siguiente.
header-theme-light-hover = Tema de color: claro. Haz clic para el siguiente.
header-theme-dark-hover = Tema de color: oscuro. Haz clic para el siguiente.

# The tab with the open project's file name.
header-new-project = Proyecto nuevo
header-tab-unsaved-hover =
    { $path }
    Cambios sin guardar
header-tab-not-saved-hover = Aún no guardado

# The ribbon's groups: their captions (shown in capitals) open the menu of the same name.
header-group-file = Archivo
header-group-draw = Dibujar
header-group-operation = Operación
header-group-library = Biblioteca
header-group-view = Vista
header-group-hover = Todo lo de { $group }

# FILE: short words under icons.
header-new = Nuevo
header-new-hover = Proyecto nuevo
header-open = Abrir
header-import = Importar
header-import-hover = Proyecto nuevo a partir de un DXF o SVG
header-save = Guardar

# DRAW: short words under icons, and what each does.
header-circle = Círculo
header-circle-hover = Dibujar un círculo: centro y luego un punto en él
header-rectangle = Rectángulo
header-rectangle-hover = Dibujar un rectángulo: dos esquinas opuestas
header-polyline = Polilínea
header-polyline-hover = Dibujar una polilínea: haz clic en los puntos
header-text = Texto
header-text-hover = Escribir un texto: haz clic donde empieza y luego escríbelo en Propiedades
header-frame = Marco
header-frame-hover = Dibujar un marco para un letrero: alrededor del texto o las formas seleccionados, o a un tamaño, con bordes
header-image = Imagen
header-image-hover = Vectorizar una imagen en formas, para tallado en V o seguir trayectoria
header-add-dxf = Añadir DXF
header-add-dxf-hover = Un DXF o SVG en este proyecto, junto a lo que ya hay: colocado y actualizado por separado
# "FILE > Import": the ribbon group FILE and its Import button.
header-add-dxf-first-hover = Primero importa un DXF o SVG (ARCHIVO > Importar)
header-import-image-menu = Importar imagen…
header-add-dxf-menu = Añadir DXF o SVG al proyecto…

# OPERATION: "Follow" is the short word under the icon for a Follow path operation.
header-follow = Seguir
# $operation is what the operation does (a hover text of its own).
header-needs-selection-hover = { $operation }. Selecciona formas primero.
header-select-shapes-first = Selecciona formas primero
header-new-profile = Nuevo contorneado
header-new-pocket = Nueva cajera
header-new-vcarve = Nuevo tallado en V
header-new-follow-path = Nuevo seguir trayectoria
header-new-holes = Nuevos taladros
header-new-surface = Nuevo planeado

# LIBRARY
header-tools = Herramientas
header-tools-hover = Tu biblioteca de herramientas, en Este equipo en el Navegador
header-machines = Máquinas
header-machines-hover = Tus perfiles de máquina, en Este equipo en el Navegador
header-tool-library = Biblioteca de herramientas

# VIEW
header-measure = Medir
header-measure-hover = Dos clics dan la distancia, ΔX, ΔY y el ángulo. Lo medido se muestra en la barra del lienzo.

# Generate and Export at the ribbon's right end.
header-export = Exportar
header-cancel = Cancelar
header-cancel-hover = Detener el cálculo
header-generate = Calcular
header-generate-hover = Planificar las trayectorias, escribir y comprobar el código G
header-arc = Arco
header-arc-hover = Dibujar un arco: inicio, fin y un punto por el que pasa
header-slot = Ranura
header-slot-hover = Dibujar una ranura: los centros de sus extremos y luego su ancho
header-polygon = Polígono
header-polygon-hover = Dibujar un polígono regular: el centro y luego un vértice (el número de lados en la barra de estado)
header-group-modify = Modificar
header-offset = Desfase
header-offset-hover = Una forma a distancia de las seleccionadas: vaya al lado deseado, haga clic o escriba la distancia
header-fillet = Redondeo
header-fillet-hover = Redondear una esquina de la forma seleccionada con un arco: haga clic en la esquina; Mayús+clic redondea todas
header-chamfer = Chaflán
header-chamfer-hover = Cortar en recto una esquina de la forma seleccionada: haga clic en la esquina; Mayús+clic corta todas
header-trim = Recortar
header-trim-hover = Quitar el trozo de una forma dibujada bajo el cursor, hasta donde la cruzan otras formas: haga clic en el trozo
header-extend = Alargar
header-extend-hover = Alargar el extremo recto de una forma abierta hasta la forma siguiente: haga clic cerca del extremo
header-break = Partir
header-break-hover = Partir una forma dibujada en el punto pulsado: una abierta en dos, una cerrada se abre ahí
header-union = Unir
header-union-hover = Unir las formas cerradas seleccionadas en una: todo lo que cubre cualquiera de ellas
header-subtract = Restar
header-subtract-hover = Recortar las formas cerradas seleccionadas de la forma que pulse a continuación
header-intersect = Intersecar
header-intersect-hover = Conservar lo que cubren todas las formas cerradas seleccionadas
header-array = Matriz
header-array-hover = Copias de las formas dibujadas seleccionadas en cuadrícula: columnas, filas y separación en Selección
header-polar-array = Matriz polar
header-polar-array-hover = Copias de las formas dibujadas seleccionadas alrededor de un centro que pulse: número y ángulo en Selección
