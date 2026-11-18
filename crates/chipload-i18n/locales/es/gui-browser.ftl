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
browser-chamfer = Chaflán
browser-chamfer-hover = La arista superior de las formas cerradas elegidas rota con una fresa en V, del ancho que indiques
browser-holes = Taladros
browser-holes-hover = Los círculos seleccionados: taladrados con una broca, o mandrinados en hélice con una fresa
browser-surface = Planeado
browser-relief = Relieve
browser-relief-hover = Un modelo 3D tallado desde arriba en líneas paralelas, cada punto tan bajo como la herramienta puede llegar sin cortar el modelo. Dentro del contorno del modelo seleccionado, o del primero
browser-relief-needs-model = Añade primero un modelo 3D (Archivo > Añadir modelo 3D…)
browser-surface-hover = Todo el material en bruto rebajado en plano con una fresa ancha: aplanar el tablero de sacrificio o planear el material. No necesita selección

# Job settings and Tools.
browser-job-settings = Máquina
browser-job-settings-hover = La máquina: su postprocesador, las unidades del G-code, la Z segura, los recorridos y los cambios de herramienta, comunes a todas las sujeciones
browser-stock = Material en bruto
browser-stock-hover = El tablero que corta esta sujeción: su grosor, tamaño y material. Haz clic para ajustarlos.
browser-work-zero = Cero pieza
browser-from-front = de { $front }
browser-stock-locked-hover = El tablero de { $front }, dado la vuelta: se ajusta en su bruto y aquí se sigue
browser-work-zero-locked-hover = El cero de pieza de { $front }, reflejado sobre la línea de giro: se ajusta allí, aquí se sigue, nunca se mueve por sí solo
browser-work-zero-hover = Dónde está el X0 Y0 Z0 del código G. Haz clic para ajustarlo, o selecciónalo y arrástralo en la vista.
browser-job-settings-problems-hover = Máquina, postprocesador, unidades y recorridos. { $count ->
        [one] { $count } problema
       *[other] { $count } problemas
    }
browser-tools = Herramientas
browser-tools-hover = Las herramientas del proyecto: haz clic para verlas todas y añadir una

# Parts: groups of operations, each cut with its instances before the next.
browser-setups = Sujeciones
browser-setups-hover = Los tableros de los que se corta el proyecto, uno tras otro: cada uno un programa propio, con su material en bruto y su cero pieza. Un clic lo hace activo
browser-add-setup = Añadir una sujeción
browser-add-setup-hover = Otro tablero: su material, su cero y sus piezas, cortado como un programa propio
browser-add-back-side = Añadir el reverso
browser-add-back-side-hover = Una nueva sujeción: este tablero dado la vuelta, con una pieza propia. Dibuja el reverso visto desde el anverso, a través del tablero: sus cortes se reflejan
browser-setup-no-dowels-hover = Se recoloca con pasadores, y el anverso aún no tiene agujeros de pasador: añádelos en su formulario
browser-setup-parts = { $count ->
    [one] { $count } pieza
   *[other] { $count } piezas
}
browser-setup-active-hover = La sujeción activa: se muestra a todo color, se simula y se calcula. Un clic muestra su tablero; su cero está debajo
browser-setup-hover = Un clic la hace la sujeción activa: se muestra, se simula y se calcula
browser-move-to-setup = Mover a la sujeción
browser-place-on-setup = Colocar en la sujeción
browser-place-on-setup-hover = Una instancia nueva en el material en bruto de esta sujeción, donde está dibujada la pieza
browser-part-not-placed = sin colocar
browser-instance-of = { $part } #{ $number }
browser-parts = Piezas
browser-parts-hover = Cada pieza con sus operaciones. Sus instancias están bajo las sujeciones donde se cortan: arrastra una pieza a una sujeción para colocarla allí
browser-export-setup = Exportar el código G de esta sujeción…
browser-export-setup-hover = Su programa y su hoja de preparación, llamados -S1, -S2 si hay varias; pasa a ser la sujeción activa y se calcula antes si no se muestra
browser-new-part = Pieza nueva
# At a part row's right end: the part new operations go into.
browser-part-active = activa
browser-part-active-hover = La pieza activa: aquí van las operaciones nuevas. Clic para sus propiedades e instancias; clic derecho para más
browser-part-mirrored = reflejada
browser-add-instance = Añadir instancia
browser-add-instance-hover = Colocar la pieza otra vez junto a lo que corta, enlazada: cambie la pieza y la instancia la sigue. Arrástrela para moverla y su asa redonda para girarla.
browser-add-mirrored-instance = Añadir instancia reflejada
browser-add-mirrored-instance-hover = Lo mismo, reflejado de izquierda a derecha: el otro lado de una caja. Se sigue cortando en concordancia.
browser-instance-hover = La pieza colocada otra vez: las mismas formas y operaciones, movidas, giradas o reflejadas. Haga clic para cambiarla, o haga clic en su sombra en la vista.
browser-instance-mirror = Reflejar
browser-instance-turn = Girar 90°
browser-instance-remove = Quitar
browser-face-board = Planear el tablero…
browser-face-board-hover = Un planeado de todo el tablero, primero en esta sujeción, cortado una vez sean cuales sean sus piezas e instancias: aplanar el tablero de sacrificio o planear el material. El área de una pieza es un planeado de su contorno seleccionado
browser-surface-needs-shapes = Selecciona primero el contorno de la pieza. Todo el tablero es de la sujeción: clic derecho en ella, Planear el tablero…
app-face-board-failed = No se puede planear el tablero
browser-place-part = Colocar…
browser-place-part-hover = Selecciona dónde se corta la pieza: un fantasma para mover y girar (arrástralo, su asa redonda, o Q para la brújula); sus formas quedan donde están dibujadas
browser-reset-placement = Restablecer colocación
browser-reset-placement-hover = Cortar de nuevo la pieza donde están dibujadas sus formas
browser-part-hover = Clic para sus propiedades e instancias; doble clic para hacerla la pieza activa; clic derecho para más
browser-part-drag-hover = Arrástrela a una sujeción para cortarla allí; con Ctrl (⌘), también allí
# A verb: make this the active part.
browser-activate = Activar
browser-activate-hover = Las operaciones nuevas van a la pieza activa
browser-export-part = Exportar el G-code de esta pieza…
browser-export-part-hover = Escribir el G-code solo de esta pieza, como si todas las demás piezas estuvieran desactivadas; también su hoja de preparación si ese ajuste está activo.
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
browser-add-floor-pass = Añadir pasada de fondo
browser-add-floor-pass-hover = Una fresa plana que vacía el fondo entre las paredes de los trazos demasiado anchos, en las formas y profundidades de este tallado en V. Se corta antes del tallado en V
browser-floor = Fondo
browser-floor-pass-hover = Una pasada de fondo: una fresa plana vacía el fondo entre las paredes de su tallado en V, antes de este; la fresa en V repasa donde no llega
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

# Reliefs and their roughing pass (docs/specs/stl-relief.md §9, §14).
browser-add-roughing = Añadir pasada de desbaste
browser-add-roughing-hover = Una fresa plana o de punta esférica simple que primero quita el material por niveles, dejando sobrematerial para el relieve
browser-add-smoothing-needs-roughing = La pasada de alisado sigue a una pasada de desbaste: añade primero una pasada de desbaste
browser-roughing = Desbaste
browser-roughing-hover = Una pasada de desbaste: el material quitado por niveles antes del relieve, con sobrematerial sobre el modelo
browser-smoothing = Alisado
browser-smoothing-hover = Una pasada de alisado: tras el desbaste, la superficie seguida una vez, quita los escalones que el desbaste deja en las pendientes
browser-flute = Acanalado
browser-flute-hover = Una ranura a lo largo de las formas abiertas seleccionadas que entra y sale en rampa en sus extremos
browser-pattern = Patrón
browser-pattern-hover = Ranuras de líneas, trama cruzada, anillos u ondas grabadas dentro de las formas cerradas seleccionadas
browser-prism = Prisma
browser-prism-hover = Las formas cerradas seleccionadas quedan en pie como tejados, con una arista viva en el centro, cortadas con una fresa en V
browser-moulding = Moldura
browser-moulding-hover = Un perfil (mediacaña, gola, junquillo…) barrido a lo largo de las trayectorias seleccionadas
