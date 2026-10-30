# Texts of the app that name a length or its unit, which follow the display units
# (Settings > Units): $unit is "mm" or "in", a length argument brings its own unit
# ("10 mm", "0.3937 in").

# Where X, Y and Z zero are, after "Work zero:". $at is one of the work zero choices
# ("stock bottom left"), $point where that is in the drawing ("(0.000, 0.000) mm"), $z one
# of the two below.
units-text-work-zero = { $at } = { $point } i ritningen, Z0 = { $z }
units-text-zero-stock-top = ämnets ovansida
units-text-zero-stock-bottom = ämnets undersida

# Edit > Paste. $shift: how far a paste lands from the copied shapes.
units-text-paste-hover = { $shift } till höger och uppåt från de kopierade formerna; Ctrl+V över ritytan klistrar in vid markören
# $diameter: the new tool's diameter.
units-text-new-tool-hover = En { $diameter } pinnfräs att redigera; operationer väljer den i sin rad Verktyg
units-text-oversize-hover = Hur mycket mer än ritat som fräses bort, { $unit } (clearance i filen): positivt gör en ficka större eller en detalj mindre, negativt lämnar material kvar för en senare bearbetning. Mindre än verktygsradien. Andra CAM-program kallar motsatsen "bearbetningsmån".
units-text-picture-position-hover = Nedre vänstra hörnet av bildens ram, { $unit }
units-text-text-position-corner-hover = Nedre vänstra hörnet av textens ram, { $unit }
units-text-text-position-centre-hover = Centrum av textens ram, { $unit }
units-text-dxf-position-hover = Nedre vänstra hörnet av ritningens ram, { $unit }. Former ritade i Chipload och andra ritningsfiler flyttas inte.
units-text-pitch-hover = Från kopia till kopia, { $unit }. Negativt växer åt vänster eller nedåt.
