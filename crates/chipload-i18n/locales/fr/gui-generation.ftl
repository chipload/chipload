# Generating the toolpath and G-code, and the live preview
# (crates/chipload-gui/src/app/generation.rs).

generation-no-project = Importez d'abord un DXF ou SVG ou ouvrez un projet.
generation-could-not-start = Le calcul n'a pas pu démarrer : { $error }
generation-cancelled = Calcul annulé
# What went wrong when the planning stopped without an answer.
generation-worker-stopped = le calcul s'est arrêté sans résultat

generation-check-failed = Vérification du G-code ÉCHOUÉE. Rien ne peut être exporté.
# Followed by the reason on the next line.
generation-failed = Échec du calcul :
generation-failed-unexpectedly = Échec inattendu du calcul : { $error }
generation-please-report = Votre projet est inchangé. Merci de signaler ce problème avec le fichier du projet.

# $post: the post-processor's name, e.g. GRBL.
generation-done = { $lines } lignes calculées pour { $post }. Vérification réussie.
generation-done-files = { $lines } lignes calculées dans { $files } fichiers, un par groupe d'outils, pour { $post }. Vérification réussie.
# A warning from the G-code checker.
generation-check-warning = Vérification : { $message }
# Where X, Y and Z zero are on the stock.
generation-work-zero = Origine pièce : { $zero }
# "All parts" is a switch on the bar above the canvas.
generation-one-part-shown = Affiché : pièce « { $part } » seulement (Toutes les pièces, dans la barre du canevas, les montre toutes). L'export écrit chaque pièce usinée.

generation-export-when-done = Export dès que le parcours d'outil est calculé
