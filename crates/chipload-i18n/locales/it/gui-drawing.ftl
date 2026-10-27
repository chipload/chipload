# Drawing, changing and deleting shapes (crates/chipload-gui/src/app/drawing.rs).
# Shapes are named S and their number: S3.

# The kind of a new shape when it isn't known; normally the kind is "rectangle", "circle", …
drawing-shape = forma
# $kind: the kind of shape ("rectangle", "circle", …); $layer: the layer's name.
drawing-added = Aggiunto: { $kind } S{ $id } sul layer { $layer }
# The kind of shape just drawn, in "Added … S3 on layer …".
drawing-kind-circle = cerchio
drawing-kind-rectangle = rettangolo
drawing-kind-polyline = polilinea
# Followed by the error on its own line.
drawing-add-failed = Impossibile aggiungere la forma:
# Shown under the shape form when its values can't be applied.
drawing-no-shape-selected = nessuna forma selezionata
drawing-no-project = nessun progetto

drawing-imported-not-deletable = Le forme importate non si possono eliminare qui; modifica il disegno e usa Aggiorna disegno da DXF.
# $shapes: the deleted shapes, such as "S1, S4".
drawing-deleted-report = { $count ->
        [one] Eliminata la forma
       *[other] Eliminate le forme
    } { $shapes }.
drawing-deleted-status = { $count ->
        [one] Eliminata la forma
       *[other] Eliminate le forme
    } { $shapes }
drawing-imported-kept = Le forme importate nella selezione sono state mantenute (cambiano solo con il DXF).
drawing-kind-slot = asola
