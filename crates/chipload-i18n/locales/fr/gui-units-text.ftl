# Texts of the app that name a length or its unit, which follow the display units
# (Settings > Units): $unit is "mm" or "in", a length argument brings its own unit
# ("10 mm", "0.3937 in").

# Where X, Y and Z zero are, after "Work zero:". $at is one of the work zero choices
# ("stock bottom left"), $point where that is in the drawing ("(0.000, 0.000) mm"), $z one
# of the two below.
units-text-work-zero = { $at } = dessin { $point }, Z0 = { $z }
units-text-zero-stock-top = dessus du brut
units-text-zero-stock-bottom = dessous du brut

# Edit > Paste. $shift: how far a paste lands from the copied shapes.
units-text-paste-hover = { $shift } à droite et en haut des formes copiées ; Ctrl+V sur le canevas colle au curseur
# $diameter: the new tool's diameter.
units-text-new-tool-hover = Une fraise de { $diameter } à modifier ; les opérations la choisissent dans leur ligne Outil
units-text-oversize-hover = De combien on usine plus que le dessin, en { $unit } (clearance dans le fichier) : positif agrandit une poche ou réduit une pièce, négatif laisse de la matière pour une coupe ultérieure. Inférieure au rayon de l'outil. D'autres programmes de FAO appellent l'inverse « surépaisseur ».
units-text-picture-position-hover = Le coin inférieur gauche de la boîte de l'image, en { $unit }
units-text-text-position-corner-hover = Le coin inférieur gauche de la boîte du texte, en { $unit }
units-text-text-position-centre-hover = Le centre de la boîte du texte, en { $unit }
units-text-dxf-position-hover = Le coin inférieur gauche de la boîte du dessin, en { $unit }. Les formes dessinées dans Chipload et les autres DXF ne bougent pas.
units-text-pitch-hover = D'une copie à l'autre, en { $unit }. Négatif s'étend vers la gauche ou le bas.
