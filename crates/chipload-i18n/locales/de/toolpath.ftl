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

## Errors: why no toolpath could be made

toolpath-error-cancelled = Generation cancelled
# An error after where it happened: $context is an operation's label, or one of the
# context texts below.
toolpath-error-context = { $context }: { $error }
toolpath-error-label-contour = { $label }, contour #{ $contour }
# $part is a part's name, $copy the copy's number.
toolpath-error-part-copy = part { $part }, copy { $copy }
toolpath-error-copy = copy { $copy }
# $at is an operation's number in the cut list ("3", "3.1" for a rest pass).
toolpath-error-no-operation = No operation { $at } in the job

## Errors: selecting shapes

# $shapes are shape numbers ("S4, S7").
toolpath-error-shape-missing = shape { $shapes } is not part of any contour (deleted, or a duplicate or zero-length segment the import removed)
# $layers are layer names, separated by commas.
toolpath-error-partly-selected = contour #{ $contour } is only partly selected (layers { $layers }); fix the drawing or the selector
# $reason is why the drawing's contour can't be machined, as the DXF import found it (in English).
toolpath-error-contour-invalid = contour #{ $contour } is invalid: { $reason }
# $selector is what the operation selects, built from the toolpath-error-selector texts.
toolpath-error-selects-nothing = selector { $selector } matches nothing
# $contours are contour numbers ("#3, #4").
toolpath-error-selects-open = selector { $selector } matches only open contours ({ $contours }); this operation needs closed ones
toolpath-error-selector-layer = layer '{ $layer }'
toolpath-error-selector-shapes = shapes { $shapes }
toolpath-error-selector-and = { $first } and { $second }
# $id is the number of a DXF, a picture or a text in the project.
toolpath-error-selector-dxf = DXF { $id }
toolpath-error-selector-image = image { $id }
toolpath-error-selector-text = text { $id }
# $shapes are the shapes left out ("S3, S5").
toolpath-error-selector-but = { $source } but { $shapes }
toolpath-error-selector-of = { $what } of { $source }
# $a and $b are contour numbers.
toolpath-error-same-shape = contours #{ $a } and #{ $b } are the same shape; select only one of them

## Errors: operations and tools

toolpath-error-pass-depths = depths must satisfy 0 <= start ({ $start }) < end ({ $end }) and step_down ({ $step_down }) must be positive
# $tool is a tool's number.
toolpath-error-rest-tool = rest after tool { $tool }, which the project doesn't have
# $diameter is the hole's, $tool the tool's diameter.
toolpath-error-blind-bore = hole { $diameter } is blind and more than twice the { $tool } tool; use a pocket
# $tool is the tool's number, $type its kind as written in the project file (endmill, drill, vbit).
toolpath-error-no-hole-fits = no selected circle can be cut with tool { $tool } ({ $diameter } { $type })
toolpath-error-no-depth = no depth to cut to
toolpath-error-no-angle = tool { $tool } has no angle
# $got is an angle in degrees, without its unit.
toolpath-error-vbit-angle = the V-bit angle must be more than 0 and less than 180 degrees, got { $got }
# $got is a length, without its unit.
toolpath-error-stepover = stepover must be positive, got { $got }

## Errors: profiles, ramps and tabs

toolpath-error-ring-no-length = a profile ring has no length
toolpath-error-lap-no-course = a wall lap has no course
# $angle is in degrees, without its unit; $length is how far the ring goes around.
toolpath-error-ring-ramp = The ring is too short to ramp down { $drop } at { $angle }° ({ $length } around); use a steeper ramp angle or a straight plunge with a centre-cutting tool
# $reach is how much of the path the ramp may use.
toolpath-error-ramp-reach = The path is too short to ramp down { $drop } at { $angle }° (only { $reach } to ramp along); use a steeper ramp angle or a straight plunge with a centre-cutting tool
toolpath-error-path-ramp = The path is too short to ramp down { $drop } at { $angle }° ({ $length } long); use a steeper ramp angle or a straight plunge with a centre-cutting tool
# $angle is in degrees, without its unit.
toolpath-error-helix-turns = a helix of radius { $radius } at { $angle }° would need { $turns } turns to go down { $drop }; use a larger radius or a steeper angle
# $offset is how far inside the line the tool's centre would run.
toolpath-error-tool-does-not-fit = Tool does not fit inside the contour (its centre would run { $offset } inside the line)
toolpath-error-offset-verification = Offset failed verification (path not at tool radius from the contour); the contour may be degenerate
# $farthest is how far the finishing lap runs from the roughing path.
toolpath-error-roughing-reach = the roughing can't reach part of the wall (the finishing lap runs { $farthest } from the roughing path, a tool radius or more): use less finishing stock or a smaller tool
# $fits is how many tabs would fit.
toolpath-error-tabs-fit = { $count } tabs of { $width } don't fit on a { $length } path ({ $fits ->
    [0] not even one fits
   *[other] at most { $fits } fit
})
# $from is a distance along the ring without its unit, $to one with it.
toolpath-error-tab-carry = could not carry a tab ({ $from } to { $to } along the ring) onto the roughing ring
# $what is one of the three texts after it.
toolpath-error-tab-verification = Tab toolpath failed verification: { $what }
# $z is a height, $at a distance along the ring.
toolpath-error-tab-below = Z{ $z } at { $at } along the ring is below the tab
toolpath-error-tab-back = the tool goes back along the ring at { $at }
toolpath-error-tab-end-below = Z{ $z } at the tab end { $end } along the ring is below the tab top Z{ $top }
# $label is the operation's label; $at the corner's point.
toolpath-error-corner-leg = { $label }: the leg at corner { $at } did not go into the ring as out and back
toolpath-error-corner-cuts = { $label }: the { $style ->
    [dogbone] dogbone
   *[tbone] T-bone
} at corner { $at } would cut into contour #{ $contour }; lower the corners' max angle or change the drawing
# $x and $y are a point's coordinates, without their unit.
toolpath-error-arc-off-circle = arc output: ({ $x }, { $y }) is not on the circle it was tagged with

## Errors: pockets

# $contours are contour numbers ("#3, #4"): the pocket's boundary and its islands.
toolpath-error-islands-enclose = contours { $contours }: overlapping islands enclose an area between them, which pockets don't support yet
toolpath-error-pocket-no-fit = contours { $contours }: tool (diameter { $diameter }) does not fit in the pocket
toolpath-error-pocket-no-fit-clearance = contours { $contours }: tool (diameter { $diameter }, clearance { $clearance }) does not fit in the pocket
toolpath-error-not-nested = contours { $contours }: pocket offsets are not nested
toolpath-error-too-many-rings = contours { $contours }: more than { $most } pocket rings
toolpath-error-pocket-verification = contours { $contours }: pocket toolpath failed verification (too close to a wall or island); the drawing may be degenerate
toolpath-error-nothing-to-clear = Nothing to clear: the selected islands cover the selected boundaries (contours { $contours })
toolpath-error-island-outside = pocket offset produced an island outside every area
toolpath-error-shrink-hole = pocket offset made a hole while shrinking a single loop
toolpath-error-open-loop = pocket offset returned an open loop where a closed one was expected
toolpath-error-rest-outside = the rest machining area lies outside the pocket
toolpath-error-left-hole-outside = material left: a hole outside every piece
toolpath-error-left-holes-enclose = material left: holes enclose an area
toolpath-error-left-wrong-area = material left: a boolean gave the wrong area
toolpath-error-left-invalid = material left: invalid geometry from the offset library

## Errors: V-carving, centre lines and letters

# What a centre line is for, inside the texts below.
toolpath-error-medial-name = { $name ->
    [vcarve] V-carve
   *[centre-line] centre line
}
# $name is toolpath-error-medial-name; $error the Voronoi library's own words.
toolpath-error-medial-failed = contours { $contours }: the { $name } centre line could not be built ({ $error }); does the outline touch or cross itself?
toolpath-error-medial-cell = { $name } centre line: a cell without its chord
# $reach is how far from the origin the centre line works.
toolpath-error-medial-far = a { $name } outline at { $at } is too far from the drawing origin (the centre line works within ±{ $reach })
toolpath-error-medial-step = The { $name } centre line
toolpath-error-vcarve-verification = contours { $contours }: V-carve verification failed: the cut would cross the outline near { $at }
toolpath-error-join-gave = joining two overlapping outlines gave { $outlines } outlines
toolpath-error-join-changed = joining overlapping outlines changed what is filled near { $at }
toolpath-error-joined-cross = joined outlines cross each other
toolpath-error-outline-unreadable = an outline the geometry library can't read

## Errors: copies and parts

# $width is the cut's width; $got the pitch, without its unit.
toolpath-error-copies-dx = Copies overlap: with columns the pitch dx must be at least { $width } (the cut is { $width } wide), got { $got }
toolpath-error-copies-dy = Copies overlap: with rows the pitch dy must be at least { $height } (the cut is { $height } high), got { $got }
# $other and $copy are copies' numbers; $dx and $dy the pitch, without its unit.
toolpath-error-copies-overlap = Copies overlap: copies { $other } and { $copy } cut into each other; move them apart (pitch dx { $dx }, dy { $dy })
# $part and $other_part are parts' names; $x0 and $y0 are without their unit.
toolpath-error-parts-overlap = Part "{ $part }" copy { $copy } overlaps part "{ $other_part }" copy { $other_copy } (X{ $x0 } to { $x1 }, Y{ $y0 } to { $y1 }): move the copies apart or change the grid

## Errors: inside the geometry library

# $what is toolpath-error-step or toolpath-error-medial-step.
toolpath-error-library-failed = { $what } failed inside the geometry library
toolpath-error-step = { $step ->
    [pocket-offset] Pocket offset
    [joining-letters] Joining letters
    [checking-joined-letters] Checking joined letters
    [material-left-union] Material left union
    [material-left-offset] Material left offset
    [material-left-boolean] Material left boolean
    [comparing-shapes] Comparing shapes
    [merging-islands] Merging islands
    [subtracting-an-island] Subtracting an island
   *[offset] Offset
}
