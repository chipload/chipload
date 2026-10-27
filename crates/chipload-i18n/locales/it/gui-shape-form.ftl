# The form for a drawn shape's values, and the lines about an imported shape
# (crates/chipload-gui/src/shape_form.rs).

# The form's heading: the kind of shape and its id (S3).
shape-form-title-path = percorso S{ $id }
shape-form-title-circle = Cerchio S{ $id }
shape-form-title-rectangle = Rettangolo S{ $id }
shape-form-title-polyline = Polilinea S{ $id }

# The drawing layer the shape is on.
shape-form-layer = Layer
shape-form-radius = Raggio
shape-form-length = Lunghezza (X)
shape-form-width = Larghezza (Y)
shape-form-angle = Angolo
# Whether the polyline is a closed loop; the checkbox's text follows.
shape-form-closed = Chiusa
shape-form-closed-checkbox = torna al primo punto
# Del is the keyboard key.
shape-form-delete = Elimina forma  (Del)
shape-form-delete-hover = Le lavorazioni che la usano vengono segnalate finché la loro geometria non viene scelta di nuovo

# How to edit a selected polyline on the canvas. A bulge bends a segment into an arc;
# Alt and Shift are keyboard keys.
shape-form-polyline-help = Nell'area di disegno: trascina un quadrato per spostare un punto · trascina una maniglia tonda centrale per inserirvi un nuovo punto · Shift+trascina una maniglia centrale per curvare il segmento in un arco (bulge) · il riquadro tratteggiato la sposta, la ruota e la scala · Alt: niente aggancio

shape-form-corner-radius = Raggio angoli
shape-form-corner-radius-hover = Arrotonda tutti e quattro gli angoli; al massimo metà del lato più corto
# A button that sets the corner radius back to 0.
shape-form-reset = Ripristina
shape-form-sharp-corners = Angoli vivi
# The centre point of a circle or rectangle.
shape-form-centre = Centro

# The column of the points table: how much the segment to the next point is bent.
shape-form-bulge = Bulge
shape-form-bulge-hover = Curvatura del segmento verso il punto successivo: 0 dritto, 1 mezzo cerchio (antiorario), -1 nell'altro verso
shape-form-insert-point = Inserisci un punto dopo questo
shape-form-delete-point = Elimina questo punto

# An imported shape (from a DXF) can't be edited.
shape-form-imported = Forma S{ $id } (importata, sola lettura)
# The DXF entity the shape comes from: its type (LINE, ARC, …) and handle.
shape-form-imported-from = Layer { $layer }, DXF { $entity } { $handle }
# Shown when the DXF entity's type is not known.
shape-form-entity = entità
shape-form-position = Posizione
shape-form-position-hover = L'angolo in basso a sinistra del riquadro che la circonda; un valore digitato la sposta lì
shape-form-turn = Ruota
shape-form-turn-ccw-hover = Ruota di un quarto in senso antiorario attorno al centro del suo riquadro
shape-form-turn-cw-hover = Ruota di un quarto in senso orario attorno al centro del suo riquadro
shape-form-turn-by = di{" "}
shape-form-turn-by-hover = Digita un angolo e premi Invio per ruotarla di tanto (senso antiorario; negativo: orario)
shape-form-name = Nome
shape-form-name-hover = Il suo nome nel Browser; vuoto: mostrata col suo numero
shape-form-title-slot = Asola S{ $id }
