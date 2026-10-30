# The form for a drawn shape's values, and the lines about an imported shape
# (crates/chipload-gui/src/shape_form.rs).

# The form's heading: the kind of shape and its id (S3).
shape-form-title-path = tracé S{ $id }
shape-form-title-circle = Cercle S{ $id }
shape-form-title-rectangle = Rectangle S{ $id }
shape-form-title-polyline = Polyligne S{ $id }

# The drawing layer the shape is on.
shape-form-layer = Calque
shape-form-radius = Rayon
shape-form-length = Longueur (X)
shape-form-width = Largeur (Y)
shape-form-angle = Angle
# Whether the polyline is a closed loop; the checkbox's text follows.
shape-form-closed = Fermée
shape-form-closed-checkbox = retour au premier point
# Del is the keyboard key.
shape-form-delete = Supprimer la forme  (Del)
shape-form-delete-hover = Les opérations qui l'utilisent sont signalées jusqu'à ce que leur géométrie soit choisie à nouveau

# How to edit a selected polyline on the canvas. A bulge bends a segment into an arc;
# Alt and Shift are keyboard keys.
shape-form-polyline-help = Sur le canevas : glisser un carré déplace un point · glisser une ligne (ou sa poignée ronde au milieu) déplace ce segment · Ctrl+glisser y insère un point · Shift+glisser courbe le segment en arc (bombé) · le cadre en pointillés la déplace, la tourne et la met à l'échelle · Alt : sans accrochage

shape-form-corner-radius = Rayon des coins
shape-form-corner-radius-hover = Arrondit les quatre coins ; au plus la moitié du petit côté
# A button that sets the corner radius back to 0.
shape-form-reset = Réinitialiser
shape-form-sharp-corners = Coins vifs
# The centre point of a circle or rectangle.
shape-form-centre = Centre

# The column of the points table: how much the segment to the next point is bent.
shape-form-bulge = Bombé
shape-form-bulge-hover = Courbure du segment vers le point suivant : 0 droit, 1 un demi-cercle (sens anti-horaire), -1 dans l'autre sens
shape-form-insert-point = Insérer un point après celui-ci
shape-form-delete-point = Supprimer ce point

# An imported shape (from a DXF) can't be edited.
shape-form-imported = Forme S{ $id } (importée, lecture seule)
# The DXF entity the shape comes from: its type (LINE, ARC, …) and handle.
shape-form-imported-from = Calque { $layer }, { $entity } { $handle }
# Shown when the DXF entity's type is not known.
shape-form-entity = entité
shape-form-position = Position
shape-form-position-hover = Le coin inférieur gauche du cadre qui l'entoure ; une valeur saisie l'y déplace
shape-form-turn = Tourner
shape-form-turn-ccw-hover = Un quart de tour dans le sens anti-horaire autour du milieu de son cadre
shape-form-turn-cw-hover = Un quart de tour dans le sens horaire autour du milieu de son cadre
shape-form-turn-by = de{" "}
shape-form-turn-by-hover = Saisissez un angle et appuyez sur Entrée pour la tourner d'autant (sens anti-horaire ; négatif : sens horaire)
shape-form-name = Nom
shape-form-name-hover = Son nom dans le Navigateur ; vide : affichée par son numéro
shape-form-title-slot = Rainure S{ $id }
shape-form-title-polygon = Polygone S{ $id }
shape-form-sides = Côtés
shape-form-polygon-radius = Rayon
shape-form-polygon-radius-hover = Du centre à un sommet
shape-form-across-flats = Sur plats
shape-form-across-flats-hover = D'un côté au côté opposé (deux fois la distance du centre au milieu d'un côté) ; le changer change le rayon
shape-form-polygon-angle = Angle
shape-form-polygon-angle-hover = Où pointe le premier sommet, dans le sens antihoraire depuis +X
