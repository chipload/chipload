# The user's library in the Browser and in Properties (crates/chipload-gui/src/app/library_view.rs).

# Buttons in a library folder's menu and properties.
library-view-new-tool = Neues Werkzeug
library-view-new-machine = Neue Maschine
library-view-import = Importieren…
library-view-export = Exportieren…

# The Browser's root for what is kept on this computer, not in the project.
library-view-this-computer = Dieser Computer
library-view-this-computer-hover = Ihre Einstellungen, Werkzeuge, Maschinen und Postprozessoren, auf diesem Computer für jedes Projekt gespeichert. Ein Projekt behält Kopien von allem, was es verwendet, damit es in einem Jahr und auf einem anderen Computer gleich fräst.
library-view-preferences = Einstellungen
library-view-preferences-hover = Das Einrichteblatt, das automatische Aktualisieren des Werkzeugwegs, das Farbschema
library-view-no-settings-folder = Kein Einstellungsordner
# $var is the name of an environment variable.
library-view-no-settings-folder-hover = Setzen Sie { $var }, um eine Bibliothek zu behalten
library-view-tools = Werkzeuge
library-view-tools-hover = Ihre Werkzeugbibliothek: ein Werkzeug auf die Werkzeuge des Projekts ziehen, um es zu verwenden
library-view-machines = Maschinen
library-view-machines-hover = Ihre Maschinenprofile: die Werte, mit denen neue Projekte beginnen
library-view-tool-hover = Klicken zum Bearbeiten; auf die Werkzeuge des Projekts ziehen, um es dort zu verwenden

# Commands on a library tool or machine profile.
library-view-add-to-project = Zum Projekt hinzufügen
library-view-add-to-project-hover = Als eigenes Werkzeug ins Projekt kopieren
library-view-use-in-project = In diesem Projekt verwenden
library-view-use-in-project-hover = Seine Werte in die Maschine des Projekts kopieren (rückgängig mit Ctrl+Z)
library-view-entry-disabled-hover = Öffnen Sie ein Projekt, und speichern oder verwerfen Sie zuerst die Änderungen
# A button: make this entry the one new projects start with.
library-view-make-default = Als Standard
library-view-make-default-hover = Neue Projekte beginnen damit
library-view-delete-hover = Aus Ihrer Bibliothek nehmen; Projekte behalten ihre Kopien
library-view-save-hover = Die Änderungen in Ihre Bibliothek schreiben
library-view-revert = Verwerfen
library-view-revert-hover = Die Änderungen verwerfen
library-view-not-saved = nicht gespeichert

library-view-tool-library = Werkzeugbibliothek
library-view-tool-library-about = Ein Projekt behält eine Kopie jedes Werkzeugs, das es verwendet. Klicken Sie auf ein Werkzeug unter Dieser Computer, um es zu bearbeiten; ziehen Sie es auf die Werkzeuge des Projekts oder klicken Sie mit rechts darauf, um es zu verwenden.
library-view-machines-about = Ein Projekt behält eine Kopie der Werte seiner Maschine. Rechtsklick auf ein Profil verwendet es in diesem Projekt; der Standard ist das, womit neue Projekte beginnen.
library-view-tool-count = { $count ->
    [one] { $count } Werkzeug
   *[other] { $count } Werkzeuge
} in Ihrer Bibliothek.
library-view-machine-count = { $count ->
    [one] { $count } Maschine
   *[other] { $count } Maschinen
} in Ihrer Bibliothek.

library-view-library-tool = Bibliothekswerkzeug
library-view-tool-gone = Dieses Werkzeug ist nicht mehr in Ihrer Bibliothek.
library-view-machine-profile = Maschinenprofil
library-view-profile-gone = Dieses Profil ist nicht mehr in Ihrer Bibliothek.
library-view-entry-about = In Ihrer Bibliothek, für jedes Projekt. Änderungen bleiben erhalten, wenn Sie Speichern drücken.
# $tools lists the project's tools, like "T1, T3".
library-view-project-copies = Dieses Projekt hat eine Kopie: { $tools }
library-view-project-machine = Die Maschine dieses Projekts stammt daraus.

library-view-deleted = „{ $name }“ aus Ihrer Bibliothek gelöscht; Projekte behalten ihre Kopien
library-view-saved = In Ihrer Bibliothek gespeichert

# Notes at the right end of a library entry's row in the Browser.
library-view-note-default = Standard
library-view-note-unsaved = ungespeichert
