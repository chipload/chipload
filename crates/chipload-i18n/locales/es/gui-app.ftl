# The main window: opening, importing, saving and exporting, parts, rest passes, the
# selection (crates/chipload-gui/src/app/mod.rs).

# The buttons of a folded side panel; P is the key that folds and unfolds both panels.
app-show-browser-hover = Mostrar el Navegador (P: ambos paneles)
app-show-properties-hover = Mostrar Propiedades (P: ambos paneles)
# The heading of the right-hand panel.
app-properties = Propiedades

app-start-hint = Importa un DXF o SVG (Ctrl+I) para empezar un proyecto, o abre uno (Ctrl+O).

app-cannot-open = No se puede abrir { $file }:
app-project = Proyecto: { $file }
app-old-schema = Este archivo es de la versión de esquema { $version }; al guardar se escribe la versión { $current }.

app-new-project = Proyecto nuevo.
# $what: a machine or tool from the user's library, e.g. machine "Router".
app-started-with = Iniciado con { $what }.
app-new-project-next = Dibuja formas con el menú Dibujar, o trae un dibujo con Archivo > Importar DXF al proyecto. Revisa el material en bruto, la máquina y la herramienta en Ajustes del trabajo.
app-units-missing = { $file } no indica qué unidades usa; elige mm o pulgadas.
app-px-missing = { $file } solo da su tamaño en píxeles; elige sus píxeles por pulgada.
app-cannot-import = No se puede importar { $file }:
app-new-project-from = Proyecto nuevo a partir de { $file } ({ $shapes } formas).
app-import-next = Selecciona contornos y luego Nuevo contorneado o Nueva cajera. Revisa el material en bruto, la máquina y la herramienta en Ajustes del trabajo.

# Followed by the list of problems.
app-project-incomplete = El proyecto no está completo:
app-lost-geometry = Algunas operaciones perdieron su geometría: vuelve a elegirla
app-could-not-update-drawing = No se pudo actualizar el dibujo:
app-could-not-add = No se pudo añadir { $file }:
app-could-not-move-drawing = No se pudo mover el dibujo:

# The distance between copies of a part; $dy and $gap carry their unit.
app-pitch-set = Paso { $dx } × { $dy } (separación { $gap })
app-pitch-needs-plan = No se pudo fijar el paso a partir de una separación: primero hay que planificar la pieza.

app-deleted-part = { $count ->
    [0] Pieza { $name } eliminada (Ctrl+Z la recupera)
    [one] Pieza { $name } y su operación eliminadas (Ctrl+Z las recupera)
   *[other] Pieza { $name } y sus { $count } operaciones eliminadas (Ctrl+Z las recupera)
}
app-part-needed = Un proyecto necesita al menos una pieza

# A rest pass clears what a larger tool left in a pocket.
app-could-not-add-rest-pass = No se pudo añadir una pasada de restos: { $error }
app-rest-with-corners = Esta cajera tiene esquinas dogbone o T-bone, y todavía no se pueden combinar con pasadas de restos. Las esquinas ya llegan adonde llegaría una pasada de restos.
# $tool: a tool number such as T3.
app-rest-tool-missing = La herramienta { $tool } no está en el proyecto: primero asigna una herramienta a la cajera.
app-rest-no-smaller = No hay ninguna fresa más pequeña que { $tool } ({ $diameter }) en el proyecto ni en tu biblioteca de herramientas. Añade una en Herramientas y vuelve a hacer clic derecho en la cajera.
app-now-rest-pass = La operación { $op } es ahora una pasada de restos de la operación { $parent }; se corta después de la serie de cajeras a la que pertenece

app-generate-before-export = Calcula (F5) antes de exportar el código G.
app-planning-then-export = Planificando todas las piezas y luego exportando
app-could-not-export = No se pudo exportar { $path }:
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
app-selection-dxf = DXF { $id }
app-selection-image = imagen { $id }
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
