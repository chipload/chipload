# The form for a drawn shape's values, and the lines about an imported shape
# (crates/chipload-gui/src/shape_form.rs).

# The form's heading: the kind of shape and its id (S3).
shape-form-title-path = bana S{ $id }
shape-form-title-circle = Cirkel S{ $id }
shape-form-title-rectangle = Rektangel S{ $id }
shape-form-title-polyline = Polylinje S{ $id }

# The drawing layer the shape is on.
shape-form-layer = Lager
shape-form-radius = Radie
shape-form-length = Längd (X)
shape-form-width = Bredd (Y)
shape-form-angle = Vinkel
# Whether the polyline is a closed loop; the checkbox's text follows.
shape-form-closed = Sluten
shape-form-closed-checkbox = tillbaka till första punkten
# Del is the keyboard key.
shape-form-delete = Radera form  (Del)
shape-form-delete-hover = Operationer som använder den flaggas tills deras geometri har valts igen

# How to edit a selected polyline on the canvas. A bulge bends a segment into an arc;
# Alt and Shift are keyboard keys.
shape-form-polyline-help = På ritytan: dra en fyrkant för att flytta en punkt · dra ett runt mitthandtag för att infoga en ny punkt där · Shift+dra ett mitthandtag för att böja segmentet till en båge (bukt) · den streckade rutan flyttar, vrider och skalar den · Alt: ingen fästning

shape-form-corner-radius = Hörnradie
shape-form-corner-radius-hover = Rundar alla fyra hörnen; högst halva den kortare sidan
# A button that sets the corner radius back to 0.
shape-form-reset = Återställ
shape-form-sharp-corners = Skarpa hörn
# The centre point of a circle or rectangle.
shape-form-centre = Centrum

# The column of the points table: how much the segment to the next point is bent.
shape-form-bulge = Bukt
shape-form-bulge-hover = Böjningen av segmentet till nästa punkt: 0 rakt, 1 en halvcirkel (moturs), -1 åt andra hållet
shape-form-insert-point = Infoga en punkt efter denna
shape-form-delete-point = Radera denna punkt

# An imported shape (from a DXF) can't be edited.
shape-form-imported = Form S{ $id } (importerad, skrivskyddad)
# The DXF entity the shape comes from: its type (LINE, ARC, …) and handle.
shape-form-imported-from = Lager { $layer }, DXF { $entity } { $handle }
# Shown when the DXF entity's type is not known.
shape-form-entity = objekt
shape-form-position = Position
shape-form-position-hover = Nedre vänstra hörnet av rutan runt den; ett inskrivet värde flyttar den dit
shape-form-turn = Vrid
shape-form-turn-ccw-hover = Vrid ett kvarts varv moturs kring mitten av rutan
shape-form-turn-cw-hover = Vrid ett kvarts varv medurs kring mitten av rutan
shape-form-turn-by = med{" "}
shape-form-turn-by-hover = Skriv en vinkel och tryck Enter för att vrida den så mycket (moturs; negativt vrider medurs)
shape-form-name = Namn
shape-form-name-hover = Dess namn i bläddraren; tomt: visas med sitt nummer
shape-form-title-slot = Spår S{ $id }
shape-form-title-polygon = Polygon S{ $id }
shape-form-sides = Sidor
shape-form-polygon-radius = Radie
shape-form-polygon-radius-hover = Från centrum till ett hörn
shape-form-across-flats = Nyckelvidd
shape-form-across-flats-hover = Från en sida till den motsatta (två gånger avståndet från centrum till en sidas mitt); ändrar radien
shape-form-polygon-angle = Vinkel
shape-form-polygon-angle-hover = Dit första hörnet pekar, moturs från +X
