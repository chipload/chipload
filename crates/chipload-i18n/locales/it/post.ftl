# Messages of the post-processors (crates/chipload-post): a config or template that can't
# be read, or a program it can't write. Names of posts, templates and the config's keys
# (schema_version, decimals, units, …) are written as in the config file and never
# translated; $name, $post and $template come in quotes. $error is the words of the
# library that failed (the TOML reader, the template engine, the file system).

## Reading a config

post-error-version = La schema_version { $found } della configurazione del post-processore non è supportata (questa app supporta fino alla { $current }). Aggiorna l'applicazione o controlla il file.
post-error-cannot-read = Impossibile leggere la configurazione del post-processore: { $error }
post-error-cannot-read-file = Impossibile leggere la configurazione del post-processore { $path }: { $error }
# $path is a config or template file.
post-error-cannot-read-path = Impossibile leggere { $path }: { $error }
# $error is the TOML reader's words, or the problems below, joined by semicolons.
post-error-invalid = Configurazione del post-processore non valida: { $error }
post-error-template-name = template deve essere un semplice nome di file, trovato { $got }
post-error-decimals = decimals deve essere al massimo 6, trovato { $got }
post-error-inch-decimals = inch_decimals deve essere al massimo 6, trovato { $got }
post-error-units-empty = units deve indicare almeno uno tra mm e inch
# $got is a time in seconds, without its unit.
post-error-dwell = spindle_dwell_s deve essere almeno 0, trovato { $got }
post-error-template-unreadable = Impossibile leggere il template { $template }: { $error }
post-error-name = Nome di post-processore non valido: { $name }
post-error-builtin-name = { $name } è il nome di un post-processore integrato; rinomina il tuo file
post-error-unknown = Post-processore sconosciuto: { $name }
post-error-unknown-template = Template sconosciuto: { $template }
post-error-syntax = Impostazioni di sintassi del template: { $error }

## Writing a program

# $units is "mm" or "inch", $listed the same in quotes, as the config would list it.
# "{{ units_word }}" and G21 are written in the template that way.
post-error-units = Il post-processore { $post } non può scrivere G-code in { $units }: la sua configurazione non elenca { $listed } in units (il suo template deve scrivere { "{{ units_word }}" } invece di G21)
post-error-tool-changes = Il post-processore { $post } non può scrivere i cambi utensile in un unico file; scrivi invece un file per utensile (cambi utensile: files)
post-error-arc-centre = Movimento ad arco senza centro
post-error-arc-from = Arco da una posizione sconosciuta
post-error-arc-to = Arco verso una posizione sconosciuta
post-error-feed-rate = Movimento in avanzamento senza velocità di avanzamento
post-error-no-groups = Nessun gruppo di utensili da generare
post-error-template-failed = Template { $template } non riuscito: { $error }
