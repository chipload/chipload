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
