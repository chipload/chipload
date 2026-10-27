# Drawing, changing and deleting shapes (crates/chipload-gui/src/app/drawing.rs).
# Shapes are named S and their number: S3.

# The kind of a new shape when it isn't known; normally the kind is "rectangle", "circle", …
drawing-shape = form
# $kind: the kind of shape ("rectangle", "circle", …); $layer: the layer's name.
drawing-added = Lade till { $kind } S{ $id } på lager { $layer }
# The kind of shape just drawn, in "Added … S3 on layer …".
drawing-kind-circle = cirkel
drawing-kind-rectangle = rektangel
drawing-kind-polyline = polylinje
# Followed by the error on its own line.
drawing-add-failed = Kan inte lägga till formen:
# Shown under the shape form when its values can't be applied.
drawing-no-shape-selected = ingen form markerad
drawing-no-project = inget projekt

drawing-imported-not-deletable = Importerade former kan inte raderas här; ändra ritningen och använd Uppdatera ritning från DXF.
# $shapes: the deleted shapes, such as "S1, S4".
drawing-deleted-report = Raderade { $count ->
        [one] formen
       *[other] formerna
    } { $shapes }.
drawing-deleted-status = Raderade { $count ->
        [one] formen
       *[other] formerna
    } { $shapes }
drawing-imported-kept = Importerade former i markeringen behölls (de ändras bara med DXF:en).
drawing-kind-slot = spår
drawing-kind-polygon = polygon
