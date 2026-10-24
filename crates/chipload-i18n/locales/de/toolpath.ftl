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
toolpath-no-rpm = Werkzeug { $tool } hat keine Drehzahl (U/min)
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
toolpath-rest-pass-no-rpm = Werkzeug { $tool } hat keine Drehzahl (U/min)

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
