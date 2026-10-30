# Texts of the app that name a length or its unit, which follow the display units
# (Settings > Units): $unit is "mm" or "in", a length argument brings its own unit
# ("10 mm", "0.3937 in").

# Where X, Y and Z zero are, after "Work zero:". $at is one of the work zero choices
# ("stock bottom left"), $point where that is in the drawing ("(0.000, 0.000) mm"), $z one
# of the two below.
units-text-work-zero = { $at } = disegno { $point }, Z0 = { $z }
units-text-zero-stock-top = superficie del grezzo
units-text-zero-stock-bottom = fondo del grezzo

# Edit > Paste. $shift: how far a paste lands from the copied shapes.
units-text-paste-hover = { $shift } a destra e in alto rispetto alle forme copiate; Ctrl+V sull'area di disegno incolla nel punto del cursore
# $diameter: the new tool's diameter.
units-text-new-tool-hover = Una fresa da { $diameter } da modificare; le lavorazioni la scelgono nella loro riga Utensile
units-text-oversize-hover = Quanto si taglia in più rispetto al disegno, { $unit } (clearance nel file): un valore positivo allarga una tasca o rimpicciolisce un pezzo, uno negativo lascia materiale per un taglio successivo. Minore del raggio dell'utensile. Altri programmi CAM chiamano l'opposto "sovrametallo".
units-text-picture-position-hover = L'angolo in basso a sinistra del riquadro dell'immagine, { $unit }
units-text-text-position-corner-hover = L'angolo in basso a sinistra del riquadro del testo, { $unit }
units-text-text-position-centre-hover = Il centro del riquadro del testo, { $unit }
units-text-dxf-position-hover = L'angolo in basso a sinistra del riquadro del disegno, { $unit }. Le forme disegnate in Chipload e gli altri file di disegno non si spostano.
units-text-pitch-hover = Da copia a copia, { $unit }. Un valore negativo cresce verso sinistra o verso il basso.
