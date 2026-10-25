# The form for a drawn shape's values, and the lines about an imported shape
# (crates/chipload-gui/src/shape_form.rs).

# The form's heading: the kind of shape and its id (S3).
shape-form-title-path = Pfad S{ $id }
shape-form-title-circle = Kreis S{ $id }
shape-form-title-rectangle = Rechteck S{ $id }
shape-form-title-polyline = Polylinie S{ $id }

# The drawing layer the shape is on.
shape-form-layer = Layer
shape-form-radius = Radius
shape-form-length = Länge (X)
shape-form-width = Breite (Y)
shape-form-angle = Winkel
# Whether the polyline is a closed loop; the checkbox's text follows.
shape-form-closed = Geschlossen
shape-form-closed-checkbox = zurück zum ersten Punkt
# Del is the keyboard key.
shape-form-delete = Form löschen  (Del)
shape-form-delete-hover = Bearbeitungen, die sie verwenden, werden markiert, bis ihre Geometrie neu gewählt ist

# How to edit a selected polyline on the canvas. A bulge bends a segment into an arc;
# Alt and Shift are keyboard keys.
shape-form-polyline-help = Auf der Zeichenfläche: ein Quadrat ziehen verschiebt einen Punkt · einen runden Mittelgriff ziehen fügt dort einen neuen Punkt ein · Shift+Ziehen an einem Mittelgriff biegt das Segment zu einem Bogen (Wölbung) · der gestrichelte Rahmen verschiebt, dreht und skaliert sie · Alt: ohne Fang

shape-form-corner-radius = Eckenradius
shape-form-corner-radius-hover = Rundet alle vier Ecken; höchstens die Hälfte der kürzeren Seite
# A button that sets the corner radius back to 0.
shape-form-reset = Zurücksetzen
shape-form-sharp-corners = Scharfe Ecken
# The centre point of a circle or rectangle.
shape-form-centre = Mittelpunkt

# The column of the points table: how much the segment to the next point is bent.
shape-form-bulge = Wölbung
shape-form-bulge-hover = Biegung des Segments zum nächsten Punkt: 0 gerade, 1 ein Halbkreis (gegen den Uhrzeigersinn), -1 andersherum
shape-form-insert-point = Nach diesem einen Punkt einfügen
shape-form-delete-point = Diesen Punkt löschen

# An imported shape (from a DXF) can't be edited.
shape-form-imported = Form S{ $id } (importiert, schreibgeschützt)
# The DXF entity the shape comes from: its type (LINE, ARC, …) and handle.
shape-form-imported-from = Layer { $layer }, DXF { $entity } { $handle }
# Shown when the DXF entity's type is not known.
shape-form-entity = Element
