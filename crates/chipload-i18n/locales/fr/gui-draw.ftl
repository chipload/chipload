# Drawing shapes with the mouse: the status bar while drawing, and snapping
# (crates/chipload-gui/src/draw.rs). Esc, Enter and Backspace are keyboard keys; the
# grid is the step clicks are rounded to, e.g. "1 mm".

draw-status-circle-start = Cercle : cliquez le centre (grille { $grid }, Esc annule)
draw-status-circle = Cercle de rayon { $radius } : cliquez un point du cercle (grille { $grid }, Esc annule)
draw-status-rectangle-start = Rectangle : cliquez un coin (grille { $grid }, Esc annule)
# The rectangle's size so far: along X, then along Y.
draw-status-rectangle = Rectangle { $length } x { $width } : cliquez le coin opposé (grille { $grid }, Esc annule)
draw-status-polyline-start = Polyligne { $count ->
        [one] { $count } point
       *[other] { $count } points
    } : cliquez le premier point (grille { $grid }, Esc annule)
draw-status-polyline = Polyligne { $count ->
        [one] { $count } point
       *[other] { $count } points
    } : cliquez le point suivant ; Enter ou double-clic la termine ouverte, le premier point la ferme, Backspace retire le dernier point (grille { $grid }, Esc annule)

# Why a click did nothing.
draw-no-size-circle = le cercle n'aurait aucune taille
draw-no-size-rectangle = le rectangle n'aurait aucune taille
draw-polyline-too-short = une polyligne demande au moins 2 points

# The point the cursor snapped to, on contour #4.
draw-snap-end = extrémité de #{ $contour }
draw-snap-centre = centre de #{ $contour }
draw-snap-middle = milieu de #{ $contour }
draw-snap-intersection = intersection de #{ $contour } et #{ $other }
draw-snap-intersection-self = intersection sur #{ $contour }
draw-snap-quadrant = quadrant de #{ $contour }
draw-snap-perpendicular = perpendiculaire à #{ $contour }
draw-snap-tangent = tangente à #{ $contour }
draw-snap-nearest = sur #{ $contour }

# A value typed while drawing that can't be used.
draw-entry-unreadable = illisible : tapez x,y  @dx,dy  une longueur, ou longueur<angle
draw-entry-needs-point = le premier point : tapez x,y
draw-entry-no-direction = éloignez le curseur du dernier point pour donner la direction
draw-status-arc-start = Dessin d'un arc : cliquez où il commence (grille { $grid }, Échap annule)
draw-status-arc-end = Dessin d'un arc, corde { $length } : cliquez où il finit (grille { $grid }, Échap annule)
draw-status-arc = Dessin d'un arc de rayon { $radius } : cliquez un point par où il passe, ou tapez son rayon (grille { $grid }, Échap annule)
draw-status-slot-start = Dessin d'une rainure : cliquez le centre d'une extrémité (grille { $grid }, Échap annule)
draw-status-slot-end = Dessin d'une rainure, { $length } entre les extrémités : cliquez le centre de l'autre extrémité (grille { $grid }, Échap annule)
draw-status-slot = Dessin d'une rainure de largeur { $width } : cliquez pour donner sa largeur, ou tapez-la (grille { $grid }, Échap annule)
draw-arc-on-a-line = les trois points sont alignés : utilisez l'outil polyligne
draw-no-size-slot = la rainure n'aurait ni longueur ni largeur
draw-entry-radius-too-small = le rayon doit valoir au moins la moitié de la distance entre les extrémités
draw-entry-no-side = placez le curseur du côté où l'arc doit se bomber
draw-status-polygon-start = Dessin d'un polygone : cliquez le centre (grille { $grid }, Échap annule)
draw-status-polygon = Dessin d'un polygone de rayon { $radius } : cliquez un sommet (grille { $grid }, Échap annule)
modify-offset-pick = Décaler : sélectionnez les formes (cliquez-en une ; Ctrl ajoute), Échap termine
modify-offset-status = Décalage { $distance } { $side } : cliquez, ou tapez la distance (Alt : hors grille, Échap termine)
modify-offset-done = { $count ->
    [one] Un décalage ajouté
   *[other] { $count } décalages ajoutés
} à { $distance }.
modify-side-outside = à l'extérieur
modify-side-inside = à l'intérieur
modify-side-left = à gauche
modify-side-right = à droite
modify-corners-round = Coins arrondis
modify-corners-sharp = Coins vifs
modify-corners-hover = Arrondis : le décalage exact, des arcs autour des coins. Vifs : les côtés prolongés jusqu'à se rejoindre, comme un cadre.
modify-offset-count = Nombre
modify-corner-pick = Sélectionnez une forme dessinée (cliquez-la), puis cliquez ses coins ; Échap termine
modify-fillet-status = Congé : cliquez un coin ; Maj+clic arrondit tous les coins (Échap termine)
modify-chamfer-status = Chanfrein : cliquez un coin ; Maj+clic coupe tous les coins (Échap termine)
modify-fillet-radius = Rayon
modify-chamfer-distance = Distance
modify-trim-status = Ajuster : cliquez le morceau d'une forme dessinée à enlever ; il s'affiche avant le clic (Échap termine)
modify-extend-status = Prolonger : cliquez près de l'extrémité d'une forme ouverte pour la prolonger jusqu'à la forme suivante (Échap termine)
modify-break-status = Couper : cliquez une forme dessinée là où elle doit être coupée (Échap termine)
modify-extend-closed = une forme fermée n'a pas d'extrémité à prolonger
modify-combine-pick = Sélectionnez d'abord au moins deux formes dessinées fermées
modify-subtract-status = Soustraire : cliquez la forme dans laquelle découper celles sélectionnées (Échap termine)
modify-construction-on = Les prochaines formes dessinées sont de la géométrie de construction (K l'arrête).
modify-construction-off = Les prochaines formes dessinées sont des formes ordinaires.
modify-array-pick = Sélectionnez d'abord les formes dessinées à copier
modify-array-status = Réseau : réglez les copies sous Sélection, puis Créer (Échap termine)
modify-polar-status = Réseau circulaire : cliquez le centre, réglez les copies sous Sélection, puis Créer (Échap termine)
array-columns = Colonnes
array-rows = Rangées
array-dx = Espacement des colonnes
array-dy = Espacement des rangées
array-count = Nombre
array-center = Centre
array-angle = Angle
array-angle-hover = 360° répartit les copies régulièrement tout autour ; moins place la première et la dernière à ses extrémités ; négatif tourne dans le sens horaire
array-rotate = Tourner les copies
array-create = Créer
arrange-align = Aligner
arrange-left = Côtés gauches alignés
arrange-centre = Centres alignés, horizontalement
arrange-right = Côtés droits alignés
arrange-top = Hauts alignés
arrange-middle = Milieux alignés, verticalement
arrange-bottom = Bas alignés
arrange-stock = Sur le brut
arrange-stock-hover = Aligner les formes sur les côtés ou le milieu du brut plutôt que sur le cadre qui les entoure
arrange-distribute = Répartir
arrange-across = Écarts égaux horizontalement, la plus à gauche et la plus à droite restent en place
arrange-up = Écarts égaux verticalement, la plus basse et la plus haute restent en place
