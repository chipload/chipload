# "This computer" in the Browser: Preferences and post-processors (crates/chipload-gui/src/app/computer.rs).

computer-posts = Post-processeurs
computer-posts-hover = Ce qui transforme les parcours d'outil en G-code pour votre machine : intégrés et les vôtres
# Notes at the right end of a post's row in the Browser.
computer-post-built-in = intégré
computer-post-yours = le vôtre
computer-post-this-project = ce projet

computer-preferences = Préférences
computer-preferences-about = Pour tous les projets sur cet ordinateur.
computer-settings-folder = Dossier des réglages
# $var is the name of an environment variable.
computer-settings-folder-none = aucun : définissez { $var } pour garder réglages et bibliothèque
computer-setup-sheet = Fiche de réglage
computer-setup-sheet-hover = Une page HTML pour l'opérateur à côté des fichiers exportés : brut, zéro, outils dans l'ordre, opérations, temps et avertissements
computer-setup-sheet-write = en écrire une avec le G-code
computer-toolpath = Parcours d'outil
computer-toolpath-auto = mise à jour automatique (aussi dans le menu Vue)
computer-toolpath-auto-hover = Calculer tout seul une demi-seconde après chaque modification
computer-units = Unités
computer-units-hover = Unités d'affichage et de saisie des longueurs et avances. Les projets restent en mm ; les unités du G-code sont celles de la machine (Paramètres d'usinage > Machine)
computer-theme = Thème de couleurs
computer-theme-system = Système
computer-theme-light = Clair
computer-theme-dark = Sombre

computer-posts-count = { $builtin } intégrés, { $yours } à vous.
computer-posts-folder = Les vôtres vont dans { $folder }
computer-posts-no-folder = Pas de dossier des réglages, donc seulement les post-processeurs intégrés.
# NAME.toml is a file name: keep it as it is.
computer-posts-about = Votre propre post-processeur tient en deux fichiers : NAME.toml avec ses réglages, et le modèle qu'il nomme. Partez d'une copie d'un post-processeur intégré ; la page Post-processors du guide explique les deux.
computer-posts-reload = Relire le dossier
computer-posts-reload-hover = Après avoir ajouté ou modifié les fichiers d'un post-processeur

computer-post = Post-processeur
computer-post-gone = Ce post-processeur n'existe plus.
computer-post-unusable = Inutilisable : { $error }
computer-post-use = Utiliser dans ce projet
computer-post-use-hover = En faire le post-processeur du projet
computer-post-use-copy-hover = En faire le post-processeur du projet : une copie va dans le projet
computer-post-in-use = Le projet l'utilise
computer-post-no-project = Ouvrez d'abord un projet
