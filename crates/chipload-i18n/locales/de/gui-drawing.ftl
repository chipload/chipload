# Drawing, changing and deleting shapes (crates/chipload-gui/src/app/drawing.rs).
# Shapes are named S and their number: S3.

# The kind of a new shape when it isn't known; normally the kind is "rectangle", "circle", …
drawing-shape = Form
# $kind: the kind of shape ("rectangle", "circle", …); $layer: the layer's name.
drawing-added = { $kind } S{ $id } auf Layer { $layer } hinzugefügt
# The kind of shape just drawn, in "Added … S3 on layer …".
drawing-kind-circle = Kreis
drawing-kind-rectangle = Rechteck
drawing-kind-polyline = Polylinie
# Followed by the error on its own line.
drawing-add-failed = Die Form kann nicht hinzugefügt werden:
# Shown under the shape form when its values can't be applied.
drawing-no-shape-selected = keine Form ausgewählt
drawing-no-project = kein Projekt

drawing-imported-not-deletable = Importierte Formen können hier nicht gelöscht werden; ändern Sie die Zeichnung und verwenden Sie Zeichnung aus DXF aktualisieren.
# $shapes: the deleted shapes, such as "S1, S4".
drawing-deleted-report = { $count ->
        [one] Form
       *[other] Formen
    } { $shapes } gelöscht.
drawing-deleted-status = { $count ->
        [one] Form
       *[other] Formen
    } { $shapes } gelöscht
drawing-imported-kept = Importierte Formen in der Auswahl wurden behalten (sie ändern sich nur mit der DXF).
drawing-kind-slot = Langloch
drawing-kind-polygon = Vieleck
