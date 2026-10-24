# File dialogs, typed-path prompts, the About window and the window title
# (crates/chipload-gui/src/app/dialogs.rs).

# Help > User guide, until the guide is published online. Keep the file names as they are.
dialogs-user-guide = La guía del usuario viene con cada versión como chipload-guide-<version>.zip (abre index.html). Desde el código fuente: packaging/docs/build-book.sh.

# Names of file types in the system's file dialogs.
dialogs-filter-projects = Proyectos de Chipload
dialogs-filter-dxf = Dibujos DXF

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
dialogs-path-open = Un proyecto .chipload o un .dxf:
dialogs-path-save-title = Guardar proyecto
dialogs-path-save = Guardar el proyecto en:
dialogs-path-export-title = Exportar código G
dialogs-path-export = Exportar el código G comprobado a:
dialogs-path-update-title = Actualizar dibujo
dialogs-path-update = El DXF:
dialogs-path-image-title = Importar imagen
dialogs-path-image = Una imagen (PNG, JPEG o BMP):
dialogs-path-add-dxf-title = Añadir DXF
dialogs-path-add-dxf = El DXF que se añade al proyecto:
dialogs-path-update-dxf-title = Actualizar DXF
dialogs-path-update-dxf = La nueva versión del DXF:

# In the window's title bar, for a project never saved.
dialogs-title-new-project = proyecto nuevo
