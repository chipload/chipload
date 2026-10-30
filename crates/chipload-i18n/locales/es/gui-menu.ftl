# The menu bar and its menus (crates/chipload-gui/src/app/menu.rs).

menu-language = Idioma
menu-language-system = Sistema
menu-language-system-hover = El idioma del sistema operativo si Chipload lo tiene; si no, inglés
menu-language-review = { $language } (falta revisión)
menu-language-review-hover = Traducido, pero aún no lo ha leído un hablante nativo que mecanice. Las correcciones son bienvenidas: consulta la guía del usuario, Idiomas.

# File menu.
menu-new-project = Proyecto nuevo
menu-open = Abrir…
menu-open-recent = Abrir reciente
# Empties the list of recent projects.
menu-clear-list = Vaciar lista
menu-import-dxf = Importar DXF o SVG…
menu-import-image = Importar imagen…
menu-import-image-hover = Vectorizar una imagen en formas, para tallado en V
menu-open-path = Abrir ruta…
menu-import-image-path = Importar imagen desde ruta…
menu-several-dxfs = Varios archivos de dibujo: actualiza uno desde el Navegador (clic derecho en él)
menu-add-dxf = Añadir DXF o SVG al proyecto…
menu-add-dxf-hover = Un DXF o SVG en este proyecto, junto a lo que ya hay: colocado y actualizado por separado
menu-add-dxf-path = Añadir DXF o SVG desde ruta…
menu-save = Guardar
menu-save-as = Guardar como…
menu-save-path = Guardar en ruta…
menu-export = Exportar código G…
menu-export-path = Exportar código G a ruta…
menu-export-dxf = Exportar dibujo como DXF…
menu-export-dxf-hover = Las formas que muestra el lienzo, en sus capas, para un láser, un programa CAD o un presupuesto
menu-construction = Auxiliar
menu-construction-hover = Geometría auxiliar: se muestra punteada y sirve para forzar el cursor, pero nunca se corta. Para líneas y círculos guía
menu-toggle-construction = Geometría auxiliar sí o no
menu-quit = Salir

# Edit: undo, and the selected drawn shapes.
menu-undo = Deshacer
menu-redo = Rehacer
menu-only-drawn = Solo formas dibujadas; las importadas solo cambian con su archivo
menu-copy = Copiar
menu-paste = Pegar
menu-mirror-left-right = Simetría izquierda–derecha
menu-mirror-up-down = Simetría arriba–abajo
menu-mirror-hover = Voltear las formas dibujadas seleccionadas respecto al centro de su recuadro
menu-delete-shapes = Eliminar formas
menu-delete-shapes-hover = Elimina las formas dibujadas seleccionadas; las importadas solo cambian con su archivo
# A selector: the selection written as JSON, for scripts.
menu-copy-selector = Copiar como selector

# Job menu.
menu-cancel-generation = Cancelar cálculo
menu-generate = Calcular

# View menu.
menu-fit-view = Ajustar vista
menu-top-view = Vista superior
menu-measure = Medir
menu-measure-hover = Dos clics dan la distancia, ΔX, ΔY y el ángulo
menu-3d-view = Vista 3D
menu-z-scale = Escala Z en 3D
menu-show-grid = Mostrar cuadrícula
menu-show-grid-hover = Líneas en la cuadrícula de ajuste, una más marcada cada diez
menu-show-legend = Mostrar leyenda
menu-show-browser = Mostrar Navegador
menu-show-properties = Mostrar Propiedades
menu-fold-panels = Plegar ambos paneles
menu-open-panels = Abrir ambos paneles
menu-panels-hover = El lienzo más grande: el Navegador y Propiedades plegados en franjas
menu-auto-update = Actualizar trayectoria automáticamente
menu-auto-update-hover = Calcular solo, medio segundo después de cada cambio
menu-show-toolpath = Mostrar trayectoria
menu-show-toolpath-hover = Oculta la trayectoria para ver el dibujo o el material simulado
menu-show-drawing = Mostrar dibujo
menu-show-drawing-hover = Oculta el dibujo para ver solo el material o la trayectoria
menu-show-images = Mostrar imágenes
menu-show-images-hover = Las imágenes vectorizadas en formas, tenues sobre el material
# The stock: the material being cut, as the simulation shows it.
menu-simulate = Simular
menu-simulation-hint = Muestra el material como lo deja el programa: cada movimiento cortado con la forma de la herramienta (más oscuro es más profundo; rojo es por debajo del material). Solo visualización.
menu-units = Unidades
menu-units-mm-hover = Longitudes en mm, avances en mm/min
# The unit's name, as a choice next to "mm".
menu-units-inch = pulgadas
menu-units-inch-hover = Longitudes en pulgadas, avances en in/min; los proyectos siguen en mm
menu-theme = Tema de color
menu-theme-system = Sistema
menu-theme-system-hover = Claro u oscuro, según el sistema operativo
menu-theme-light = Claro
menu-theme-light-hover = Paneles y lienzo claros
menu-theme-dark = Oscuro
menu-theme-dark-hover = Paneles y lienzo oscuros

# Draw menu.
menu-draw-circle = Círculo
menu-draw-rectangle = Rectángulo
menu-draw-polyline = Polilínea
menu-draw-text = Texto
menu-draw-text-hover = Escribe un texto y luego haz clic donde empieza

# Help menu.
menu-user-guide = Guía del usuario
menu-shortcuts = Atajos de teclado
menu-about = Acerca de y licencias

# The status bar under the canvas. What the toolpath is of, with several parts:
menu-status-part-of = pieza { $part } de { $count }
menu-status-all-parts = todas las piezas
# $time: how long the program runs, e.g. "12 min 30 s".
menu-status-cycle = ciclo { $time }
menu-status-time-hover = Tiempo de ciclo estimado (el informe tiene los detalles)
menu-status-active-part = pieza activa: { $name }
menu-status-active-part-hover = Las operaciones nuevas van a la pieza activa (Navegador, clic derecho en una pieza)
menu-status-units-hover = Las longitudes se muestran en estas unidades (Vista > Unidades)
# $step: the grid's step with its unit, e.g. "10 mm".
menu-status-grid = cuadrícula { $step }
menu-status-grid-hover = El paso de la cuadrícula: al dibujar, los clics se ajustan a ella (G la oculta)
menu-status-post-hover = El postprocesador (Máquina)
# Pieces in all, from every part that is cut.
menu-status-pieces = { $count ->
    [one] { $count } unidad
   *[other] { $count } unidades
}
menu-status-copies = { $count ->
    [one] { $count } copia
   *[other] { $count } copias
}
# The cursor in program coordinates (from the work zero), after the drawing's X and Y.
menu-status-work = (trabajo X { $x }  Y { $y })
# $target: what the cursor snapped to, e.g. "end of #4".
menu-snapped-to = — ajustado a { $target }
menu-generating = Calculando…
# $snapped: empty, or "— snapped to …" with a space before it.
menu-dragging = Arrastrando a ({ $x }, { $y }){ $snapped } (Alt: sin ajuste; en un segmento, Ctrl inserta un punto y Shift lo curva)
menu-status-hint = Clic en un contorno para seleccionarlo · Ctrl+clic añade o quita · clic derecho selecciona su capa · Esc borra la selección
menu-status-simulate-hint = Simular: arrastrar desplaza (3D: gira) · clic en el número de una operación la abre · F6 vuelve a Editar
menu-join-ends = Unir extremos…
menu-join-ends-hover = Unir en una las polilíneas dibujadas abiertas cuyos extremos casi se tocan: las seleccionadas, o todas si no hay ninguna seleccionada
menu-join-ends-disabled-hover = No hay formas dibujadas. Las formas de un DXF se pueden unir una vez descompuesto (clic derecho sobre él en el Navegador).
menu-draw-arc = Arco
menu-draw-slot = Ranura
menu-draw-polygon = Polígono
menu-polygon-sides = Lados
menu-modify-offset = Desfase
menu-modify-fillet = Redondeo
menu-modify-chamfer = Chaflán
menu-modify-trim = Recortar
menu-modify-extend = Alargar
menu-modify-break = Partir
menu-modify-union = Unir
menu-modify-subtract = Restar
menu-modify-intersect = Intersecar
menu-modify-array = Matriz
menu-modify-polar-array = Matriz polar
