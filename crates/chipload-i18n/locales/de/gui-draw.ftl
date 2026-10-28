# Drawing shapes with the mouse: the status bar while drawing, and snapping
# (crates/chipload-gui/src/draw.rs). Esc, Enter and Backspace are keyboard keys; the
# grid is the step clicks are rounded to, e.g. "1 mm".

draw-status-circle-start = Kreis zeichnen: Mittelpunkt anklicken (Raster { $grid }, Esc bricht ab)
draw-status-circle = Kreis zeichnen, Radius { $radius }: einen Punkt auf dem Kreis anklicken (Raster { $grid }, Esc bricht ab)
draw-status-rectangle-start = Rechteck zeichnen: eine Ecke anklicken (Raster { $grid }, Esc bricht ab)
# The rectangle's size so far: along X, then along Y.
draw-status-rectangle = Rechteck zeichnen, { $length } x { $width }: die gegenüberliegende Ecke anklicken (Raster { $grid }, Esc bricht ab)
draw-status-polyline-start = Polylinie zeichnen, { $count ->
        [one] { $count } Punkt
       *[other] { $count } Punkte
    }: den ersten Punkt anklicken (Raster { $grid }, Esc bricht ab)
draw-status-polyline = Polylinie zeichnen, { $count ->
        [one] { $count } Punkt
       *[other] { $count } Punkte
    }: den nächsten Punkt anklicken; Enter oder Doppelklick beendet sie offen, der erste Punkt schließt sie, Backspace entfernt den letzten Punkt (Raster { $grid }, Esc bricht ab)

# Why a click did nothing.
draw-no-size-circle = der Kreis hätte keine Größe
draw-no-size-rectangle = das Rechteck hätte keine Größe
draw-polyline-too-short = eine Polylinie braucht mindestens 2 Punkte

# The point the cursor snapped to, on contour #4.
draw-snap-end = Ende von #{ $contour }
draw-snap-centre = Mittelpunkt von #{ $contour }
draw-snap-middle = Mitte von #{ $contour }
draw-snap-intersection = Schnittpunkt von #{ $contour } und #{ $other }
draw-snap-intersection-self = Schnittpunkt auf #{ $contour }
draw-snap-quadrant = Quadrantenpunkt von #{ $contour }
draw-snap-perpendicular = Lot auf #{ $contour }
draw-snap-tangent = Tangente an #{ $contour }
draw-snap-nearest = auf #{ $contour }

# A value typed while drawing that can't be used.
draw-entry-unreadable = nicht lesbar: x,y  @dx,dy  eine Länge oder Länge<Winkel eingeben
draw-entry-needs-point = der erste Punkt: x,y eingeben
draw-entry-no-direction = den Cursor vom letzten Punkt wegbewegen, um die Richtung anzugeben
draw-status-arc-start = Bogen zeichnen: klicken, wo er beginnt (Raster { $grid }, Esc bricht ab)
draw-status-arc-end = Bogen zeichnen, Sehne { $length }: klicken, wo er endet (Raster { $grid }, Esc bricht ab)
draw-status-arc = Bogen zeichnen, Radius { $radius }: einen Punkt klicken, durch den er geht, oder den Radius eingeben (Raster { $grid }, Esc bricht ab)
draw-status-slot-start = Langloch zeichnen: den Mittelpunkt eines Endes klicken (Raster { $grid }, Esc bricht ab)
draw-status-slot-end = Langloch zeichnen, { $length } zwischen den Enden: den Mittelpunkt des anderen Endes klicken (Raster { $grid }, Esc bricht ab)
draw-status-slot = Langloch zeichnen, Breite { $width }: klicken, um die Breite anzugeben, oder sie eingeben (Raster { $grid }, Esc bricht ab)
draw-arc-on-a-line = die drei Punkte liegen auf einer Linie: das Polylinien-Werkzeug verwenden
draw-no-size-slot = das Langloch hätte keine Länge oder keine Breite
draw-entry-radius-too-small = der Radius muss mindestens halb so groß sein wie der Abstand der Enden
draw-entry-no-side = den Cursor auf die Seite bewegen, zu der sich der Bogen wölben soll
draw-status-polygon-start = Vieleck zeichnen: den Mittelpunkt klicken (Raster { $grid }, Esc bricht ab)
draw-status-polygon = Vieleck zeichnen, Radius { $radius }: eine Ecke klicken (Raster { $grid }, Esc bricht ab)
modify-offset-pick = Versatz: die Formen auswählen (eine anklicken; Strg fügt hinzu), Esc beendet
modify-offset-status = Versatz { $distance } { $side }: klicken oder den Abstand eingeben (Alt: ohne Raster, Esc beendet)
modify-offset-done = { $count ->
    [one] Ein Versatz
   *[other] { $count } Versätze
} im Abstand { $distance } hinzugefügt.
modify-side-outside = außen
modify-side-inside = innen
modify-side-left = nach links
modify-side-right = nach rechts
modify-corners-round = Runde Ecken
modify-corners-sharp = Spitze Ecken
modify-corners-hover = Rund: der genaue Versatz, Bögen um die Ecken. Spitz: die Seiten verlängert, bis sie sich treffen, wie ein Rahmen.
modify-offset-count = Anzahl
modify-corner-pick = Eine gezeichnete Form auswählen (anklicken), dann ihre Ecken klicken; Esc beendet
modify-fillet-status = Abrunden: eine Ecke klicken; Shift+Klick rundet alle Ecken (Esc beendet)
modify-chamfer-status = Fase: eine Ecke klicken; Shift+Klick schneidet alle Ecken (Esc beendet)
modify-fillet-radius = Radius
modify-chamfer-distance = Abstand
modify-trim-status = Stutzen: das Stück einer gezeichneten Form anklicken, das wegfällt; es wird vor dem Klick gezeigt (Esc beendet)
modify-extend-status = Verlängern: nahe dem Ende einer offenen gezeichneten Form klicken, um sie bis zur nächsten Form zu verlängern (Esc beendet)
modify-break-status = Teilen: eine gezeichnete Form dort klicken, wo sie geteilt werden soll (Esc beendet)
modify-extend-closed = eine geschlossene Form hat kein Ende zum Verlängern
