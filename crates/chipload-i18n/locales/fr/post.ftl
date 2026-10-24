# Messages of the post-processors (crates/chipload-post): a config or template that can't
# be read, or a program it can't write. Names of posts, templates and the config's keys
# (schema_version, decimals, units, …) are written as in the config file and never
# translated; $name, $post and $template come in quotes. $error is the words of the
# library that failed (the TOML reader, the template engine, the file system).

## Reading a config

post-error-version = La version schema_version { $found } de la configuration du post-processeur n'est pas prise en charge (cette application prend en charge jusqu'à { $current }). Mettez à jour l'application ou vérifiez le fichier.
post-error-cannot-read = Impossible de lire la configuration du post-processeur : { $error }
post-error-cannot-read-file = Impossible de lire la configuration du post-processeur { $path } : { $error }
# $path is a config or template file.
post-error-cannot-read-path = Impossible de lire { $path } : { $error }
# $error is the TOML reader's words, or the problems below, joined by semicolons.
post-error-invalid = Configuration du post-processeur non valide : { $error }
post-error-template-name = template doit être un simple nom de fichier, reçu { $got }
post-error-decimals = decimals doit valoir au plus 6, reçu { $got }
post-error-inch-decimals = inch_decimals doit valoir au plus 6, reçu { $got }
post-error-units-empty = units doit nommer au moins l'un de mm et inch
# $got is a time in seconds, without its unit.
post-error-dwell = spindle_dwell_s doit valoir au moins 0, reçu { $got }
post-error-template-unreadable = Impossible de lire le modèle { $template } : { $error }
post-error-name = Nom de post-processeur non valide : { $name }
post-error-builtin-name = { $name } est le nom d'un post-processeur intégré ; renommez votre fichier
post-error-unknown = Post-processeur inconnu : { $name }
post-error-unknown-template = Modèle inconnu : { $template }
post-error-syntax = Réglages de syntaxe du modèle : { $error }

## Writing a program

# $units is "mm" or "inch", $listed the same in quotes, as the config would list it.
# "{{ units_word }}" and G21 are written in the template that way.
post-error-units = Le post-processeur { $post } ne peut pas écrire de G-code en { $units } : sa configuration ne liste pas { $listed } sous units (son modèle doit écrire { "{{ units_word }}" } au lieu de G21)
post-error-tool-changes = Le post-processeur { $post } ne peut pas écrire de changements d'outil dans un seul fichier ; écrivez plutôt un fichier par outil (tool changes: files)
post-error-arc-centre = Mouvement en arc sans centre
post-error-arc-from = Arc depuis une position inconnue
post-error-arc-to = Arc vers une position inconnue
post-error-feed-rate = Mouvement d'avance sans vitesse d'avance
post-error-no-groups = Aucun groupe d'outils à produire
post-error-template-failed = Le modèle { $template } a échoué : { $error }
