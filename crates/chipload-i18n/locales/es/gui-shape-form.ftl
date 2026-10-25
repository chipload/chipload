# The form for a drawn shape's values, and the lines about an imported shape
# (crates/chipload-gui/src/shape_form.rs).

# The form's heading: the kind of shape and its id (S3).
shape-form-title-path = trayecto S{ $id }
shape-form-title-circle = Círculo S{ $id }
shape-form-title-rectangle = Rectángulo S{ $id }
shape-form-title-polyline = Polilínea S{ $id }

# The drawing layer the shape is on.
shape-form-layer = Capa
shape-form-radius = Radio
shape-form-length = Largo (X)
shape-form-width = Ancho (Y)
shape-form-angle = Ángulo
# Whether the polyline is a closed loop; the checkbox's text follows.
shape-form-closed = Cerrada
shape-form-closed-checkbox = vuelve al primer punto
# Del is the keyboard key.
shape-form-delete = Eliminar forma  (Del)
shape-form-delete-hover = Las operaciones que la usan quedan marcadas hasta que se vuelva a elegir su geometría

# How to edit a selected polyline on the canvas. A bulge bends a segment into an arc;
# Alt and Shift are keyboard keys.
shape-form-polyline-help = En el lienzo: arrastra un cuadrado para mover un punto · arrastra un tirador redondo central para insertar ahí un punto nuevo · Shift+arrastrar un tirador central curva el segmento en un arco (curvatura) · el cuadro discontinuo la mueve, la gira y la escala · Alt: sin ajuste

shape-form-corner-radius = Radio de esquina
shape-form-corner-radius-hover = Redondea las cuatro esquinas; como máximo la mitad del lado más corto
# A button that sets the corner radius back to 0.
shape-form-reset = Restablecer
shape-form-sharp-corners = Esquinas vivas
# The centre point of a circle or rectangle.
shape-form-centre = Centro

# The column of the points table: how much the segment to the next point is bent.
shape-form-bulge = Curvatura
shape-form-bulge-hover = Curvatura del segmento hasta el siguiente punto: 0 recto, 1 medio círculo (sentido antihorario), -1 al revés
shape-form-insert-point = Insertar un punto después de este
shape-form-delete-point = Eliminar este punto

# An imported shape (from a DXF) can't be edited.
shape-form-imported = Forma S{ $id } (importada, solo lectura)
# The DXF entity the shape comes from: its type (LINE, ARC, …) and handle.
shape-form-imported-from = Capa { $layer }, DXF { $entity } { $handle }
# Shown when the DXF entity's type is not known.
shape-form-entity = entidad
