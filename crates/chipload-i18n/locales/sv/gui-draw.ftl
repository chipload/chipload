# Drawing shapes with the mouse: the status bar while drawing, and snapping
# (crates/chipload-gui/src/draw.rs). Esc, Enter and Backspace are keyboard keys; the
# grid is the step clicks are rounded to, e.g. "1 mm".

draw-status-circle-start = Ritar en cirkel: klicka på centrum (rutnät { $grid }, Esc avbryter)
draw-status-circle = Ritar en cirkel med radie { $radius }: klicka på en punkt på cirkeln (rutnät { $grid }, Esc avbryter)
draw-status-rectangle-start = Ritar en rektangel: klicka på ett hörn (rutnät { $grid }, Esc avbryter)
# The rectangle's size so far: along X, then along Y.
draw-status-rectangle = Ritar en rektangel { $length } x { $width }: klicka på motstående hörn (rutnät { $grid }, Esc avbryter)
draw-status-polyline-start = Ritar en polylinje { $count ->
        [one] { $count } punkt
       *[other] { $count } punkter
    }: klicka på första punkten (rutnät { $grid }, Esc avbryter)
draw-status-polyline = Ritar en polylinje { $count ->
        [one] { $count } punkt
       *[other] { $count } punkter
    }: klicka på nästa punkt; Enter eller dubbelklick avslutar den öppen, första punkten sluter den, Backspace tar bort senaste punkten (rutnät { $grid }, Esc avbryter)

# Why a click did nothing.
draw-no-size-circle = cirkeln skulle sakna storlek
draw-no-size-rectangle = rektangeln skulle sakna storlek
draw-polyline-too-short = en polylinje behöver minst 2 punkter

# The point the cursor snapped to, on contour #4.
draw-snap-end = ände av #{ $contour }
draw-snap-centre = centrum av #{ $contour }
draw-snap-middle = mitten av #{ $contour }
draw-snap-intersection = skärning mellan #{ $contour } och #{ $other }
draw-snap-intersection-self = skärning på #{ $contour }
draw-snap-quadrant = kvadrantpunkt på #{ $contour }
draw-snap-perpendicular = vinkelrätt mot #{ $contour }
draw-snap-tangent = tangent till #{ $contour }
draw-snap-nearest = på #{ $contour }

# A value typed while drawing that can't be used.
draw-entry-unreadable = kan inte läsas: skriv x,y  @dx,dy  en längd, eller längd<vinkel
draw-entry-needs-point = första punkten: skriv x,y
draw-entry-no-direction = flytta markören från den senaste punkten för att ange riktningen
