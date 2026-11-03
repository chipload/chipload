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
user-library-plunge-feed = Avance de penetración
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
user-library-cutting-data = Datos de corte
user-library-cutting-data-hover = Tus propios valores para esta herramienta, una fila por material. Cualquier material se usa sin material, o para un material sin fila propia. Se copian a una operación al elegir un material, cambiar su herramienta o añadirla; nunca cambian una operación por sí solos.
user-library-any-material = Cualquier material
user-library-material-gone = (un material que ya no está en tu biblioteca)
user-library-row-summary = ranura { $slot_feed } / { $slot_step_down } · vaciado { $feed } / { $step_down } × { $stepover }
user-library-row-speed = Velocidad del husillo
user-library-row-speed-hover = La velocidad para la que valen estos valores. Elegir el material para toda una sujeción da esta velocidad a la herramienta
user-library-row-own-speed = La de la herramienta
user-library-slot-feed = Avance ranura
user-library-slot-step-down = Prof. de pasada ranura
user-library-slot-hover = Cortes a todo el ancho: contorneados, trayectorias, tallados en V, taladrados helicoidales, el primer anillo de una cajera
user-library-clearing-feed = Avance vaciado
user-library-clearing-step-down = Prof. de pasada vaciado
user-library-clearing-hover = Cortes con paso lateral: los anillos de una cajera, un planeado
user-library-add-material = Añadir un material…
user-library-remove-row = Quitar
user-library-remove-row-hover = Quitar de la herramienta los valores de este material
user-library-tools-updated = Actualizado desde tu biblioteca:
user-library-tool-updated = T{ $tool }: { $changes }
user-library-tool-differs = T{ $tool }: la herramienta «{ $name }» de tu biblioteca tiene otros valores: { $changes }
user-library-update-all = Actualizar todo desde la biblioteca
user-library-update-all-hover = Cada herramienta del proyecto que difiere de la de la biblioteca toma sus valores; una velocidad del material de la sujeción se mantiene. Un paso de deshacer
user-library-update-all-none = Cada herramienta de tu biblioteca tiene sus valores
