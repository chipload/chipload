# Messages of the planner (crates/chipload-toolpath): rules a project must meet before it
# can be cut. Most are shown after the operation's label, as "Operation 3 (Outline): …";
# see job-operation-problem. Words in the texts that name fields or values of the project
# file (safe_z, z_zero, max_feed, entry: ramp, center_cutting, …) are written in the file
# that way and are never translated. A length or feed argument brings its own unit
# ("12.5 mm", "1200 mm/min").

## The project as a whole

toolpath-no-operations = Der Job hat keine Bearbeitungen
toolpath-all-disabled = Alle Bearbeitungen sind deaktiviert: nichts zu fräsen
# $tool is a tool's number.
toolpath-tool-undefined = Werkzeug { $tool } ist im Job nicht definiert
# Under the tool's spindle speed field.
toolpath-no-rpm-short = keine Drehzahl
toolpath-no-rpm = Werkzeug { $tool } hat keine Drehzahl (rpm)
# Under the machine's safe height; $top is the height of the stock's top.
toolpath-safe-z-short = muss über der Rohteiloberkante liegen (Z { $top })
# $zero is "top" or "bottom", as written in the project file.
toolpath-safe-z = safe_z { $safe } muss über der Rohteiloberkante liegen (Z { $top }); z_zero ist '{ $zero }'

## Depths and sizes

# $below is how far below the stock's bottom it goes, $allowed the setting "max depth
# below stock".
toolpath-too-deep = Endtiefe { $end } liegt { $below } unter dem Rohteil ({ $thickness } dick), mehr als die zulässigen { $allowed } (Max. Tiefe unter Rohteil)
toolpath-clearance-radius = Übermaß { $clearance } muss kleiner als der Werkzeugradius { $radius } sein
toolpath-finish-radius = Aufmaß { $stock } für den Schlichtgang muss kleiner als der Werkzeugradius { $radius } sein
toolpath-tab-height = Höhe der Haltestege { $height } muss kleiner als die Schnitttiefe { $cut } sein (Endtiefe minus Starttiefe)
# $field is "feed" or "plunge_feed", as written in the project file.
toolpath-max-feed = { $field } { $feed } überschreitet max_feed { $max } der Maschine
toolpath-helix-for-path = Helix-Eintauchen ist für Taschen; Pfad folgen kann per Rampe eintauchen (entry: ramp)
toolpath-helix-radius = Helixradius { $radius } muss kleiner als der Werkzeugradius { $tool_radius } sein, sonst bleibt in der Mitte ein Kern stehen
# $most is a number of tool diameters, $longest that as a length.
toolpath-lead-radius = Anfahrradius { $radius } ist mehr als { $most } × Werkzeugdurchmesser ({ $longest })

## Tools

# $tool is a tool's number, $type a kind of tool (job-tool-type).
toolpath-surface-endmill = Werkzeug { $tool } ist ein { $type }; Planfräsen braucht einen Schaftfräser
toolpath-vcarve-vbit = V-Gravur braucht einen V-Fräser, Werkzeug { $tool } ist ein { $type }
toolpath-vbit-no-angle = Werkzeug { $tool } ist ein V-Fräser ohne Winkel; geben Sie seinen Winkel an
toolpath-pocket-endmill = Taschen brauchen einen Schaftfräser, Werkzeug { $tool } ist ein { $type }
toolpath-path-drill = Werkzeug { $tool } ist ein Bohrer und kann keinem Pfad folgen; verwenden Sie einen Schaftfräser oder V-Fräser
toolpath-holes-vbit = Werkzeug { $tool } ist ein V-Fräser und kann keine Bohrungen fräsen; verwenden Sie einen Bohrer oder Schaftfräser
# $method is the holes' method as written in the project file, "drill" or "bore"; $type
# a kind of tool with its article (job-tool-type-article).
toolpath-holes-method = die Methode ist { $method ->
    [drill] drill, die einen Bohrer braucht
   *[bore] bore, die einen Schaftfräser braucht
}; Werkzeug { $tool } ist { $type }
# Angles in degrees.
toolpath-flank-not-centre-cutting = Werkzeug { $tool } schneidet nicht über Mitte, und die Flanken der Haltestege ({ $flank }°) sind steiler als der Rampenwinkel ({ $ramp }°); eine Flanke hinunter ist eine Rampe, also machen Sie sie nicht steiler als { $ramp }°
toolpath-plunge-not-centre-cutting = Werkzeug { $tool } schneidet nicht über Mitte, und diese Bearbeitung taucht senkrecht ein; tauchen Sie stattdessen per Rampe ein (entry: ramp) oder verwenden Sie ein über Mitte schneidendes Werkzeug

## Rest machining

# $tool is the number of the larger tool the pocket rests after.
toolpath-rest-no-tool = Rest nach Werkzeug { $tool }, das es im Projekt nicht gibt
toolpath-rest-not-endmill = Rest nach Werkzeug { $tool }, ein { $type }; das Werkzeug einer Tasche ist ein Schaftfräser
# Two tool diameters.
toolpath-rest-not-larger = Rest nach Werkzeug { $tool } ({ $large }) braucht ein größeres Werkzeug als das dieser Tasche ({ $small })
toolpath-rest-dogbones = Dogbones und Restbearbeitung lassen sich noch nicht kombinieren; versehen Sie die Tasche des großen Werkzeugs mit Dogbones
toolpath-rest-passes-dogbones = Dogbones und Restdurchgänge lassen sich noch nicht kombinieren; Dogbone-Ecken brauchen keinen Rest
toolpath-rest-pass-no-tool = Werkzeug { $tool } ist im Job nicht definiert
toolpath-rest-pass-endmill = Werkzeug { $tool } ist ein { $type }; ein Restdurchgang fräst mit einem Schaftfräser
toolpath-rest-pass-larger = Werkzeug { $tool } ({ $diameter }) muss kleiner sein als Werkzeug { $before } ({ $before_diameter }) davor
toolpath-rest-pass-no-rpm = Werkzeug { $tool } hat keine Drehzahl (rpm)

## Planning notes: warnings that don't stop the G-code

# A note about an operation after its label ("Operation 3 (Outline): …"); $note is one
# of the notes below.
toolpath-labelled = { $label }: { $note }
# A note about one contour of an operation: "Operation 3 (Outline), contour #4: …".
toolpath-labelled-contour = { $label }, Kontur #{ $contour }: { $note }
# Lists built one item at a time: "1, 2, 3" and "1, 2 and 3". $list is the items so far.
toolpath-comma = { $list }, { $item }
toolpath-and = { $list } und { $last }
# An operation's number in a list of operations.
toolpath-number = { $number }
# An operation's number and its kind in a list: "3 (pocket)".
toolpath-operation-kind = { $number } ({ $kind ->
    [profile] Konturfräsen
    [pocket] Tasche
    [vcarve] V-Gravur
    [path] Pfad folgen
    [holes] Bohrungen
   *[surface] Planfräsen
})
# $layers is the contour's layer names, $from and $to its two ends ("(10.000, 5.000) mm").
toolpath-skipped-open = offene Kontur #{ $contour } auf { $layers } von { $from } bis { $to } übersprungen
toolpath-into-spoilboard = fräst { $below } unter das Rohteil (in die Opferplatte)
# $operations lists operation numbers with their kind: "1 (pocket), 3 (profile) and 4 (holes)".
toolpath-plunge-unknown = Werkzeug { $tool } ist nicht als center_cutting markiert, und { $count ->
    [one] Bearbeitung { $operations } taucht
   *[other] die Bearbeitungen { $operations } tauchen
} senkrecht ein: stellen Sie sicher, dass es ein über Mitte schneidendes Werkzeug ist, oder tauchen Sie per Rampe ein (entry: ramp)
# $operations is a list of operation numbers ("1, 2 and 4"); $label the surface's label.
toolpath-surface-after = { $label } plant das Rohteil, nachdem { $count ->
    [one] Bearbeitung { $operations } es bereits gefräst hat
   *[other] die Bearbeitungen { $operations } es bereits gefräst haben
}: Planfräsen kommt normalerweise zuerst
# $tool is the number of the larger tool the pocket rests after.
toolpath-rest-no-earlier = Rest nach Werkzeug { $tool }, aber keine frühere Tasche fräst diese Formen damit so tief; der Rest der Tasche bleibt ungefräst
toolpath-rest-pass-plunge = taucht mit Werkzeug { $tool } senkrecht ein, das nicht als über Mitte schneidend markiert ist; verwenden Sie eine Helix oder eine Rampe, falls es nicht über Mitte schneidet
# $groups is how many separate runs of operations use the tool; $operations their numbers
# ("1, 2, 5").
toolpath-tool-returns = Werkzeug { $tool } wird in { $groups } Gruppen verwendet (Bearbeitungen { $operations }); sie direkt hintereinander zu legen spart einen Werkzeugwechsel
toolpath-vbit-no-tip = Werkzeug { $tool } hat keinen Spitzendurchmesser; die V-Gravur nimmt es als spitz an
# Angles in degrees.
toolpath-flank-unknown-centre-cutting = die Flanken der Haltestege ({ $flank }°) sind steiler als der Rampenwinkel ({ $ramp }°); Werkzeug { $tool } ist nicht als center_cutting markiert, stellen Sie sicher, dass es ein über Mitte schneidendes Werkzeug ist

## Planning notes: profiles

# $loops is how many separate loops the contour is cut as.
toolpath-narrow-loops = das Werkzeug passt nicht durch eine enge Stelle; als { $loops } getrennte Schleifen gefräst, dazwischen bleibt Material stehen
# $point is a start point the user set ("(10.000, 5.000) mm").
toolpath-start-far-contour = Startpunkt { $point } ist { $distance } von Kontur #{ $contour } entfernt; der nächstgelegene Punkt wird verwendet
toolpath-start-twice-contour = Startpunkt { $point } liegt auch auf Kontur #{ $contour }, die schon einen hat; er wird ignoriert
toolpath-tabs-inside = Haltestege an einer Innenkontur halten das ausgeschnittene Abfallstück fest
# $contours is a list of contour numbers ("#3, #4").
toolpath-tabs-on-curves = einige Haltestege auf den Konturen { $contours } sitzen auf einer Ecke oder Kurve (keine gerade Strecke in der Nähe)
# $pairs lists the tabs that meet, by number ("1 and 2, 3 and 4"); $flank is in degrees.
toolpath-tabs-joined = die Haltestege { $pairs } treffen sich unterhalb ihrer Oberkante (Flanken mit { $flank }°): sie halten wie ein längerer Haltesteg
toolpath-finish-lap-cuts = der Schlichtgang nimmt in Ecken oder engen Stellen bis zu { $most } ab (Aufmaß für den Schlichtgang { $stock })
# $style is the operation's corner style.
toolpath-corner-unreached = Ecke bei { $point }: das Werkzeug reicht nicht hinein; { $style ->
    [dogbone] kein Dogbone
   *[tbone] kein T-Bone
}

## Planning notes: pockets
## $contours is a list of contour numbers ("#3, #4"): the pocket's boundary and its islands.

toolpath-corners-not-rounded = Konturen { $contours }: Ringecken nicht verrundet; Verrunden würde Material stehen lassen
toolpath-pocket-narrow = Konturen { $contours }: das Werkzeug passt nicht durch eine enge Stelle; als { $areas } Bereiche ausgeräumt, dazwischen bleibt Material stehen
# $width is a number without its unit; $height brings the unit; $near is a point.
toolpath-uncut = Konturen { $contours }: ungefrästes Material an { $count ->
    [one] { $count } Stelle
   *[other] { $count } Stellen
}, die größte etwa { $width } x { $height } bei { $near }; verringern Sie die seitliche Zustellung, um es auszuräumen
toolpath-lead-reduced = Konturen { $contours }: Anfahrradius auf { $used } verringert (kein Platz für { $radius })
toolpath-no-lead = Konturen { $contours }: kein Platz für eine Anfahrt (nicht einmal { $least }); die Wand wird ohne Anfahrt gefräst
toolpath-helix-reduced = Konturen { $contours }: Helixradius auf { $used } verringert (kein Platz für { $radius })
toolpath-no-helix-room = Konturen { $contours }: kein Platz für eine Helix (nicht einmal { $least }); stattdessen Eintauchen per Rampe
toolpath-no-helix-narrow = Konturen { $contours }: kein Platz für eine Helix (die Tasche ist zu eng); stattdessen Eintauchen per Rampe
toolpath-no-helix-way = Konturen { $contours }: kein Platz für eine Helix (ihr Weg zur ersten Bahn kreuzt eine Wand oder Insel); stattdessen Eintauchen per Rampe
toolpath-start-far-wall = Konturen { $contours }: Startpunkt { $point } ist { $distance } von der Wand entfernt; der nächstgelegene Punkt wird verwendet
toolpath-start-twice-wall = Konturen { $contours }: Startpunkt { $point } liegt auf einer Wand, die schon einen hat; er wird ignoriert
toolpath-rest-nothing-left = das größere Werkzeug erreicht die ganze Tasche; für die Restbearbeitung bleibt nichts übrig
# $width is a number without its unit; $height brings the unit.
toolpath-rest-too-small = Restbearbeitung: Material an { $count ->
    [one] { $count } Stelle
   *[other] { $count } Stellen
} ist auch für dieses Werkzeug zu klein (die größte etwa { $width } × { $height })

## Planning notes: V-carving, follow paths and holes

# $across is the V-bit's diameter, $cap the deepest it carves, $wide the widest stroke it
# carves at that depth, $need the depth the widest stroke needs.
toolpath-vcarve-too-wide-bit = Konturen { $contours }: zu breit für den V-Fräser ({ $across } Durchmesser), der höchstens { $cap } tief graviert ({ $wide } breit an der Oberfläche); die Ränder der breitesten Stellen werden nicht erreicht (sie bräuchten { $need }). Erlauben Sie mehr Tiefe, nehmen Sie einen größeren Winkel oder räumen Sie den Boden aus
toolpath-vcarve-too-wide = Konturen { $contours }: zu breit für eine Gravur von höchstens { $cap } Tiefe ({ $wide } breit an der Oberfläche); die Ränder der breitesten Stellen werden nicht erreicht (sie bräuchten { $need }). Erlauben Sie mehr Tiefe, nehmen Sie einen größeren Winkel oder räumen Sie den Boden aus
# $narrow is a length of centre line, $tip the V-bit's tip diameter.
toolpath-vcarve-narrow = Konturen { $contours }: { $narrow } Mittellinie sind schmaler als die Spitze ({ $tip }) und werden nicht graviert
toolpath-path-too-short = Kontur #{ $contour } ist zu kurz zum Folgen ({ $length }); übersprungen
toolpath-groove-no-angle = Werkzeug { $tool } ist ein V-Fräser ohne Winkel: die Breite der Nut ist unbekannt
toolpath-groove-past-cone = die V-Nut ist tiefer als der Kegel des V-Fräsers: oben { $width } breit, mit senkrechten Wänden über dem Kegel
toolpath-groove-width = die V-Nut ist oben { $width } breit
toolpath-holes-not-circles = { $count ->
    [one] { $count } Form übersprungen, die kein Kreis ist
   *[other] { $count } Formen übersprungen, die keine Kreise sind
}
# $diameter is the holes' diameter, $tool the tool's, $within how far they may differ.
toolpath-holes-not-drill-size = { $count ->
    [one] Bohrung { $diameter }
   *[other] { $count } Bohrungen mit { $diameter }
}: nicht der Durchmesser des { $tool }-Bohrers (auf { $within } genau); nicht gebohrt
toolpath-holes-too-small-to-bore = { $count ->
    [one] Bohrung { $diameter }
   *[other] { $count } Bohrungen mit { $diameter }
}: zu klein zum Helixbohren mit dem { $tool }-Werkzeug; nicht gefräst
toolpath-holes-core = { $count ->
    [one] { $count } Bohrung lässt
   *[other] { $count } Bohrungen lassen
} einen losen Kern zurück (Bohrung größer als das Doppelte des Werkzeugs); sichern Sie ihn oder verwenden Sie eine Tasche

## Errors: why no toolpath could be made

toolpath-error-cancelled = Berechnung abgebrochen
# An error after where it happened: $context is an operation's label, or one of the
# context texts below.
toolpath-error-context = { $context }: { $error }
toolpath-error-label-contour = { $label }, Kontur #{ $contour }
# $part is a part's name, $copy the copy's number.
toolpath-error-part-copy = Teil { $part }, Kopie { $copy }
toolpath-error-copy = Kopie { $copy }
# $at is an operation's number in the cut list ("3", "3.1" for a rest pass).
toolpath-error-no-operation = Keine Bearbeitung { $at } im Job

## Errors: selecting shapes

# $shapes are shape numbers ("S4, S7").
toolpath-error-shape-missing = Form { $shapes } gehört zu keiner Kontur (gelöscht, oder ein doppeltes Segment oder eines der Länge null, das der Import entfernt hat)
# $layers are layer names, separated by commas.
toolpath-error-partly-selected = Kontur #{ $contour } ist nur teilweise ausgewählt (Layer { $layers }); korrigieren Sie die Zeichnung oder die Auswahl
# $reason is why the drawing's contour can't be machined, as the DXF import found it (in English).
toolpath-error-contour-invalid = Kontur #{ $contour } ist ungültig: { $reason }
# $selector is what the operation selects, built from the toolpath-error-selector texts.
toolpath-error-selects-nothing = Auswahl { $selector } trifft nichts
# $contours are contour numbers ("#3, #4").
toolpath-error-selects-open = Auswahl { $selector } trifft nur offene Konturen ({ $contours }); diese Bearbeitung braucht geschlossene
toolpath-error-selector-layer = Layer '{ $layer }'
toolpath-error-selector-shapes = Formen { $shapes }
toolpath-error-selector-and = { $first } und { $second }
# $id is the number of a DXF, a picture or a text in the project.
toolpath-error-selector-dxf = DXF { $id }
toolpath-error-selector-image = Bild { $id }
toolpath-error-selector-text = Text { $id }
# $shapes are the shapes left out ("S3, S5").
toolpath-error-selector-but = { $source } außer { $shapes }
toolpath-error-selector-of = { $what } aus { $source }
# $a and $b are contour numbers.
toolpath-error-same-shape = Konturen #{ $a } und #{ $b } sind dieselbe Form; wählen Sie nur eine davon aus

## Errors: operations and tools

toolpath-error-pass-depths = Tiefen müssen 0 <= Start ({ $start }) < Ende ({ $end }) erfüllen, und step_down ({ $step_down }) muss positiv sein
# $tool is a tool's number.
toolpath-error-rest-tool = Rest nach Werkzeug { $tool }, das es im Projekt nicht gibt
# $diameter is the hole's, $tool the tool's diameter.
toolpath-error-blind-bore = Bohrung { $diameter } ist ein Sackloch und mehr als doppelt so groß wie das Werkzeug ({ $tool }); verwenden Sie eine Tasche
# $tool is the tool's number, $type its kind as written in the project file (endmill, drill, vbit).
toolpath-error-no-hole-fits = kein ausgewählter Kreis kann mit Werkzeug { $tool } ({ $diameter } { $type }) gefräst werden
toolpath-error-no-depth = keine Tiefe zum Fräsen
toolpath-error-no-angle = Werkzeug { $tool } hat keinen Winkel
# $got is an angle in degrees, without its unit.
toolpath-error-vbit-angle = der Winkel des V-Fräsers muss mehr als 0 und weniger als 180 Grad sein, ist { $got }
# $got is a length, without its unit.
toolpath-error-stepover = die seitliche Zustellung muss positiv sein, ist { $got }

## Errors: profiles, ramps and tabs

toolpath-error-ring-no-length = ein Ring des Konturfräsens hat keine Länge
toolpath-error-lap-no-course = ein Umlauf an der Wand hat keinen Verlauf
# $angle is in degrees, without its unit; $length is how far the ring goes around.
toolpath-error-ring-ramp = Der Ring ist zu kurz, um per Rampe mit { $angle }° um { $drop } abzusenken ({ $length } Umfang); verwenden Sie einen steileren Rampenwinkel oder tauchen Sie mit einem über Mitte schneidenden Werkzeug senkrecht ein
# $reach is how much of the path the ramp may use.
toolpath-error-ramp-reach = Der Pfad ist zu kurz, um per Rampe mit { $angle }° um { $drop } abzusenken (nur { $reach } Weg für die Rampe); verwenden Sie einen steileren Rampenwinkel oder tauchen Sie mit einem über Mitte schneidenden Werkzeug senkrecht ein
toolpath-error-path-ramp = Der Pfad ist zu kurz, um per Rampe mit { $angle }° um { $drop } abzusenken ({ $length } lang); verwenden Sie einen steileren Rampenwinkel oder tauchen Sie mit einem über Mitte schneidenden Werkzeug senkrecht ein
# $angle is in degrees, without its unit.
toolpath-error-helix-turns = eine Helix mit Radius { $radius } bei { $angle }° bräuchte { $turns } Umdrehungen, um { $drop } abzusenken; verwenden Sie einen größeren Radius oder einen steileren Winkel
# $offset is how far inside the line the tool's centre would run.
toolpath-error-tool-does-not-fit = Das Werkzeug passt nicht in die Kontur (sein Mittelpunkt liefe { $offset } innerhalb der Linie)
toolpath-error-offset-verification = Offset hat die Prüfung nicht bestanden (Weg nicht im Abstand des Werkzeugradius von der Kontur); die Kontur ist möglicherweise entartet
# $farthest is how far the finishing lap runs from the roughing path.
toolpath-error-roughing-reach = das Schruppen erreicht einen Teil der Wand nicht (der Schlichtgang verläuft { $farthest } vom Schruppweg entfernt, einen Werkzeugradius oder mehr): verwenden Sie weniger Aufmaß für den Schlichtgang oder ein kleineres Werkzeug
# $fits is how many tabs would fit.
toolpath-error-tabs-fit = { $count ->
    [one] { $count } Haltesteg
   *[other] { $count } Haltestege
} von { $width } passen nicht auf einen Weg von { $length } ({ $fits ->
    [0] nicht einmal einer passt
    [one] höchstens { $fits } passt
   *[other] höchstens { $fits } passen
})
# $from is a distance along the ring without its unit, $to one with it.
toolpath-error-tab-carry = ein Haltesteg ({ $from } bis { $to } entlang des Rings) ließ sich nicht auf den Schrupp-Ring übertragen
# $what is one of the three texts after it.
toolpath-error-tab-verification = Werkzeugweg der Haltestege hat die Prüfung nicht bestanden: { $what }
# $z is a height, $at a distance along the ring.
toolpath-error-tab-below = Z{ $z } bei { $at } entlang des Rings liegt unter dem Haltesteg
toolpath-error-tab-back = das Werkzeug fährt bei { $at } entlang des Rings zurück
toolpath-error-tab-end-below = Z{ $z } am Ende des Haltestegs bei { $end } entlang des Rings liegt unter der Oberkante des Haltestegs Z{ $top }
# $label is the operation's label; $at the corner's point.
toolpath-error-corner-leg = { $label }: der Abstecher an Ecke { $at } ging nicht als Hin- und Rückweg in den Ring
toolpath-error-corner-cuts = { $label }: { $style ->
    [dogbone] der Dogbone
   *[tbone] der T-Bone
} an Ecke { $at } würde in Kontur #{ $contour } schneiden; wählen Sie für diese Ecke etwas anderes, verringern Sie den max. Winkel der Ecken oder ändern Sie die Zeichnung
toolpath-error-corner-cuts-own = { $label }: { $style ->
    [dogbone] der Dogbone
   *[tbone] der T-Bone
} an Ecke { $at } würde in eine andere Wand ihrer eigenen Kontur #{ $contour } schneiden; wählen Sie für diese Ecke etwas anderes (einen T-Bone entlang der anderen Wand, oder rund) oder ändern Sie die Zeichnung
toolpath-corner-choice-unmatched = { $label }: die bei { $at } gewählte Ecke ist keine Ecke seiner Formen mehr (hat sich die Zeichnung geändert?); entfernen Sie die Einstellung oder wählen Sie neu
# $x and $y are a point's coordinates, without their unit.
toolpath-error-arc-off-circle = Bogenausgabe: ({ $x }, { $y }) liegt nicht auf dem Kreis, dem der Punkt zugeordnet wurde

## Errors: pockets

# $contours are contour numbers ("#3, #4"): the pocket's boundary and its islands.
toolpath-error-islands-enclose = Konturen { $contours }: überlappende Inseln umschließen eine Fläche zwischen sich, was Taschen noch nicht unterstützen
toolpath-error-pocket-no-fit = Konturen { $contours }: Werkzeug (Durchmesser { $diameter }) passt nicht in die Tasche
toolpath-error-pocket-no-fit-clearance = Konturen { $contours }: Werkzeug (Durchmesser { $diameter }, Übermaß { $clearance }) passt nicht in die Tasche
toolpath-error-not-nested = Konturen { $contours }: Offsets der Tasche sind nicht ineinander verschachtelt
toolpath-error-too-many-rings = Konturen { $contours }: mehr als { $most } Ringe in der Tasche
toolpath-error-pocket-verification = Konturen { $contours }: Werkzeugweg der Tasche hat die Prüfung nicht bestanden (zu nah an einer Wand oder Insel); die Zeichnung ist möglicherweise entartet
toolpath-error-nothing-to-clear = Nichts auszuräumen: die ausgewählten Inseln bedecken die ausgewählten Begrenzungen (Konturen { $contours })
toolpath-error-island-outside = Taschen-Offset ergab eine Insel außerhalb aller Flächen
toolpath-error-shrink-hole = Taschen-Offset erzeugte beim Verkleinern einer einzelnen Schleife ein Loch
toolpath-error-open-loop = Taschen-Offset lieferte eine offene Schleife, wo eine geschlossene erwartet wurde
toolpath-error-rest-outside = der Bereich der Restbearbeitung liegt außerhalb der Tasche
toolpath-error-left-hole-outside = Restmaterial: ein Loch außerhalb aller Stücke
toolpath-error-left-holes-enclose = Restmaterial: Löcher umschließen eine Fläche
toolpath-error-left-wrong-area = Restmaterial: eine boolesche Operation ergab die falsche Fläche
toolpath-error-left-invalid = Restmaterial: ungültige Geometrie aus der Offset-Bibliothek

## Errors: V-carving, centre lines and letters

# What a centre line is for, inside the texts below.
toolpath-error-medial-name = { $name ->
    [vcarve] der V-Gravur
   *[centre-line] der nachgezeichneten Striche
}
# $name is toolpath-error-medial-name; $error the Voronoi library's own words.
toolpath-error-medial-failed = Konturen { $contours }: die Mittellinie { $name } konnte nicht erstellt werden ({ $error }); berührt oder kreuzt sich der Umriss selbst?
toolpath-error-medial-cell = Mittellinie { $name }: eine Zelle ohne ihre Sehne
# $reach is how far from the origin the centre line works.
toolpath-error-medial-far = ein Umriss { $name } bei { $at } liegt zu weit vom Ursprung der Zeichnung entfernt (die Mittellinie funktioniert innerhalb ±{ $reach })
toolpath-error-medial-step = Die Mittellinie { $name }
toolpath-error-vcarve-verification = Konturen { $contours }: Prüfung der V-Gravur fehlgeschlagen: der Schnitt würde den Umriss bei { $at } kreuzen
toolpath-error-join-gave = das Vereinigen zweier überlappender Umrisse ergab { $outlines } Umrisse
toolpath-error-join-changed = das Vereinigen überlappender Umrisse hat bei { $at } geändert, was gefüllt ist
toolpath-error-joined-cross = vereinigte Umrisse kreuzen sich
toolpath-error-outline-unreadable = ein Umriss, den die Geometriebibliothek nicht lesen kann

## Errors: copies and parts

# $width is the cut's width; $got the pitch, without its unit.
toolpath-error-copies-dx = Kopien überlappen: mit Spalten muss die Teilung dx mindestens { $width } sein (der Schnitt ist { $width } breit), ist { $got }
toolpath-error-copies-dy = Kopien überlappen: mit Zeilen muss die Teilung dy mindestens { $height } sein (der Schnitt ist { $height } hoch), ist { $got }
# $other and $copy are copies' numbers; $dx and $dy the pitch, without its unit.
toolpath-error-copies-overlap = Kopien überlappen: die Kopien { $other } und { $copy } schneiden ineinander; rücken Sie sie auseinander (Teilung dx { $dx }, dy { $dy })
# $part and $other_part are parts' names; $x0 and $y0 are without their unit.
toolpath-error-parts-overlap = Teil „{ $part }“ Kopie { $copy } überlappt Teil „{ $other_part }“ Kopie { $other_copy } (X{ $x0 } bis { $x1 }, Y{ $y0 } bis { $y1 }): rücken Sie die Kopien auseinander oder ändern Sie das Raster

## Errors: inside the geometry library

# $what is toolpath-error-step or toolpath-error-medial-step.
toolpath-error-library-failed = { $what } ist in der Geometriebibliothek fehlgeschlagen
toolpath-error-step = { $step ->
    [pocket-offset] Taschen-Offset
    [joining-letters] Vereinigen der Buchstaben
    [checking-joined-letters] Prüfen der vereinigten Buchstaben
    [material-left-union] Vereinigung des Restmaterials
    [material-left-offset] Offset des Restmaterials
    [material-left-boolean] Boolesche Operation des Restmaterials
    [comparing-shapes] Vergleichen der Formen
    [merging-islands] Zusammenführen der Inseln
    [subtracting-an-island] Abziehen einer Insel
   *[offset] Offset
}
