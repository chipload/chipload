# The main window: opening, importing, saving and exporting, parts, rest passes, the
# selection (crates/chipload-gui/src/app/mod.rs).

# The heading of the right-hand panel.
app-properties = Propiedades

app-start-hint = Importa un DXF o SVG (Ctrl+I) para empezar un proyecto, o abre uno (Ctrl+O).

app-cannot-open = No se puede abrir { $file }:
app-project = Proyecto: { $file }
app-old-schema = Este archivo es de la versión de esquema { $version }; al guardar se escribe la versión { $current }.

app-new-project = Proyecto nuevo.
# $what: a machine or tool from the user's library, e.g. machine "Router".
app-started-with = Iniciado con { $what }.
app-new-project-next = Dibuja formas con el menú Dibujar, o trae un dibujo con Añadir DXF o SVG. Revisa el tablero en la sujeción, la máquina en Máquina y la herramienta en Herramientas.
app-units-missing = { $file } no indica qué unidades usa; elige mm o pulgadas.
app-px-missing = { $file } solo da su tamaño en píxeles; elige sus píxeles por pulgada.
app-cannot-import = No se puede importar { $file }:
app-new-project-from = Proyecto nuevo a partir de { $file } ({ $shapes } formas).
app-import-next = Selecciona contornos y luego Nuevo contorneado o Nueva cajera. Revisa el tablero en la sujeción, la máquina en Máquina y la herramienta en Herramientas.

# Followed by the list of problems.
app-project-incomplete = El proyecto no está completo:
app-lost-geometry = Algunas operaciones perdieron su geometría: vuelve a elegirla
app-could-not-update-drawing = No se pudo actualizar el dibujo:
app-could-not-add = No se pudo añadir { $file }:
app-could-not-move-drawing = No se pudo mover el dibujo:

# The distance between a grid's cells; $dy and $gap carry their unit.
app-pitch-set = Paso { $dx } × { $dy } (separación { $gap })
app-pitch-needs-plan = No se pudo fijar el paso a partir de una separación: primero hay que planificar la pieza.
app-instance-needs-plan = No se pudo colocar la nueva instancia: primero hay que planificar la pieza.

app-deleted-part = { $count ->
    [0] Pieza { $name } eliminada (Ctrl+Z la recupera)
    [one] Pieza { $name } y su operación eliminadas (Ctrl+Z las recupera)
   *[other] Pieza { $name } y sus { $count } operaciones eliminadas (Ctrl+Z las recupera)
}
app-part-needed = Un proyecto necesita al menos una pieza
app-board-surface-stays = El planeado de todo el tablero es de la sujeción y queda el primero en ella

# A rest pass clears what a larger tool left in a pocket.
app-could-not-add-rest-pass = No se pudo añadir una pasada de restos: { $error }
app-could-not-add-floor-pass = No se pudo añadir una pasada de fondo: { $error }
app-setup-refused = No se pudieron cambiar las sujeciones: { $error }
app-dowels-placed = Agujeros de pasador recolocados, lejos de los cortes de ambas caras. El cero se movió con el primer pasador si estaba allí.
app-setup-has-operations = La sujeción tiene piezas con operaciones: muévelas antes a otra sujeción o elimínalas.
app-exported-setup = Solo la sujeción «{ $name }»: activa otra sujeción y expórtala también.
app-floor-no-endmill = No hay ninguna fresa plana en el proyecto ni en tu biblioteca de herramientas
app-rest-with-corners = Esta cajera tiene esquinas dogbone o T-bone, y todavía no se pueden combinar con pasadas de restos. Las esquinas ya llegan adonde llegaría una pasada de restos.
# $tool: a tool number such as T3.
app-rest-tool-missing = La herramienta { $tool } no está en el proyecto: primero asigna una herramienta a la cajera.
app-rest-no-smaller = No hay ninguna fresa más pequeña que { $tool } ({ $diameter }) en el proyecto ni en tu biblioteca de herramientas. Añade una en Herramientas y vuelve a hacer clic derecho en la cajera.
app-now-rest-pass = La operación { $op } es ahora una pasada de restos de la operación { $parent }; se corta después de la serie de cajeras a la que pertenece

app-generate-before-export = Calcula (F5) antes de exportar el código G.
app-planning-then-export = Planificando todas las piezas y luego exportando
app-planning-part-then-export = Planificando la pieza { $name } y exportando después
app-could-not-export = No se pudo exportar { $path }:
app-dxf-exported = Se escribió { $path }.
app-could-not-export-dxf = El dibujo no se exportó:
app-sheet-failed = Se exportó el código G, pero no se pudo escribir la hoja de preparación { $path }:
app-exported-n-files = Se exportaron { $count } archivos: entre uno y otro cambia la herramienta y fija el cero Z, manteniendo el cero X e Y.
app-sheet-for-operator = Hoja de preparación para el operario: { $path }
app-exported-files = Se exportaron { $count } archivos junto a { $path }
app-exported-files-sheet = Se exportaron { $count } archivos y la hoja de preparación { $sheet } junto a { $path }
app-exported = Exportado { $path }
app-exported-sheet = Exportados { $path } y la hoja de preparación { $sheet }

app-cannot-save-yet = El proyecto todavía no se puede guardar:
app-could-not-save = No se pudo guardar { $path }:
app-saved = Guardado { $path }

# What is selected on the canvas.
app-nothing-selected = No hay nada seleccionado.
# $count contours selected; $what says how (one of the texts below); $list their numbers.
app-selected = { $count } seleccionados ({ $what }): { $list }
app-selection-layer = capa { $layer }
app-selection-dxf = dibujo { $id }
app-selection-image = imagen { $id }
app-selection-model = modelo { $id }
app-selection-text = texto { $id }
# $source: one of the three texts above.
app-selection-left-out = { $source }, { $count } excluidos
app-selection-contours = contornos
# Open contours (not closed) are not cut.
app-open-not-cut = Abiertos (no se cortan): { $list }

# File menu, for a project started without a drawing.
app-import-dxf-into-project = Importar DXF o SVG al proyecto…
app-import-dxf-into-project-path = Importar DXF o SVG al proyecto desde ruta…
# File menu, a new version of the project's drawing.
app-update-drawing-from-dxf = Actualizar dibujo desde DXF o SVG…
app-update-drawing-from-path = Actualizar dibujo desde ruta…

# The Nest window (docs/specs/nesting.md §7)
nest-menu = Anidar…
nest-menu-hover = Anidar…: cada instancia de las piezas de la sujeción colocada en su tablero lo más junta posible, apartada por la separación; primero una vista previa, en un diálogo
nest-again = Anidar de nuevo
nest-again-hover = Anidar la sujeción de nuevo con la separación, el margen y los giros de la última vez, sin diálogo
nest-title = Anidar: { $setup }
nest-board-follows = El tablero sigue al dibujo: anidar le da este tamaño, con su esquina en el origen del dibujo.
nest-board = Tablero
nest-gap = Separación
nest-gap-hover = La menor distancia entre lo que cortan dos instancias: una franja de tablero que mantiene la plancha rígida y las pestañas sujetas
nest-margin = Margen
nest-margin-hover = El espacio que queda en el borde del tablero, para tornillos o sargentos
nest-part = Pieza
nest-count = Veces
nest-count-hover = Cuántas veces se corta la pieza: primero se añaden o quitan instancias, nunca la pieza misma
nest-turns = Giros
nest-turns-none = Como está
nest-turns-half = Medias vueltas
nest-turns-quarter = Cuartos de vuelta
nest-turns-every-15 = Cada 15°
nest-running = Anidando…
nest-stale = Ajustes cambiados: Vista previa para ver el anidado
nest-apply = Aplicar
nest-preview = Vista previa
nest-stopped-unexpectedly = El anidado se detuvo inesperadamente
