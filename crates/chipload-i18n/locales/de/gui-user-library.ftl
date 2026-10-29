# The library rows of a project tool and the machine, and copying values into and out of the project (crates/chipload-gui/src/app/user_library.rs).

# The user's library of tools and machine profiles, kept on this computer.
user-library-library = Bibliothek
user-library-use-profile = Profil verwenden…
user-library-use-tool = Bibliothekswerkzeug verwenden…
user-library-save-to-library = In Bibliothek speichern
user-library-save-to-library-named = In Bibliothek speichern…
user-library-save-to-library-hover = Kopiert diese Werte in Ihre Bibliothek, für andere Projekte
user-library-save-to-linked-profile-hover = Diese Werte in das verknüpfte Profil schreiben, für andere Projekte
user-library-save-to-linked-tool-hover = Diese Werte in das verknüpfte Bibliothekswerkzeug schreiben, für andere Projekte
user-library-save-as-new = Als neu speichern…
user-library-save-as-new-hover = Diese Werte unter einem anderen Namen in Ihrer Bibliothek behalten
user-library-update-from-library = Aus Bibliothek aktualisieren
user-library-update-from-library-hover = Die Werte der Bibliothek ins Projekt kopieren (rückgängig mit Ctrl+Z)
user-library-show-in-library = In Bibliothek zeigen
user-library-show-in-library-hover = Unter Dieser Computer im Browser öffnen
# $var is the name of an environment variable.
user-library-no-settings-folder-set = Kein Einstellungsordner gefunden; setzen Sie { $var }
user-library-no-settings-folder-use = Kein Einstellungsordner gefunden; setzen Sie { $var }, um eine Bibliothek zu verwenden.
user-library-no-settings-folder = Kein Einstellungsordner
user-library-kept-in = Gespeichert in { $path }

user-library-name = Name
# The tool's number (T1, T2, …) in a project.
user-library-tool-number = Werkzeugnummer
user-library-tool-number-hover = Die Nummer, die es in einem Projekt bekommt, in dem diese Nummer frei ist
user-library-cutting-values = Schnittwerte
user-library-cutting-values-hover = Werden in eine neue Bearbeitung mit diesem Werkzeug eingetragen, wenn es keine Bearbeitung darüber gibt, von der kopiert werden kann. Ändert nie eine bestehende Bearbeitung.
user-library-feed = Vorschub
user-library-plunge-feed = Eintauchvorschub
user-library-step-down = Zustellung
user-library-stepover = Seitl. Zustellung
user-library-safe-z = Sicherheitshöhe
user-library-max-overcut = Max. Tiefe unter Rohteil
user-library-max-overcut-hover = Wie weit ein Schnitt unter das Rohteil gehen darf (in die Opferplatte)

user-library-settings-not-saved = Einstellungen nicht gespeichert: { $error }
user-library-recent-gone = { $path } existiert nicht mehr; es wurde aus der Liste der zuletzt geöffneten genommen.
# Followed by the reason on the next line.
user-library-post-named = Postprozessor „{ $name }“:
user-library-post-chosen = Postprozessor: { $name }
user-library-no-machine-profile = Es gibt kein Maschinenprofil „{ $name }“.
user-library-no-library-tool = Es gibt kein Bibliothekswerkzeug „{ $name }“.
user-library-no-project-tool = Das Projekt hat kein Werkzeug { $id }.
user-library-machine-profile-named = Maschinenprofil „{ $name }“
user-library-library-tool-named = Bibliothekswerkzeug „{ $name }“
# $what is user-library-machine-profile-named or user-library-library-tool-named.
user-library-no-values-changed-status = { $what }: keine Werte geändert
user-library-values-changed-status = { $what }: { $count ->
        [one] { $count } Wert geändert
       *[other] { $count } Werte geändert
    }
user-library-changes-heading = { $what }:
user-library-no-values-changed = keine Werte geändert

user-library-profile-gone = Das Profil, aus dem es stammt, ist nicht in Ihrer Bibliothek.
user-library-library-tool-gone = Das Bibliothekswerkzeug, aus dem es stammt, ist nicht in Ihrer Bibliothek.
user-library-no-tool-to-save = Das Projekt hat kein Werkzeug zum Speichern.
# Followed by the reason on the next line.
user-library-not-saved = Nicht in der Bibliothek gespeichert:
user-library-saved = „{ $name }“ in Ihrer Bibliothek gespeichert

# Dialog titles.
user-library-save-machine-title = Maschinenprofil speichern
user-library-save-tool-title = Werkzeug in Bibliothek speichern
user-library-name-in-library = Name in Ihrer Bibliothek:
user-library-replaces = Ersetzt „{ $name }“ in Ihrer Bibliothek.
# Buttons.
user-library-replace = Ersetzen

user-library-tool-exists = Es gibt bereits ein Werkzeug „{ $name }“.
user-library-machine-exists = Es gibt bereits eine Maschine „{ $name }“.
# The names a new library entry gets ("New tool 2" if taken).
user-library-new-tool = Neues Werkzeug
user-library-new-machine = Neue Maschine
