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
shape-form-polyline-help = En el lienzo: arrastra un cuadrado para mover un punto (Shift: pasos de 15°) · Ctrl+clic en cuadrados elige varios; arrastra uno para moverlos juntos · arrastra una línea (o su tirador redondo central) para mover ese segmento · Ctrl+arrastrar inserta ahí un punto nuevo · Shift+arrastrar curva el segmento en un arco (curvatura) · el cuadro discontinuo la mueve, la gira y la escala · Alt: sin ajuste

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
shape-form-imported-from = Capa { $layer }, { $entity } { $handle }
# Shown when the DXF entity's type is not known.
shape-form-entity = entidad
shape-form-position = Posición
shape-form-position-hover = La esquina inferior izquierda del cuadro que la rodea; un valor escrito la mueve ahí
shape-form-size = Tamaño
shape-form-size-hover = El ancho y el alto del recuadro que lo rodea; un valor escrito escala alrededor del centro del recuadro
shape-form-keep-proportions = Mantener proporciones
shape-form-keep-proportions-hover = Marcado: un nuevo ancho o alto escala por igual. Desmarcado: estira en un solo sentido; círculos y arcos pasan a ser polilíneas que siguen la curva estirada, y los radios de esquina se mantienen
shape-form-turn = Girar
shape-form-turn-ccw-hover = Un cuarto de vuelta en sentido antihorario alrededor del centro de su cuadro
shape-form-turn-cw-hover = Un cuarto de vuelta en sentido horario alrededor del centro de su cuadro
shape-form-turn-by = {" "}
shape-form-turn-by-hover = Escribe un ángulo y pulsa Intro para girarla eso (sentido antihorario; negativo: horario)
shape-form-name = Nombre
shape-form-name-hover = Su nombre en el Navegador; vacío: se muestra por su número
shape-form-title-slot = Ranura S{ $id }
shape-form-title-polygon = Polígono S{ $id }
shape-form-sides = Lados
shape-form-polygon-radius = Radio
shape-form-polygon-radius-hover = Del centro a un vértice
shape-form-across-flats = Entre caras
shape-form-across-flats-hover = De un lado al opuesto (dos veces la distancia del centro al punto medio de un lado); cambiarlo cambia el radio
shape-form-polygon-angle = Ángulo
shape-form-polygon-angle-hover = Hacia dónde apunta el primer vértice, en sentido antihorario desde +X
shape-form-title-spline = Spline S{ $id }
shape-form-corner-radius-column = Radio
shape-form-corner-radius-column-hover = Un radio en una esquina la redondea con un arco tangente a ambos lados; el punto queda en la esquina viva. 0: viva
shape-form-corner-radius-none = Aquí no hay esquina que redondear: los lados se unen sin esquina, o siguen rectos o vuelven atrás
shape-form-corners = Esquinas
shape-form-each-corner = Cada esquina
shape-form-each-corner-hover = Marcado: un radio por esquina, numeradas en sentido antihorario desde la primera (en un rectángulo: desde la inferior izquierda a 0°). Desmarcado: un radio para todas las esquinas
