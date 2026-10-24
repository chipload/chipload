# Drawing, changing and deleting shapes (crates/chipload-gui/src/app/drawing.rs).
# Shapes are named S and their number: S3.

# The kind of a new shape when it isn't known; normally the kind is "rectangle", "circle", …
drawing-shape = shape
# $kind: the kind of shape ("rectangle", "circle", …); $layer: the layer's name.
drawing-added = Added { $kind } S{ $id } on layer { $layer }
# The kind of shape just drawn, in "Added … S3 on layer …".
drawing-kind-circle = circle
drawing-kind-rectangle = rectangle
drawing-kind-polyline = polyline
# Followed by the error on its own line.
drawing-add-failed = Cannot add the shape:
# Shown under the shape form when its values can't be applied.
drawing-no-shape-selected = no shape selected
drawing-no-project = no project

drawing-imported-not-deletable = Imported shapes can't be deleted here; change the drawing and use Update drawing from DXF.
# $shapes: the deleted shapes, such as "S1, S4".
drawing-deleted-report = Deleted { $count ->
        [one] shape
       *[other] shapes
    } { $shapes }.
drawing-deleted-status = Deleted { $count ->
        [one] shape
       *[other] shapes
    } { $shapes }
drawing-imported-kept = Imported shapes in the selection were kept (they change only with the DXF).
