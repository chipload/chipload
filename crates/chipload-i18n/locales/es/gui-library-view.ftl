# The user's library in the Browser and in Properties (crates/chipload-gui/src/app/library_view.rs).

# Buttons in a library folder's menu and properties.
library-view-new-tool = Herramienta nueva
library-view-new-machine = Máquina nueva
library-view-import = Importar…
library-view-export = Exportar…

# The Browser's root for what is kept on this computer, not in the project.
library-view-this-computer = Este equipo
library-view-this-computer-hover = Tus preferencias, herramientas, máquinas y postprocesadores, guardados en este equipo para todos los proyectos. Un proyecto guarda copias de lo que usa, así que corta igual dentro de un año y en otro equipo.
library-view-preferences = Preferencias
library-view-preferences-hover = La hoja de preparación, la actualización automática de la trayectoria, el tema de color
library-view-no-settings-folder = No hay carpeta de ajustes
# $var is the name of an environment variable.
library-view-no-settings-folder-hover = Define { $var } para conservar una biblioteca
library-view-tools = Herramientas
library-view-tools-hover = Tu biblioteca de herramientas: arrastra una herramienta a las Herramientas del proyecto para usarla
library-view-machines = Máquinas
library-view-machines-hover = Tus perfiles de máquina: los valores con los que empiezan los proyectos nuevos
library-view-tool-hover = Haz clic para editarla; arrástrala a las Herramientas del proyecto para usarla allí

# Commands on a library tool or machine profile.
library-view-add-to-project = Añadir al proyecto
library-view-add-to-project-hover = Copiarla al proyecto como herramienta propia
library-view-use-in-project = Usar en este proyecto
library-view-use-in-project-hover = Copiar sus valores a la máquina del proyecto (se deshace con Ctrl+Z)
library-view-entry-disabled-hover = Abre un proyecto, y guarda o revierte los cambios primero
# A button: make this entry the one new projects start with.
library-view-make-default = Predeterminar
library-view-make-default-hover = Los proyectos nuevos empiezan con ella
library-view-delete-hover = Sacarla de tu biblioteca; los proyectos conservan sus copias
library-view-save-hover = Escribir los cambios en tu biblioteca
library-view-revert = Revertir
library-view-revert-hover = Descartar los cambios
library-view-not-saved = sin guardar

library-view-tool-library = Biblioteca de herramientas
library-view-tool-library-about = Un proyecto guarda una copia de cada herramienta que usa. Haz clic en una herramienta en Este equipo para editarla; arrástrala a las Herramientas del proyecto, o haz clic derecho en ella, para usarla.
library-view-machines-about = Un proyecto guarda una copia de los valores de su máquina. Haz clic derecho en un perfil para usarlo en este proyecto; el predeterminado es con el que empiezan los proyectos nuevos.
library-view-tool-count = { $count ->
    [one] { $count } herramienta
   *[other] { $count } herramientas
} en tu biblioteca.
library-view-machine-count = { $count ->
    [one] { $count } máquina
   *[other] { $count } máquinas
} en tu biblioteca.

library-view-library-tool = Herramienta de la biblioteca
library-view-tool-gone = Esta herramienta ya no está en tu biblioteca.
library-view-machine-profile = Perfil de máquina
library-view-profile-gone = Este perfil ya no está en tu biblioteca.
library-view-entry-about = En tu biblioteca, para todos los proyectos. Los cambios se conservan al pulsar Guardar.
# $tools lists the project's tools, like "T1, T3".
library-view-project-copies = Este proyecto tiene una copia: { $tools }
library-view-project-machine = La máquina de este proyecto salió de aquí.

library-view-deleted = "{ $name }" eliminado de tu biblioteca; los proyectos conservan sus copias
library-view-saved = Guardado en tu biblioteca

# Notes at the right end of a library entry's row in the Browser.
library-view-note-default = predeterminado
library-view-note-unsaved = sin guardar
