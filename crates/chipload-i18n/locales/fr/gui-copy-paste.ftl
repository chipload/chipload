# Copy, paste and mirror drawn shapes (crates/chipload-gui/src/app/copy_paste.rs).
# Drawn shapes are drawn in Chipload; imported ones come from a DXF, a picture or a text.

copy-paste-imported-no-copy = Les formes importées ne peuvent pas être copiées ; seulement les formes dessinées
copy-paste-select-to-copy = Sélectionnez des formes dessinées à copier
copy-paste-copied = { $count ->
    [one] { $count } forme copiée
   *[other] { $count } formes copiées
}

copy-paste-no-shapes = Le presse-papiers ne contient aucune forme
copy-paste-copy-first = Copiez d'abord des formes dessinées (Ctrl+C)
# $shapes: the new shapes' names, e.g. S4, S5.
copy-paste-pasted = Collé : { $shapes }
# Followed by the reason on the next line.
copy-paste-cannot-paste = Impossible de coller :

copy-paste-imported-no-mirror = Les formes importées ne peuvent pas être mises en miroir ; seulement les formes dessinées
copy-paste-select-to-mirror = Sélectionnez des formes dessinées à mettre en miroir
copy-paste-mirrored = Miroir appliqué
copy-paste-mirrored-drawn = Miroir appliqué (les formes importées de la sélection restent telles quelles)
# Followed by the reason on the next line.
copy-paste-cannot-mirror = Impossible d'appliquer le miroir :
