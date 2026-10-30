# Texts of the app that name a length or its unit, which follow the display units
# (Settings > Units): $unit is "mm" or "in", a length argument brings its own unit
# ("10 mm", "0.3937 in").

# Where X, Y and Z zero are, after "Work zero:". $at is one of the work zero choices
# ("stock bottom left"), $point where that is in the drawing ("(0.000, 0.000) mm"), $z one
# of the two below.
units-text-work-zero = { $at } = Zeichnung { $point }, Z0 = { $z }
units-text-zero-stock-top = Rohteiloberkante
units-text-zero-stock-bottom = Rohteilunterkante

# Edit > Paste. $shift: how far a paste lands from the copied shapes.
units-text-paste-hover = { $shift } rechts und oberhalb der kopierten Formen; Ctrl+V über der Zeichenfläche fügt am Cursor ein
# $diameter: the new tool's diameter.
units-text-new-tool-hover = Ein { $diameter } Schaftfräser zum Bearbeiten; Bearbeitungen wählen ihn in ihrer Zeile Werkzeug
units-text-oversize-hover = Wie viel mehr als gezeichnet gefräst wird, { $unit } (clearance in der Datei): positiv macht eine Tasche größer oder ein Teil kleiner, negativ lässt Material für einen späteren Schnitt stehen. Kleiner als der Werkzeugradius. Andere CAM-Programme nennen das Gegenteil „Aufmaß“.
units-text-picture-position-hover = Die untere linke Ecke des Bildrahmens, { $unit }
units-text-text-position-corner-hover = Die untere linke Ecke des Textrahmens, { $unit }
units-text-text-position-centre-hover = Die Mitte des Textrahmens, { $unit }
units-text-dxf-position-hover = Die untere linke Ecke des Zeichnungsrahmens, { $unit }. In Chipload gezeichnete Formen und andere Zeichnungsdateien bewegen sich nicht.
units-text-pitch-hover = Von Kopie zu Kopie, { $unit }. Negativ wächst nach links oder unten.
