# File > Import image… and the window that traces a picture into shapes
# (crates/chipload-gui/src/app/images.rs).

# The name of picture files in the file dialog's filter.
images-pictures-filter = Images
images-cannot-read = Impossible de lire { $path } : { $error }
# Followed by the error on its own line.
images-cannot-import = Impossible d'importer { $path } :
# Followed by the error on its own line.
images-trace-at-size-failed = Impossible de vectoriser l'image à cette taille :
images-tracing-failed = la vectorisation de l'image a échoué
# After a picture is traced into the project. "V-carve (ribbon)" is the ribbon's V-carve button.
images-traced-next = L'image est sélectionnée : Gravure en V (ruban) la grave. Réglez sa taille et sa position dans Propriétés.
images-traced = { $file } vectorisé
images-removed = Image { $id } retirée

# Window titles. $file: the picture's file name.
images-import-title = Importer une image : { $file }
# "Trace" is a verb here: trace this picture again.
images-trace-title = Vectoriser { $file }
# The window's button, a verb.
images-import = Importer

# Under the preview. $zoom: such as "400%".
images-zoom-note = Zoom { $zoom } · glisser pour déplacer · double-clic pour ajuster
images-scroll-to-zoom = Molette pour zoomer

# The settings.
images-threshold = Seuil
images-threshold-hover = Les niveaux de gris en dessous sont noirs (0 noir, 255 blanc). Auto choisit la valeur qui sépare le mieux les parties sombres et claires.
# Checkbox: pick the threshold automatically.
images-threshold-auto = auto
images-invert = Inverser
images-invert-check = graver les parties claires
images-blur = Flou
images-blur-hover = Lisse les bords irréguliers et le bruit avant le seuil.
images-smallest-piece = Plus petit morceau
images-smallest-piece-hover = Les taches noires ou blanches plus petites que ceci sont supprimées.
images-smoothing = Lissage
images-smoothing-hover = Jusqu'où un bord vectorisé peut être redressé par rapport au bord de l'image.
# What tracing makes, a noun.
images-trace = Tracé
images-trace-hover = Contours autour des parties noires, pour une gravure en V ; ou lignes médianes des traits noirs (écriture, dessin au trait), pour un suivi de tracé.
images-trace-outlines = Contours (gravure en V)
images-trace-centre-lines = Lignes médianes (suivi de tracé)
images-arcs = Arcs
images-arcs-hover = Ajuster des arcs en plus des lignes : programmes plus courts et mouvements plus fluides pour les contours, poches et suivis de tracé.
images-arcs-check = ajuster des arcs

# What the preview traced.
images-tracing = Vectorisation…
images-found-shapes = { $count ->
        [one] { $count } forme
       *[other] { $count } formes
    }
# $threshold: the grey value the automatic threshold picked, 1 to 254.
images-found-shapes-auto = { $count ->
        [one] { $count } forme
       *[other] { $count } formes
    } (seuil { $threshold } choisi)
images-found-centre-lines = { $count ->
        [one] { $count } ligne médiane
       *[other] { $count } lignes médianes
    }
images-found-centre-lines-auto = { $count ->
        [one] { $count } ligne médiane
       *[other] { $count } lignes médianes
    } (seuil { $threshold } choisi)
# The narrowest and widest strokes; $wide carries the unit.
images-stroke-widths = Traits de { $narrow } à { $wide } de large
images-dots-left-out = { $count ->
        [one] { $count } point exclu
       *[other] { $count } points exclus
    }
# Traced pieces where no arcs were fitted.
images-kept-as-lines = { $count ->
        [one] { $count } gardé en lignes
       *[other] { $count } gardés en lignes
    }
images-specks-dropped = { $count ->
        [one] { $count } tache supprimée
       *[other] { $count } taches supprimées
    }
images-black-at-border = Du noir touche le bord : inversez si c'est le fond.
