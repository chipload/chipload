# Recovery copies of unsaved work, and the window that offers them at start
# (crates/chipload-gui/src/app/recovery.rs).

recovery-copies-off = Les copies de récupération sont désactivées : { $error }
# Followed by the reason on the next line.
recovery-cannot-open = Impossible d'ouvrir la copie de récupération :
recovery-recovered = Travail non enregistré récupéré : { $title }. Enregistrez-le pour le garder.

recovery-title = Récupérer le travail non enregistré
recovery-intro = Chipload s'est fermé sans enregistrer ces modifications :
# A copy of a project that was never saved to a file.
recovery-untitled = Sans titre (jamais enregistré)

# When the unsaved changes in a copy were made.
recovery-changes-unknown = Modifications d'une date inconnue
recovery-changes-moments = Modifications d'il y a moins d'une minute
recovery-changes-minutes = { $count ->
    [one] Modifications d'il y a { $count } minute
   *[other] Modifications d'il y a { $count } minutes
}
recovery-changes-hours = { $count ->
    [one] Modifications d'il y a { $count } heure
   *[other] Modifications d'il y a { $count } heures
}
recovery-changes-days = { $count ->
    [one] Modifications d'il y a { $count } jour
   *[other] Modifications d'il y a { $count } jours
}

recovery-saved-since = Le fichier du projet a été enregistré depuis la création de cette copie.
recovery-cannot-be-opened = Impossible à ouvrir : { $problem }

# A button: open the copy.
recovery-open = Ouvrir
recovery-open-hover = L'ouvrir comme projet non enregistré ; enregistrez-le pour le garder
recovery-open-dirty = Enregistrez ou fermez d'abord le projet en cours
recovery-open-unreadable = Cette copie est illisible
# A button: decide about the copy another time.
recovery-later = Plus tard
recovery-later-hover = La garder et redemander la prochaine fois
