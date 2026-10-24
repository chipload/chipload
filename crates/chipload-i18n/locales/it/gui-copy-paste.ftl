# Copy, paste and mirror drawn shapes (crates/chipload-gui/src/app/copy_paste.rs).
# Drawn shapes are drawn in Chipload; imported ones come from a DXF, a picture or a text.

copy-paste-imported-no-copy = Le forme importate non si possono copiare; solo quelle disegnate
copy-paste-select-to-copy = Seleziona forme disegnate da copiare
copy-paste-copied = { $count ->
    [one] Copiata { $count } forma
   *[other] Copiate { $count } forme
}

copy-paste-no-shapes = Gli appunti non contengono forme
copy-paste-copy-first = Copia prima delle forme disegnate (Ctrl+C)
# $shapes: the new shapes' names, e.g. S4, S5.
copy-paste-pasted = Incollate { $shapes }
# Followed by the reason on the next line.
copy-paste-cannot-paste = Impossibile incollare:

copy-paste-imported-no-mirror = Le forme importate non si possono specchiare; solo quelle disegnate
copy-paste-select-to-mirror = Seleziona forme disegnate da specchiare
copy-paste-mirrored = Specchiate
copy-paste-mirrored-drawn = Specchiate (le forme importate nella selezione restano come sono)
# Followed by the reason on the next line.
copy-paste-cannot-mirror = Impossibile specchiare:
