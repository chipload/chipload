# The form for a drawn shape's values, and the lines about an imported shape
# (crates/chipload-gui/src/shape_form.rs).

# The form's heading: the kind of shape and its id (S3).
shape-form-title-path = path S{ $id }
shape-form-title-circle = Circle S{ $id }
shape-form-title-rectangle = Rectangle S{ $id }
shape-form-title-polyline = Polyline S{ $id }

# The drawing layer the shape is on.
shape-form-layer = Layer
shape-form-radius = Radius
shape-form-length = Length (X)
shape-form-width = Width (Y)
shape-form-angle = Angle
# Whether the polyline is a closed loop; the checkbox's text follows.
shape-form-closed = Closed
shape-form-closed-checkbox = back to the first point
# Del is the keyboard key.
shape-form-delete = Delete shape  (Del)
shape-form-delete-hover = Operations that use it are flagged until their geometry is chosen again

# How to edit a selected polyline on the canvas. A bulge bends a segment into an arc;
# Alt and Shift are keyboard keys.
shape-form-polyline-help = On the canvas: drag a square to move a point · drag a round middle handle to insert a new point there · Shift+drag a middle handle to bend the segment into an arc (bulge) · Alt: no snapping

shape-form-corner-radius = Corner radius
shape-form-corner-radius-hover = Rounds all four corners; at most half the shorter side
# A button that sets the corner radius back to 0.
shape-form-reset = Reset
shape-form-sharp-corners = Sharp corners
# The centre point of a circle or rectangle.
shape-form-centre = Centre

# The column of the points table: how much the segment to the next point is bent.
shape-form-bulge = Bulge
shape-form-bulge-hover = Bend of the segment to the next point: 0 straight, 1 a half circle (counter-clockwise), -1 the other way
shape-form-insert-point = Insert a point after this one
shape-form-delete-point = Delete this point

# An imported shape (from a DXF) can't be edited.
shape-form-imported = Shape S{ $id } (imported, read-only)
# The DXF entity the shape comes from: its type (LINE, ARC, …) and handle.
shape-form-imported-from = Layer { $layer }, DXF { $entity } { $handle }
# Shown when the DXF entity's type is not known.
shape-form-entity = entity
