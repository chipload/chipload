# The header: the top bar and the ribbon of grouped buttons (crates/chipload-gui/src/app/header.rs).

# Export's hover text. $count is the number of parts that are cut (always more than one),
# $part the name of the part the preview shows.
header-export-hover = Enregistrer le G-code vérifié
header-export-hover-parts = Enregistrer le G-code vérifié de chaque pièce usinée ({ $count } pièces)
header-export-hover-parts-shown = Enregistrer le G-code vérifié de chaque pièce usinée ({ $count } pièces) ; l'aperçu ne montre que « { $part } »
header-export-generating-hover = Enregistrer le G-code une fois calculé
header-export-first-hover = Calculer d'abord

# The top bar's icon buttons.
header-open-hover = Ouvrir un projet
header-save-hover = Enregistrer le projet
header-undo = Annuler
header-redo = Rétablir
header-help = Aide
# The colour theme button: what is set now, a click goes on to the next.
header-theme-system-hover = Thème de couleurs : selon le système. Cliquer pour le suivant.
header-theme-light-hover = Thème de couleurs : clair. Cliquer pour le suivant.
header-theme-dark-hover = Thème de couleurs : sombre. Cliquer pour le suivant.

# The tab with the open project's file name.
header-new-project = Nouveau projet
header-tab-unsaved-hover =
    { $path }
    Modifications non enregistrées
header-tab-not-saved-hover = Pas encore enregistré

# The ribbon's groups: their captions (shown in capitals) open the menu of the same name.
header-group-file = Fichier
header-group-draw = Dessiner
header-group-operation = Opération
header-group-library = Bibliothèque
header-group-view = Vue
header-group-hover = Tout dans { $group }

# FILE: short words under icons.
header-new = Nouveau
header-new-hover = Nouveau projet
header-open = Ouvrir
header-import = Importer
header-import-hover = Nouveau projet à partir d'un DXF
header-save = Enregistrer

# DRAW: short words under icons, and what each does.
header-circle = Cercle
header-circle-hover = Dessiner un cercle : centre, puis un point du cercle
header-rectangle = Rectangle
header-rectangle-hover = Dessiner un rectangle : deux coins opposés
header-polyline = Polyligne
header-polyline-hover = Dessiner une polyligne : cliquer les points
header-text = Texte
header-text-hover = Écrire un texte : cliquer là où il commence, puis le saisir dans Propriétés
header-image = Image
header-image-hover = Vectoriser une image en formes, pour une gravure en V ou un suivi de tracé
header-add-dxf = Ajouter DXF
header-add-dxf-hover = Une autre pièce : un DXF placé et mis à jour séparément
# "FILE > Import": the ribbon group FILE and its Import button.
header-add-dxf-first-hover = Importez d'abord un DXF (FICHIER > Importer)
header-import-image-menu = Importer une image…
header-add-dxf-menu = Ajouter un DXF au projet…

# OPERATION: "Follow" is the short word under the icon for a Follow path operation.
header-follow = Suivi
# $operation is what the operation does (a hover text of its own).
header-needs-selection-hover = { $operation }. Sélectionnez d'abord des formes.
header-select-shapes-first = Sélectionnez d'abord des formes
header-new-profile = Nouveau contour
header-new-pocket = Nouvelle poche
header-new-vcarve = Nouvelle gravure en V
header-new-follow-path = Nouveau suivi de tracé
header-new-holes = Nouveaux perçages
header-new-surface = Nouveau surfaçage

# LIBRARY
header-tools = Outils
header-tools-hover = Votre bibliothèque d'outils, sous Cet ordinateur dans le Navigateur
header-machines = Machines
header-machines-hover = Vos profils de machine, sous Cet ordinateur dans le Navigateur
header-tool-library = Bibliothèque d'outils

# VIEW
header-measure = Mesurer
header-measure-hover = Deux clics donnent la distance, ΔX, ΔY et l'angle. Le résultat s'affiche dans la barre du canevas.

# Generate and Export at the ribbon's right end.
header-export = Exporter
header-cancel = Annuler
header-cancel-hover = Arrêter le calcul
header-generate = Calculer
header-generate-hover = Calculer les parcours d'outil, écrire et vérifier le G-code
header-arc = Arc
header-arc-hover = Dessiner un arc : début, fin, puis un point par où il passe
header-slot = Rainure
header-slot-hover = Dessiner une rainure : les centres de ses extrémités, puis sa largeur
header-polygon = Polygone
header-polygon-hover = Dessiner un polygone régulier : le centre, puis un sommet (le nombre de côtés dans la barre d'état)
