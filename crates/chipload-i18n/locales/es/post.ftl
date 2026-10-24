# Messages of the post-processors (crates/chipload-post): a config or template that can't
# be read, or a program it can't write. Names of posts, templates and the config's keys
# (schema_version, decimals, units, …) are written as in the config file and never
# translated; $name, $post and $template come in quotes. $error is the words of the
# library that failed (the TOML reader, the template engine, the file system).

## Reading a config

post-error-version = El schema_version { $found } de la configuración del postprocesador no es compatible (esta aplicación admite hasta { $current }). Actualiza la aplicación o revisa el archivo.
post-error-cannot-read = No se puede leer la configuración del postprocesador: { $error }
post-error-cannot-read-file = No se puede leer la configuración del postprocesador { $path }: { $error }
# $path is a config or template file.
post-error-cannot-read-path = No se puede leer { $path }: { $error }
# $error is the TOML reader's words, or the problems below, joined by semicolons.
post-error-invalid = Configuración del postprocesador no válida: { $error }
post-error-template-name = template debe ser un nombre de archivo simple, es { $got }
post-error-decimals = decimals debe ser como máximo 6, es { $got }
post-error-inch-decimals = inch_decimals debe ser como máximo 6, es { $got }
post-error-units-empty = units debe nombrar al menos uno de mm e inch
# $got is a time in seconds, without its unit.
post-error-dwell = spindle_dwell_s debe ser al menos 0, es { $got }
post-error-template-unreadable = No se puede leer la plantilla { $template }: { $error }
post-error-name = Nombre de postprocesador no válido: { $name }
post-error-builtin-name = { $name } es el nombre de un postprocesador incluido; cambia el nombre de tu archivo
post-error-unknown = Postprocesador desconocido: { $name }
post-error-unknown-template = Plantilla desconocida: { $template }
post-error-syntax = Ajustes de sintaxis de la plantilla: { $error }

## Writing a program

# $units is "mm" or "inch", $listed the same in quotes, as the config would list it.
# "{{ units_word }}" and G21 are written in the template that way.
post-error-units = El postprocesador { $post } no puede escribir código G en { $units }: su configuración no incluye { $listed } en units (su plantilla debe escribir { "{{ units_word }}" } en lugar de G21)
post-error-tool-changes = El postprocesador { $post } no puede escribir cambios de herramienta en un solo archivo; escribe un archivo por herramienta (tool changes: files)
post-error-arc-centre = Movimiento en arco sin centro
post-error-arc-from = Arco desde una posición desconocida
post-error-arc-to = Arco hacia una posición desconocida
post-error-feed-rate = Movimiento de avance sin velocidad de avance
post-error-no-groups = No hay grupos de herramientas que generar
post-error-template-failed = La plantilla { $template } falló: { $error }
