# The library rows of a project tool and the machine, and copying values into and out of the project (crates/chipload-gui/src/app/user_library.rs).

# The user's library of tools and machine profiles, kept on this computer.
user-library-profile = Profil
user-library-tool = Outil de la bibliothèque
user-library-same = identique à votre bibliothèque
user-library-differs = diffère de votre bibliothèque : { $changes }
user-library-more-hover = Enregistrer comme nouvelle entrée, ou l'afficher dans la bibliothèque
user-library-use-profile = Utiliser un profil…
user-library-use-tool = Utiliser un outil de la bibliothèque…
user-library-save-to-library = Enregistrer dans la bibliothèque
user-library-save-to-library-named = Enregistrer dans la bibliothèque…
user-library-save-to-library-hover = Copie ces valeurs dans votre bibliothèque, pour d'autres projets
user-library-save-to-linked-profile-hover = Écrire ces valeurs dans le profil auquel la machine est liée, pour d'autres projets
user-library-save-to-linked-tool-hover = Écrire ces valeurs dans l'outil de la bibliothèque auquel il est lié, pour d'autres projets
user-library-save-as-new = Enregistrer comme nouveau…
user-library-save-as-new-hover = Garder ces valeurs dans votre bibliothèque sous un autre nom
user-library-update-from-library = Mettre à jour depuis la bibliothèque
user-library-update-from-library-hover = Copier les valeurs de la bibliothèque dans le projet (annuler avec Ctrl+Z)
user-library-show-in-library = Afficher dans la bibliothèque
user-library-show-in-library-hover = L'ouvrir sous Cet ordinateur dans le Navigateur
# $var is the name of an environment variable.
user-library-no-settings-folder-set = Aucun dossier des réglages trouvé ; définissez { $var }
user-library-no-settings-folder-use = Aucun dossier des réglages trouvé ; définissez { $var } pour utiliser une bibliothèque.
user-library-no-settings-folder = Pas de dossier des réglages
user-library-kept-in = Gardé dans { $path }

user-library-name = Nom
# The tool's number (T1, T2, …) in a project.
user-library-tool-number = Numéro d'outil
user-library-tool-number-hover = Le numéro qu'il prend dans un projet où ce numéro est libre
user-library-cutting-values = Valeurs de coupe
user-library-cutting-values-hover = Reprises dans une nouvelle opération avec cet outil quand il n'y a pas d'opération au-dessus à copier. Ne modifie jamais une opération existante.
user-library-feed = Avance
user-library-plunge-feed = Avance plongée
user-library-step-down = Prof. de passe
user-library-stepover = Pas latéral
user-library-safe-z = Z de sécurité
user-library-max-overcut = Prof. max sous le brut
user-library-max-overcut-hover = Jusqu'où une coupe peut descendre sous le brut (dans le martyr)

user-library-settings-not-saved = Réglages non enregistrés : { $error }
user-library-recent-gone = { $path } n'existe plus ; il a été retiré de la liste des récents.
# Followed by the reason on the next line.
user-library-post-named = Post-processeur « { $name } » :
user-library-post-chosen = Post-processeur : { $name }
user-library-no-machine-profile = Il n'y a pas de profil de machine « { $name } ».
user-library-no-library-tool = Il n'y a pas d'outil « { $name } » dans la bibliothèque.
user-library-no-project-tool = Le projet n'a pas d'outil { $id }.
user-library-machine-profile-named = Profil de machine « { $name } »
user-library-library-tool-named = Outil de la bibliothèque « { $name } »
# $what is user-library-machine-profile-named or user-library-library-tool-named.
user-library-no-values-changed-status = { $what } : aucune valeur modifiée
user-library-values-changed-status = { $what } : { $count ->
        [one] { $count } valeur modifiée
       *[other] { $count } valeurs modifiées
    }
user-library-changes-heading = { $what } :
user-library-no-values-changed = aucune valeur modifiée

user-library-profile-gone = Le profil d'origine n'est pas dans votre bibliothèque.
user-library-library-tool-gone = L'outil d'origine n'est pas dans votre bibliothèque.
user-library-no-tool-to-save = Le projet n'a pas d'outil à enregistrer.
# Followed by the reason on the next line.
user-library-not-saved = Non enregistré dans la bibliothèque :
user-library-saved = « { $name } » enregistré dans votre bibliothèque

# Dialog titles.
user-library-save-machine-title = Enregistrer le profil de machine
user-library-save-tool-title = Enregistrer l'outil dans la bibliothèque
user-library-name-in-library = Nom dans votre bibliothèque :
user-library-replaces = Remplace « { $name } » dans votre bibliothèque.
# Buttons.
user-library-replace = Remplacer

user-library-tool-exists = Il y a déjà un outil « { $name } ».
user-library-machine-exists = Il y a déjà une machine « { $name } ».
# The names a new library entry gets ("New tool 2" if taken).
user-library-new-tool = Nouvel outil
user-library-new-machine = Nouvelle machine
