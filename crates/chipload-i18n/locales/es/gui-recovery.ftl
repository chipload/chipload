# Recovery copies of unsaved work, and the window that offers them at start
# (crates/chipload-gui/src/app/recovery.rs).

recovery-copies-off = Las copias de recuperación están desactivadas: { $error }
# Followed by the reason on the next line.
recovery-cannot-open = No se puede abrir la copia de recuperación:
recovery-recovered = Trabajo sin guardar recuperado: { $title }. Guárdalo para conservarlo.

recovery-title = Recuperar trabajo sin guardar
recovery-intro = Chipload se cerró sin guardar estos cambios:
# A copy of a project that was never saved to a file.
recovery-untitled = Sin título (nunca guardado)

# When the unsaved changes in a copy were made.
recovery-changes-unknown = Cambios de un momento desconocido
recovery-changes-moments = Cambios de hace menos de un minuto
recovery-changes-minutes = { $count ->
    [one] Cambios de hace { $count } minuto
   *[other] Cambios de hace { $count } minutos
}
recovery-changes-hours = { $count ->
    [one] Cambios de hace { $count } hora
   *[other] Cambios de hace { $count } horas
}
recovery-changes-days = { $count ->
    [one] Cambios de hace { $count } día
   *[other] Cambios de hace { $count } días
}

recovery-saved-since = El archivo del proyecto se ha guardado después de hacerse esta copia.
recovery-cannot-be-opened = No se puede abrir: { $problem }

# A button: open the copy.
recovery-open = Abrir
recovery-open-hover = Abrirla como proyecto sin guardar; guárdala para conservarla
recovery-open-dirty = Guarda o cierra primero el proyecto actual
recovery-open-unreadable = Esta copia no se puede leer
# A button: decide about the copy another time.
recovery-later = Más tarde
recovery-later-hover = Conservarla y volver a preguntar la próxima vez
