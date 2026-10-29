# The Browser: the project as one tree of folders and rows (crates/chipload-gui/src/app/browser.rs).

# The panel's caption, shown in capitals.
browser-caption = Navegador
browser-no-project = No hay ningún proyecto abierto.
browser-new-project = Proyecto nuevo

# A row's eye (shown on the canvas or not) and tick (cut or not).
browser-eye-hide-hover = Ocultarlo en el lienzo; se sigue cortando
browser-eye-show-hover = Oculto en el lienzo (se sigue cortando): mostrarlo
browser-tick-on-hover = Se corta: desmárcalo para dejarlo fuera del programa
browser-tick-off-hover = No se corta: márcalo para cortarlo

# A project tool's link to the user's tool library, at its row's right end ($name: the
# library tool's name).
browser-link-changed = { $name } (modificada)
browser-link-missing = (no está en la biblioteca)

# The kinds of operation, and what a new one of each does.
browser-profile = Contorneado
browser-profile-hover = Una operación de contorneado para los contornos seleccionados
browser-pocket = Cajera
browser-pocket-hover = Una operación de cajera para los contornos seleccionados
browser-vcarve = Tallado en V
browser-vcarve-hover = Una fresa en V a lo largo de la línea central de los contornos cerrados seleccionados, tan profunda como para que el corte sea tan ancho como la forma
browser-follow-path = Seguir trayectoria
browser-follow-path-hover = La herramienta sobre la línea de las formas seleccionadas, abiertas o cerradas: grabado, ranuras y líneas de marcado
browser-holes = Taladros
browser-holes-hover = Los círculos seleccionados: taladrados con una broca, o mandrinados en hélice con una fresa
browser-surface = Planeado
browser-surface-hover = Todo el material en bruto rebajado en plano con una fresa ancha: aplanar el tablero de sacrificio o planear el material. No necesita selección

# Job settings and Tools.
browser-job-settings = Ajustes del trabajo
browser-job-settings-hover = Material en bruto, máquina, postprocesador y herramientas
browser-work-zero = Cero pieza
browser-work-zero-hover = Dónde está el X0 Y0 Z0 del código G. Haga clic para ajustarlo, o selecciónelo y arrástrelo en la vista.
browser-job-settings-problems-hover = Material en bruto, máquina, postprocesador y herramientas. { $count ->
        [one] { $count } problema
       *[other] { $count } problemas
    }
browser-tools = Herramientas
browser-tools-hover = Las herramientas del proyecto: haz clic para verlas todas y añadir una

# Parts: groups of operations, each cut with its copies before the next.
browser-parts = Piezas
browser-parts-hover = Cada pieza se corta con sus copias antes de la siguiente; clic derecho para una pieza nueva
browser-new-part = Pieza nueva
# At a part row's right end: the part new operations go into.
browser-part-active = activa
browser-part-active-hover = La pieza activa: aquí van las operaciones nuevas. Clic para sus copias; clic derecho para más
browser-part-hover = Clic para sus copias; doble clic para hacerla la pieza activa; clic derecho para más
# A verb: make this the active part.
browser-activate = Activar
browser-activate-hover = Las operaciones nuevas van a la pieza activa
browser-new-operation = Operación nueva
browser-select-shapes-first = Selecciona formas primero
browser-move-up = Subir
browser-move-down = Bajar
# $count: the part's operations.
browser-delete-part = { $count ->
        [0] Eliminar pieza
        [one] Eliminar pieza y su operación
       *[other] Eliminar pieza y sus { $count } operaciones
    }
browser-delete-part-disabled-hover = Un proyecto necesita al menos una pieza
browser-part-empty-active = Selecciona formas y luego añade una operación
browser-part-empty = Sin operaciones: arrastra una aquí

# An operation's row and its right-click menu.
browser-no-name = (sin nombre)
# $count: contours the operation cuts that are hidden on the canvas.
browser-op-hidden = { $count } ocultos
browser-duplicate = Duplicar
browser-add-rest-pass = Añadir pasada de restos
# A tool for a rest pass: $tool the project tool's number, $size its diameter.
browser-rest-project-tool = T{ $tool } · fresa de { $size }
browser-rest-library-tool = { $name } · fresa de { $size }
browser-from-library = De tu biblioteca de herramientas
browser-copied-into-project-hover = Se copia a las Herramientas del proyecto
browser-add-rest-pass-hover = Una herramienta más pequeña que solo quita lo que deja la herramienta de esta cajera, con sus formas y profundidades. Sigue a la cajera y se corta después de las cajeras contiguas con la misma herramienta
browser-make-rest-pass = Hacerla pasada de restos de la operación { $number }
browser-make-rest-pass-hover = Copia esa cajera: como su pasada de restos la sigue, y se corta después de las cajeras contiguas con la misma herramienta
browser-delete = Eliminar

# A rest pass's row. "Rest": a rest pass, a smaller tool after the pocket's own.
browser-rest = Restos
browser-rest-pass-hover = Una pasada de restos: quita lo que deja la herramienta anterior, con las formas y profundidades de su cajera, y se corta después de las cajeras contiguas a su cajera con la misma herramienta
browser-remove = Quitar

# An operation row's hover text: its lines.
browser-op-rest-after = restos tras T{ $tool }
browser-op-not-cut = No se corta (desmarcada)
browser-op-hidden-hover = Corta { $count ->
        [one] { $count } contorno oculto
       *[other] { $count } contornos ocultos
    }: muéstralos en Dibujo para verlos
browser-op-drag-hover = Arrastra para reordenar; clic derecho para más

# Properties: what is selected on the canvas.
browser-selection = Selección
browser-copy-as-selector = Copiar como selector
browser-copy-as-selector-hover = La selección como JSON, para una operación en un archivo de proyecto

# A thin line between operation rows where the tool changes; $tool the new tool's number.
browser-tool-change = cambio de herramienta → T{ $tool }
