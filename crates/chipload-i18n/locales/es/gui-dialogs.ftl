# File dialogs, typed-path prompts, the About window and the window title
# (crates/chipload-gui/src/app/dialogs.rs).

# Help > User guide, until the guide is published online. Keep the file names as they are.
dialogs-user-guide = La guía del usuario viene con cada versión como chipload-guide-<version>.zip (abre index.html). Desde el código fuente: packaging/docs/build-book.sh.

# Names of file types in the system's file dialogs.
dialogs-filter-projects = Proyectos de Chipload
dialogs-filter-dxf = Dibujos (DXF, SVG)

dialogs-selector-copied = Selector copiado al portapapeles

dialogs-about-title = Acerca de Chipload
dialogs-about-appimage = La AppImage de Linux también contiene libxkbcommon y libxcb-xkb (licencias de tipo MIT); sus archivos de copyright están en usr/share/doc/chipload/bundled dentro de la AppImage.
dialogs-third-party = Licencias de terceros

# Asked when a DXF doesn't say whether it was drawn in millimetres or inches.
dialogs-units-title = Unidades del dibujo
dialogs-units-unknown = { $file } no indica qué unidades usa.
dialogs-units-question = ¿En qué unidades se dibujó?
# A button: the drawing was drawn in inches.
dialogs-inch = pulgadas
dialogs-units-cancelled = Dibujo no importado: sus unidades son desconocidas.
dialogs-px-title = Píxeles por pulgada
dialogs-px-unknown = { $file } solo da su tamaño en píxeles.
dialogs-px-question = ¿Cuántos píxeles forman una pulgada?
dialogs-px-96 = 96 (Inkscape, navegadores)
dialogs-px-72 = 72 (Illustrator antiguo)
dialogs-px-other = Otro:
dialogs-px-use = Usar
dialogs-px-cancelled = Dibujo no importado: su tamaño es desconocido.
dialogs-export-dxf-title = Exportar dibujo como DXF
dialogs-export-dxf-inch = En pulgadas (si no, mm)
dialogs-export-dxf-hidden = También capas y grupos ocultos
dialogs-export-dxf-construction = También la geometría auxiliar (en capas propias)
dialogs-export-dxf-selection = Solo las formas seleccionadas
dialogs-export-dxf-save = Exportar…
dialogs-export-dxf-path = Exportar a ruta…
dialogs-path-export-dxf = El DXF que escribir:
dialogs-filter-dxf-file = Dibujo DXF

dialogs-unsaved-title = Cambios sin guardar
dialogs-unsaved-body = El proyecto tiene cambios sin guardar. ¿Guardarlos primero?
dialogs-save-hover = Guardar y continuar
dialogs-dont-save = No guardar
dialogs-dont-save-hover = Descartar los cambios y continuar

# Another program saved the open project's file.
dialogs-changed-title = { $file } cambió en el disco
dialogs-changed-body = Otro programa guardó el proyecto. ¿Cargar su versión? Tus cambios sin guardar quedan a un Ctrl+Z.
dialogs-load-it = Cargarla
dialogs-load-it-hover = Mostrar la versión del archivo; Ctrl+Z recupera la tuya
dialogs-keep-mine = Mantener la mía
dialogs-keep-mine-hover = Conservar tus cambios; el próximo Guardar sobrescribe el archivo

# Windows where a file's path is typed in: a title and what to type.
dialogs-path-open-title = Abrir ruta
dialogs-path-open = Un proyecto .chipload, o un .dxf o .svg:
dialogs-path-save-title = Guardar proyecto
dialogs-path-save = Guardar el proyecto en:
dialogs-path-export-title = Exportar código G
dialogs-path-export = Exportar el código G comprobado a:
dialogs-path-update-title = Actualizar dibujo
dialogs-path-update = El DXF o SVG:
dialogs-path-image-title = Importar imagen
dialogs-path-image = Una imagen (PNG, JPEG o BMP):
dialogs-path-add-dxf-title = Añadir DXF o SVG
dialogs-path-add-dxf = El DXF o SVG que se añade al proyecto:
dialogs-path-update-dxf-title = Actualizar archivo de dibujo
dialogs-path-update-dxf = La nueva versión del DXF o SVG:

# In the window's title bar, for a project never saved.
dialogs-title-new-project = proyecto nuevo

## Explode a DXF, join close ends (docs/specs/explode-and-join.md §5)

# $name is the DXF's name in the Browser.
dialogs-explode-title = Descomponer { $name } en formas
dialogs-explode-body = Sus formas pasan a ser formas dibujadas que puedes editar, mover y borrar. Las operaciones siguen cortando las mismas formas. El proyecto ya no conoce el archivo DXF: no se podrá actualizar desde el DXF.
dialogs-explode-image-body = Sus formas pasan a ser formas dibujadas que puedes editar, mover y borrar. Las operaciones siguen cortando las mismas formas. La imagen en sí sale del proyecto: no se podrá volver a vectorizar.
dialogs-explode-text-body = Sus letras pasan a ser formas dibujadas que puedes editar, mover y borrar. Las operaciones siguen cortando las mismas formas. Las palabras ya no se podrán editar.
dialogs-explode-join = Unir extremos a menos de
dialogs-explode = Descomponer
dialogs-join-title = Unir extremos
dialogs-join-body = Las polilíneas abiertas entre { $count ->
    [one] la forma seleccionada
   *[other] las { $count } formas seleccionadas
} cuyos extremos están a menos de la separación se convierten en una, en la misma capa. Una cadena que vuelve a su inicio queda cerrada.
dialogs-join-body-all = Las polilíneas dibujadas abiertas cuyos extremos están a menos de la separación se convierten en una, en la misma capa. Una cadena que vuelve a su inicio queda cerrada.
dialogs-join-gap = Unir extremos a menos de
dialogs-join = Unir

# V-carve inlays (docs/specs/vcarve-inlay.md §4)
inlay-ribbon = Incrustación…
inlay-ribbon-hover = Una incrustación tallada en V de las formas elegidas: un cajeado aquí, y un tapón en espejo en un tablero propio que se encola dentro
inlay-menu = Nueva incrustación…
inlay-title = Incrustación tallada en V
inlay-body = Las formas elegidas se tallan como cajeado en este tablero, y como tapón, en espejo, en una sujeción nueva: un tablero propio. El tapón se encola boca abajo en el cajeado y luego se cepilla su dorso.
inlay-vbit = Fresa en V
inlay-none = Ninguna
inlay-depth = Profundidad del cajeado
inlay-depth-hover = Lo hondo que se talla el cajeado en la base
inlay-plug-start = Profundidad inicial del tapón
inlay-plug-start-hover = A qué profundidad de su tablero se toma el contorno del tapón: su cara queda así de honda en el cajeado, y el resto del cajeado queda para la cola
inlay-gap = Holgura de cola
inlay-gap-hover = Cuánto más hondo se talla el tablero del tapón, para que su dorso quede así de alto sobre la base y la cola que rebosa tenga sitio
inlay-floor = Fresa recta del fondo
inlay-floor-hover = Una fresa recta para las pasadas de fondo de ambos. El canal del tapón es casi todo fondo
inlay-thickness = Grosor del tablero del tapón
inlay-thickness-hover = El tablero del que se corta el tapón. Lo que queda bajo su canal lo mantiene unido hasta cepillarlo
inlay-make = Crear incrustación
inlay-no-vbit = Elija una fresa en V: no hay ninguna en el proyecto ni en su biblioteca de herramientas
inlay-made = Incrustación creada: la operación { $pocket } es el cajeado, la operación { $plug } el tapón, en la sujeción { $setup }, su pieza en espejo de izquierda a derecha.
inlay-refused = No se pudo crear la incrustación.
sign-title = Nuevo letrero
sign-body = Las palabras, un marco alrededor que las sigue y cada operación que corta el letrero, en una pieza propia. Después se puede cambiar todo.
sign-words = Palabras
sign-words-hint = Lo que dice el letrero; Intro empieza una línea nueva
sign-around = Alrededor de la selección
sign-around-body = { $count ->
    [one] El letrero rodea la forma seleccionada, que se talla con la fresa en V. El marco la sigue, también una imagen vuelta a trazar.
   *[other] El letrero rodea las { $count } formas seleccionadas, que se tallan con la fresa en V. El marco las sigue, también una imagen vuelta a trazar.
}
sign-around-words = Escribir palabras en su lugar
sign-around-words-hover = Hacer el letrero alrededor de palabras nuevas, como sin selección
sign-font = Fuente
sign-height = Altura
sign-style = Estilo
sign-style-hover = Grabado: las letras talladas en V. En relieve: el fondo dentro del borde más interior rebajado alrededor de las letras. Marquetería: las letras incrustadas desde un segundo tablero.
sign-style-engraved = Grabado
sign-style-raised = En relieve
sign-style-inlay = Marquetería
sign-relief = Profundidad del fondo
sign-relief-hover = A qué profundidad se rebaja el fondo alrededor de las letras en relieve, con la fresa plana
sign-edge = Canto
sign-edge-chamfer = Chaflán
sign-edge-hover = Un chaflán de este ancho alrededor del contorno, cortado con la fresa en V antes de recortar el letrero
sign-vbit = Fresa en V
sign-end-mill = Fresa plana
wizard-library-tool-hover = De tu biblioteca de herramientas: se copia al proyecto al confirmar, no antes
wizard-library-tool-gone = Una herramienta elegida de tu biblioteca ya no está en ella: elige otra
sign-board = Tablero
sign-fit-board = Ajustar alrededor del letrero
sign-fit-board-hover = El material ajustado alrededor del dibujo con este margen, con este grosor. Desactivado: el material queda como está.
sign-thickness = Grosor
sign-board-margin = Margen
sign-adds = Añade: { $operations }
sign-make = Crear letrero
sign-no-tools = Elija una fresa en V y una fresa plana, del proyecto o de su biblioteca de herramientas.
sign-made = Letrero creado: { $count } operaciones en una pieza propia. Cambie las palabras y el marco las sigue.
sign-refused = No se pudo crear el letrero.
inlay-mark-gap = dorso, { $gap } por encima
inlay-mark-surface = superficie
inlay-mark-face = cara del tapón, { $depth }
inlay-mark-pocket = cajeado, { $depth }
