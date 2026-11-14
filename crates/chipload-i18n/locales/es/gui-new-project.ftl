# The new project dialog (crates/chipload-gui/src/app/new_project.rs, docs/specs/new-project-dialog.md).

new-project-title = Proyecto nuevo
new-project-material = Material
new-project-material-none = Ninguno
new-project-thickness = Espesor
# The board: fitted round the drawing with a margin, or a size.
new-project-board = Tablero
new-project-fit = Ajustado al dibujo, margen
new-project-size = Tamaño
new-project-z-zero = Cero Z
new-project-z-zero-hover = Dónde se fija el Z0 de la máquina: en la cara superior del tablero o en el tablero de sacrificio debajo
new-project-z-top = Cara superior del tablero
new-project-z-spoilboard = Tablero de sacrificio
new-project-machine = Máquina
new-project-machine-none = Ajustes propios de Chipload (no hay máquina en tu biblioteca)
# Under the machine: its post-processor, G-code units (mm or inch) and how tools are changed.
new-project-machine-summary = { $post } · { $units } · { $change }
new-project-tools = Herramientas
new-project-search = Buscar
# Under the tool list: how many are ticked and their numbers ("T1, T2").
new-project-tools-count = { $count ->
    [0] Sin herramientas: añade una antes de generar
    [one] 1 herramienta: { $list }
   *[other] { $count } herramientas: { $list }
}
new-project-no-library-tools = Tu biblioteca de herramientas está vacía: el proyecto empieza con una fresa de 6 mm. Añade tus herramientas en Biblioteca > Herramientas.
new-project-create = Crear
