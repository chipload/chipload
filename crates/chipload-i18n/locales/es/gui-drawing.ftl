# Drawing, changing and deleting shapes (crates/chipload-gui/src/app/drawing.rs).
# Shapes are named S and their number: S3.

# The kind of a new shape when it isn't known; normally the kind is "rectangle", "circle", …
drawing-shape = forma
# $kind: the kind of shape ("rectangle", "circle", …); $layer: the layer's name.
drawing-added = Añadido: { $kind } S{ $id } en la capa { $layer }
# The kind of shape just drawn, in "Added … S3 on layer …".
drawing-kind-circle = círculo
drawing-kind-rectangle = rectángulo
drawing-kind-polyline = polilínea
# Followed by the error on its own line.
drawing-add-failed = No se puede añadir la forma:
# Shown under the shape form when its values can't be applied.
drawing-no-shape-selected = ninguna forma seleccionada
drawing-no-project = ningún proyecto

drawing-imported-not-deletable = Las formas importadas no se pueden eliminar aquí; cambia el dibujo y usa Actualizar dibujo desde DXF.
# $shapes: the deleted shapes, such as "S1, S4".
drawing-deleted-report = { $count ->
        [one] Forma eliminada:
       *[other] Formas eliminadas:
    } { $shapes }.
drawing-deleted-status = { $count ->
        [one] Forma eliminada:
       *[other] Formas eliminadas:
    } { $shapes }
drawing-imported-kept = Se conservaron las formas importadas de la selección (solo cambian con el DXF).
drawing-kind-slot = ranura
