# The user's library in the Browser and in Properties (crates/chipload-gui/src/app/library_view.rs).

# Buttons in a library folder's menu and properties.
library-view-new-tool = Nouvel outil
library-view-new-machine = Nouvelle machine
library-view-import = Importer…
library-view-export = Exporter…

# The Browser's root for what is kept on this computer, not in the project.
library-view-this-computer = Cet ordinateur
library-view-this-computer-hover = Vos préférences, outils, machines et post-processeurs, gardés sur cet ordinateur pour tous les projets. Un projet garde des copies de ce qu'il utilise, il usine donc pareil dans un an et sur un autre ordinateur.
library-view-preferences = Préférences
library-view-preferences-hover = La fiche de réglage, la mise à jour automatique du parcours d'outil, le thème de couleurs
library-view-no-settings-folder = Pas de dossier des réglages
# $var is the name of an environment variable.
library-view-no-settings-folder-hover = Définissez { $var } pour garder une bibliothèque
library-view-tools = Outils
library-view-tools-hover = Votre bibliothèque d'outils : glissez un outil sur les Outils du projet pour l'utiliser
library-view-machines = Machines
library-view-machines-hover = Vos profils de machine : les valeurs de départ des nouveaux projets
library-view-tool-hover = Cliquer pour le modifier ; le glisser sur les Outils du projet pour l'y utiliser

# Commands on a library tool or machine profile.
library-view-add-to-project = Ajouter au projet
library-view-add-to-project-hover = Le copier dans le projet comme outil à part entière
library-view-use-in-project = Utiliser dans ce projet
library-view-use-in-project-hover = Copier ses valeurs dans la machine du projet (annuler avec Ctrl+Z)
library-view-entry-disabled-hover = Ouvrez un projet, et enregistrez ou annulez d'abord les modifications
# A button: make this entry the one new projects start with.
library-view-make-default = Par défaut
library-view-make-default-hover = Les nouveaux projets démarrent avec
library-view-delete-hover = Le retirer de votre bibliothèque ; les projets gardent leurs copies
library-view-save-hover = Écrire les modifications dans votre bibliothèque
library-view-revert = Rétablir
library-view-revert-hover = Abandonner les modifications
library-view-not-saved = non enregistré

library-view-tool-library = Bibliothèque d'outils
library-view-tool-library-about = Un projet garde une copie de chaque outil qu'il utilise. Cliquez un outil sous Cet ordinateur pour le modifier ; glissez-le sur les Outils du projet, ou faites un clic droit dessus, pour l'utiliser.
library-view-machines-about = Un projet garde une copie des valeurs de sa machine. Clic droit sur un profil pour l'utiliser dans ce projet ; celui par défaut est celui avec lequel démarrent les nouveaux projets.
library-view-tool-count = { $count ->
    [one] { $count } outil
   *[other] { $count } outils
} dans votre bibliothèque.
library-view-machine-count = { $count ->
    [one] { $count } machine
   *[other] { $count } machines
} dans votre bibliothèque.

library-view-library-tool = Outil de la bibliothèque
library-view-tool-gone = Cet outil n'est plus dans votre bibliothèque.
library-view-machine-profile = Profil de machine
library-view-profile-gone = Ce profil n'est plus dans votre bibliothèque.
library-view-entry-about = Dans votre bibliothèque, pour tous les projets. Les modifications sont gardées quand vous appuyez sur Enregistrer.
# $tools lists the project's tools, like "T1, T3".
library-view-project-copies = Ce projet en a une copie : { $tools }
library-view-project-machine = La machine de ce projet vient de ce profil.

library-view-deleted = « { $name } » retiré de votre bibliothèque ; les projets gardent leurs copies
library-view-saved = Enregistré dans votre bibliothèque

# Notes at the right end of a library entry's row in the Browser.
library-view-note-default = par défaut
library-view-note-unsaved = non enregistré
