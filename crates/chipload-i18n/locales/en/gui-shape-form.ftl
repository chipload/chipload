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
shape-form-polyline-help = On the canvas: drag a square to move a point · drag a line (or its round middle handle) to move that segment · Ctrl+drag it to insert a new point there · Shift+drag it to bend the segment into an arc (bulge) · the dashed box round it moves, turns and scales it · Alt: no snapping

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
shape-form-position = Position
shape-form-position-hover = The lower-left corner of the box round it; typing a value moves it there
shape-form-turn = Turn
shape-form-turn-ccw-hover = Turn a quarter counter-clockwise about the middle of its box
shape-form-turn-cw-hover = Turn a quarter clockwise about the middle of its box
shape-form-turn-by = by{" "}
shape-form-turn-by-hover = Type an angle and press Enter to turn it by that much (counter-clockwise; negative turns clockwise)
shape-form-name = Name
shape-form-name-hover = Its name in the Browser; empty: shown by its number
shape-form-title-slot = Slot S{ $id }
shape-form-title-polygon = Polygon S{ $id }
shape-form-sides = Sides
shape-form-polygon-radius = Radius
shape-form-polygon-radius-hover = From the centre to a corner
shape-form-across-flats = Across flats
shape-form-across-flats-hover = From one side to the opposite one (twice the distance from the centre to a side's middle); changing it changes the radius
shape-form-polygon-angle = Angle
shape-form-polygon-angle-hover = Where the first corner points, counter-clockwise from +X
