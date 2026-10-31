# The library rows of a project tool and the machine, and copying values into and out of the project (crates/chipload-gui/src/app/user_library.rs).

# The user's library of tools and machine profiles, kept on this computer.
user-library-profile = Perfil
user-library-tool = Herramienta de la biblioteca
user-library-same = igual que en tu biblioteca
user-library-differs = difiere de tu biblioteca: { $changes }
user-library-more-hover = Guardar como nueva entrada o mostrarla en la biblioteca
user-library-use-profile = Usar perfil…
user-library-use-tool = Usar herramienta de la biblioteca…
user-library-save-to-library = Guardar en la biblioteca
user-library-save-to-library-named = Guardar en la biblioteca…
user-library-save-to-library-hover = Copia estos valores a tu biblioteca, para otros proyectos
user-library-save-to-linked-profile-hover = Escribir estos valores en el perfil al que está vinculada, para otros proyectos
user-library-save-to-linked-tool-hover = Escribir estos valores en la herramienta de la biblioteca a la que está vinculada, para otros proyectos
user-library-save-as-new = Guardar como nuevo…
user-library-save-as-new-hover = Guardar estos valores en tu biblioteca con otro nombre
user-library-update-from-library = Actualizar desde la biblioteca
user-library-update-from-library-hover = Copiar los valores de la biblioteca al proyecto (se deshace con Ctrl+Z)
user-library-show-in-library = Mostrar en la biblioteca
user-library-show-in-library-hover = Abrirla en Este equipo en el Navegador
# $var is the name of an environment variable.
user-library-no-settings-folder-set = No se encontró la carpeta de ajustes; define { $var }
user-library-no-settings-folder-use = No se encontró la carpeta de ajustes; define { $var } para usar una biblioteca.
user-library-no-settings-folder = No hay carpeta de ajustes
user-library-kept-in = Guardada en { $path }

user-library-name = Nombre
# The tool's number (T1, T2, …) in a project.
user-library-tool-number = N.º de herramienta
user-library-tool-number-hover = El número que recibe en un proyecto donde ese número esté libre
user-library-cutting-values = Valores de corte
user-library-cutting-values-hover = Se rellenan en una operación nueva con esta herramienta cuando no hay una operación anterior de la que copiarlos. Nunca cambian una operación existente.
user-library-feed = Avance
user-library-plunge-feed = Avance de penetración
user-library-step-down = Prof. de pasada
user-library-stepover = Paso lateral
user-library-safe-z = Z segura
user-library-max-overcut = Prof. máx. bajo el material
user-library-max-overcut-hover = Cuánto puede bajar un corte por debajo del material (en el tablero de sacrificio)

user-library-settings-not-saved = Ajustes no guardados: { $error }
user-library-recent-gone = { $path } ya no existe; se quitó de la lista de recientes.
# Followed by the reason on the next line.
user-library-post-named = Postprocesador "{ $name }":
user-library-post-chosen = Postprocesador: { $name }
user-library-no-machine-profile = No existe el perfil de máquina "{ $name }".
user-library-no-library-tool = No existe la herramienta de biblioteca "{ $name }".
user-library-no-project-tool = El proyecto no tiene la herramienta { $id }.
user-library-machine-profile-named = Perfil de máquina "{ $name }"
user-library-library-tool-named = Herramienta de biblioteca "{ $name }"
# $what is user-library-machine-profile-named or user-library-library-tool-named.
user-library-no-values-changed-status = { $what }: ningún valor cambiado
user-library-values-changed-status = { $what }: { $count ->
        [one] { $count } valor cambiado
       *[other] { $count } valores cambiados
    }
user-library-changes-heading = { $what }:
user-library-no-values-changed = ningún valor cambiado

user-library-profile-gone = El perfil del que salió no está en tu biblioteca.
user-library-library-tool-gone = La herramienta de biblioteca de la que salió no está en tu biblioteca.
user-library-no-tool-to-save = El proyecto no tiene ninguna herramienta que guardar.
# Followed by the reason on the next line.
user-library-not-saved = No se guardó en la biblioteca:
user-library-saved = "{ $name }" guardado en tu biblioteca

# Dialog titles.
user-library-save-machine-title = Guardar perfil de máquina
user-library-save-tool-title = Guardar herramienta en la biblioteca
user-library-name-in-library = Nombre en tu biblioteca:
user-library-replaces = Reemplaza "{ $name }" en tu biblioteca.
# Buttons.
user-library-replace = Reemplazar

user-library-tool-exists = Ya existe una herramienta "{ $name }".
user-library-machine-exists = Ya existe una máquina "{ $name }".
# The names a new library entry gets ("New tool 2" if taken).
user-library-new-tool = Herramienta nueva
user-library-new-machine = Máquina nueva
