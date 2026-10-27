# Drawing, changing and deleting shapes (crates/chipload-gui/src/app/drawing.rs).
# Shapes are named S and their number: S3.

# The kind of a new shape when it isn't known; normally the kind is "rectangle", "circle", …
drawing-shape = forme
# $kind: the kind of shape ("rectangle", "circle", …); $layer: the layer's name.
drawing-added = Ajouté : { $kind } S{ $id } sur le calque { $layer }
# The kind of shape just drawn, in "Added … S3 on layer …".
drawing-kind-circle = cercle
drawing-kind-rectangle = rectangle
drawing-kind-polyline = polyligne
# Followed by the error on its own line.
drawing-add-failed = Impossible d'ajouter la forme :
# Shown under the shape form when its values can't be applied.
drawing-no-shape-selected = aucune forme sélectionnée
drawing-no-project = aucun projet

drawing-imported-not-deletable = Les formes importées ne peuvent pas être supprimées ici ; modifiez le dessin et utilisez Mettre à jour le dessin depuis un DXF.
# $shapes: the deleted shapes, such as "S1, S4".
drawing-deleted-report = { $count ->
        [one] Forme supprimée :
       *[other] Formes supprimées :
    } { $shapes }.
drawing-deleted-status = { $count ->
        [one] Forme supprimée :
       *[other] Formes supprimées :
    } { $shapes }
drawing-imported-kept = Les formes importées de la sélection ont été gardées (elles ne changent qu'avec le DXF).
drawing-kind-slot = rainure
