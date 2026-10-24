# Messages of the post-processors (crates/chipload-post): a config or template that can't
# be read, or a program it can't write. Names of posts, templates and the config's keys
# (schema_version, decimals, units, …) are written as in the config file and never
# translated; $name, $post and $template come in quotes. $error is the words of the
# library that failed (the TOML reader, the template engine, the file system).

## Reading a config

post-error-version = Postprozessor-Konfiguration mit schema_version { $found } wird nicht unterstützt (diese Anwendung unterstützt bis { $current }). Aktualisieren Sie die Anwendung oder prüfen Sie die Datei.
post-error-cannot-read = Postprozessor-Konfiguration kann nicht gelesen werden: { $error }
post-error-cannot-read-file = Postprozessor-Konfiguration { $path } kann nicht gelesen werden: { $error }
# $path is a config or template file.
post-error-cannot-read-path = { $path } kann nicht gelesen werden: { $error }
# $error is the TOML reader's words, or the problems below, joined by semicolons.
post-error-invalid = Ungültige Postprozessor-Konfiguration: { $error }
post-error-template-name = template muss ein einfacher Dateiname sein, ist { $got }
post-error-decimals = decimals darf höchstens 6 sein, ist { $got }
post-error-inch-decimals = inch_decimals darf höchstens 6 sein, ist { $got }
post-error-units-empty = units muss mindestens eines von mm und inch nennen
# $got is a time in seconds, without its unit.
post-error-dwell = spindle_dwell_s muss mindestens 0 sein, ist { $got }
post-error-template-unreadable = Vorlage { $template } kann nicht gelesen werden: { $error }
post-error-name = Ungültiger Postprozessor-Name { $name }
post-error-builtin-name = { $name } ist der Name eines eingebauten Postprozessors; benennen Sie Ihre Datei um
post-error-unknown = Unbekannter Postprozessor { $name }
post-error-unknown-template = Unbekannte Vorlage { $template }
post-error-syntax = Syntax-Einstellungen der Vorlage: { $error }

## Writing a program

# $units is "mm" or "inch", $listed the same in quotes, as the config would list it.
# "{{ units_word }}" and G21 are written in the template that way.
post-error-units = Postprozessor { $post } kann keinen G-Code in { $units } schreiben: seine Konfiguration führt { $listed } nicht unter units auf (seine Vorlage muss { "{{ units_word }}" } statt G21 schreiben)
post-error-tool-changes = Postprozessor { $post } kann keine Werkzeugwechsel in eine Datei schreiben; schreiben Sie stattdessen eine Datei pro Werkzeug (Werkzeugwechsel: files)
post-error-arc-centre = Bogenbewegung ohne Mittelpunkt
post-error-arc-from = Bogen von einer unbekannten Position
post-error-arc-to = Bogen zu einer unbekannten Position
post-error-feed-rate = Vorschubbewegung ohne Vorschubgeschwindigkeit
post-error-no-groups = Keine Werkzeuggruppen zum Ausgeben
post-error-template-failed = Vorlage { $template } ist fehlgeschlagen: { $error }
