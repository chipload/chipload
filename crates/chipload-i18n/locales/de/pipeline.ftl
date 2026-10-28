# Messages of the app services (crates/chipload-pipeline): problems found with the
# post-processor a project uses.

# $post is the post-processor's name, $units the G-code units the machine is set to, as
# written in the project file ("inch"). "units" is a key of the post's config file.
pipeline-post-units = Postprozessor { $post } schreibt nur G-Code in mm (seine Konfiguration führt "{ $units }" nicht unter units)

## Planning notes

pipeline-drawing-outside-stock = Die Zeichnung ragt über das Rohteil hinaus
# $count is the number of copies. $x0 and $y0 are numbers without their unit; $x1, $y1
# and $stock_y bring it; $stock_x is a number without it.
pipeline-copies-beyond-stock = Die { $count } Kopien fräsen X{ $x0 } bis { $x1 }, Y{ $y0 } bis { $y1 }, über das Rohteil hinaus ({ $stock_x } × { $stock_y }); prüfen Sie das Rohteil oder die Kopien des Teils.
# As pipeline-copies-beyond-stock, for one part of several; $part is its name.
pipeline-part-copies-beyond-stock = Teil { $part }: Die { $count } Kopien fräsen X{ $x0 } bis { $x1 }, Y{ $y0 } bis { $y1 }, über das Rohteil hinaus ({ $stock_x } × { $stock_y }); prüfen Sie das Rohteil oder die Kopien des Teils.
# $label is an operation's label, $layer a layer's name, $sources where the layer's shapes
# come from (DXF and picture names, joined with pipeline-and).
pipeline-layer-clash = { $label }: Layer "{ $layer }" ist in { $sources }; die Bearbeitung fräst alle davon. Beschränken Sie sie auf eines (wählen Sie den Layer unter dieser DXF im Browser)
# Names joined: "a.dxf and b.dxf".
pipeline-and = { $list } und { $last }
# Where shapes come from: a DXF's or picture's name, or one without a name.
pipeline-source-named = { $name }
pipeline-source-dxf = DXF { $id }
pipeline-source-image = Bild { $id }
pipeline-source-drawn = gezeichnete Formen

## The time estimate (after planning and generating)

# A duration: "45 s", "12 min 30 s", "1 h 05 min". $minutes after hours has two digits.
pipeline-duration-seconds = { $seconds } s
pipeline-duration-minutes = { $minutes } min { $seconds } s
pipeline-duration-hours = { $hours } h { $minutes } min
# The durations are pipeline-duration-… texts; $changes is the number of tool changes.
pipeline-estimate = Geschätzte Zeit: { $total } (Fräsen { $cutting }, Eilgänge { $rapids }, Spindel { $spindle }){ $changes ->
    [0] {""}
    [one] , plus { $changes } Werkzeugwechsel
   *[other] , plus { $changes } Werkzeugwechsel
}
# The lines under it start with spaces, written {"  "}: keep them, they indent the line.
# $name is the part's name.
pipeline-estimate-part = {"  "}Teil { $number } ({ $name }): { $time }
# $label is an operation's label.
pipeline-estimate-operation = {"  "}{ $label }: { $time }
# An operation under its part: indented deeper.
pipeline-estimate-part-operation = {"    "}{ $label }: { $time }
# One G-code file of a project written one file per tool.
pipeline-estimate-file = {"  "}Datei { $number }: { $time }
# $rate is a feed ("5000 mm/min").
pipeline-estimate-rapids-assumed = {"  "}Bei den programmierten Vorschüben (Eilgänge mit { $rate }, angenommen: geben Sie den max. Vorschub der Maschine an, um besser zu schätzen); Beschleunigung und Ecken machen die tatsächliche Zeit länger.
pipeline-estimate-rapids = {"  "}Bei den programmierten Vorschüben (Eilgänge mit dem max. Vorschub der Maschine, { $rate }); Beschleunigung und Ecken machen die tatsächliche Zeit länger.

## Chip load

pipeline-chip-load-title = Zahnvorschub (beim Fräsvorschub):
# $label is an operation's label, $load a pipeline-chip-load… text.
pipeline-chip-load-line = {"  "}{ $label }: { $load }
# $load brings its unit ("0.042 mm/tooth").
pipeline-chip-load = { $load }
pipeline-chip-load-no-flutes = geben Sie die Schneiden des Werkzeugs an, um den Zahnvorschub zu sehen
pipeline-chip-load-no-rpm = geben Sie die Drehzahl des Werkzeugs an, um den Zahnvorschub zu sehen
pipeline-chip-load-no-tool = kein Werkzeug
# The hover text next to an operation's chip load. Tool diameters $small_tool (a number)
# and $large_tool (with its unit); chip loads $wood_low, $aluminium_low and
# $aluminium_high (numbers) and $wood_high (with its unit).
pipeline-chip-load-hint = Zahnvorschub: wie dick der Span ist, den jede Schneide abnimmt, Vorschub / (Drehzahl × Schneiden). Zu dünn, und das Werkzeug reibt und wird heiß; zu dick, und es kann brechen. Er hängt vom Material, vom Werkzeug und von der Maschine ab; als grober Anhalt vertragen Holz und Kunststoffe mit Werkzeugen von { $small_tool }–{ $large_tool } oft { $wood_low }–{ $wood_high }, Aluminium { $aluminium_low }–{ $aluminium_high }. Prüfen Sie die Angaben des Werkzeugherstellers.

## The drawing's facts (after importing)

# $units is "mm" or "inch", as the DXF gives them or the user chose them.
pipeline-report-units = Einheiten: { $units } ({ $from ->
    [dxf] aus der DXF
    [svg] aus dem SVG
   *[user] vom Benutzer gewählt
})
# Numbers without their unit, except $height, which brings it.
pipeline-report-extents = Ausdehnung: X { $x0 } .. { $x1 }, Y { $y0 } .. { $y1 } ({ $width } x { $height })
# $items lists DXF entity types with their counts ("LINE 4, ARC 2").
pipeline-report-imported = Importiert: { $items }
pipeline-report-flattened = Abgeflacht: { $items }
pipeline-report-ignored = Ignoriert: { $items }
pipeline-report-no-issues = Probleme: keine
pipeline-report-issues = Probleme ({ $count }):
# One issue of the import (a dxf-… text), indented.
pipeline-report-issue = {"  "}[{ $severity ->
    [warning] Warnung
   *[error] Fehler
}] { $issue }
pipeline-report-contours = Konturen ({ $count }):
# A row of the contour table, in a fixed-width font: $id, $state and $layers are padded
# with spaces to line up. The coordinates are numbers without their unit.
pipeline-report-contour = {"  "}#{ $id } { $state } Layer { $layers } X { $x0 } .. { $x1 }  Y { $y0 } .. { $y1 }
# A contour's state in the table: keep it short.
pipeline-report-contour-state = { $state ->
    [open] offen
    [invalid] UNGÜLTIG
    [hole] Loch
   *[outer] außen
}

## The G-code checker's findings (check-… texts), placed in the program

# A finding on a line of the program; $message is the finding.
pipeline-check-line = Zeile { $line }: { $message }
# A finding in one of several G-code files: $file is the file's number.
pipeline-check-file = Datei { $file }: { $message }
# A finding on a line of one of several G-code files.
pipeline-check-file-line = Datei { $file }, Zeile { $line }: { $message }

## Errors: a change or run the project refuses

# The independent checker refused the program; each finding follows on its own line.
pipeline-error-check-failed = G-Code-Prüfung fehlgeschlagen:
# One more finding (a pipeline-check-… text) on a line of its own, indented, after $list.
pipeline-error-check-finding =
    { $list }
    {"  "}{ $finding }
# Words of the command line or another library, kept as they are.
pipeline-text = { $text }

## Reports of changes to the drawing

# $x0 and $y0 are lengths without their unit, $x1 and $y1 with it; $width and $length the
# stock's size, the first without its unit.
pipeline-stock-fit = Die Zeichnung (X{ $x0 } bis { $x1 }, Y{ $y0 } bis { $y1 }) ragt über das Rohteil hinaus ({ $width } × { $length }); prüfen Sie das Rohteil unter Jobeinstellungen.

## Placing a DXF: moving, turning and scaling it

# $count is how many DXFs the project has, $dxfs their numbers and names ("1 part-a, 2 part-b").
pipeline-place-which-dxf = das Projekt hat { $count } DXFs ({ $dxfs }): geben Sie an, welche platziert werden soll
pipeline-no-dxf = es gibt keine DXF { $id }
pipeline-place-nothing-imported = das Projekt hat keine importierte Zeichnung zum Verschieben
# What is placed, when the project has a single DXF (otherwise it is the DXF's name).
pipeline-place-the-drawing = die importierte Zeichnung
# $dx and $dy are numbers as typed, without a unit.
pipeline-place-cannot-move = die Zeichnung kann nicht um ({ $dx }, { $dy }) verschoben werden
pipeline-place-already-there = Die Zeichnung ist schon dort; nichts verschoben.
# $what is pipeline-place-the-drawing or a DXF's name; $degrees without its unit.
pipeline-place-turned = Gedreht: { $what }, { $degrees }° { $direction ->
    [ccw] gegen den Uhrzeigersinn
   *[cw] im Uhrzeigersinn
}
pipeline-place-reset = Zurückgesetzt: { $what } liegt wieder wie in der DXF
# $factor is a scale factor ("2", "0.5").
pipeline-place-scaled = Skaliert: { $what } ×{ $factor }
# $scaled is pipeline-place-scaled; $width is a length without its unit, $height with it.
pipeline-place-scaled-size = { $scaled } (jetzt { $width } × { $height })
# $dx is a length without its unit, $dy with it.
pipeline-place-moved = Verschoben: { $what } um X{ $dx } Y{ $dy }
# $done is one of the texts above; $placement a pipeline-placement text.
pipeline-place-done = { $done }; sie ist jetzt { $placement }.
pipeline-place-moved-starts = { $count ->
    [one] { $count } Startpunkt mitverschoben.
   *[other] { $count } Startpunkte mitverschoben.
}
pipeline-place-moved-corners = { $count ->
    [one] { $count } Eckeneinstellung mitverschoben.
   *[other] { $count } Eckeneinstellungen mitverschoben.
}
pipeline-place-moved-tabs = { $count ->
    [one] { $count } von Hand gesetzter Haltesteg mitverschoben.
   *[other] { $count } von Hand gesetzte Haltestege mitverschoben.
}
pipeline-place-drawn-stay = In Chipload gezeichnete Formen bleiben, wo sie sind.
pipeline-place-other-dxfs-stay = Die anderen DXFs bleiben, wo sie sind.
pipeline-place-work-zero-stays = Der Werkstücknullpunkt bleibt am Punkt X{ $x }, Y{ $y }; prüfen Sie, ob er noch dort liegt, wo Sie ihn haben wollen.
pipeline-place-margin = der Randabstand muss 0 oder mehr sein, ist { $got }
pipeline-place-stock-follows = das Rohteil ist mittig auf der Zeichnung und folgt ihr, Ausrichten der Zeichnung daran ändert also nichts: wählen Sie zuerst eine Rohteillage am Zeichnungsursprung
pipeline-cut-outline = Umriss des Schnitts: { $width } × { $length } (X { $x0 } … { $x1 }, Y { $y0 } … { $y1 }, mit dem Werkzeug): das kleinste Rohteil, das den Auftrag fasst.
# $got is a number as typed, in degrees, without a unit.
pipeline-place-rotation = die Drehung muss eine Zahl in Grad sein, ist { $got }
pipeline-place-scale = der Maßstab muss eine Zahl größer als 0 sein, ist { $got }
# Scale factors: the result, and the smallest and largest allowed.
pipeline-place-scale-range = das würde die Zeichnung gegenüber der DXF ×{ $factor } skalieren; es muss zwischen ×{ $min } und ×{ $max } bleiben
# $what is "width" or "height"; $got a length without its unit.
pipeline-place-size = { $what ->
    [height] die Höhe
   *[width] die Breite
} muss größer als 0 sein, ist { $got }
pipeline-place-no-size = die importierte Zeichnung hat keine { $what ->
    [height] Höhe
   *[width] Breite
}, die sich skalieren ließe
# How a DXF is placed: "scaled ×2, turned 90°, moved X12.000 Y-3.500 from the DXF".
pipeline-placement-as-dxf = wie in der DXF
pipeline-placement-from-dxf = { $parts } gegenüber der DXF
pipeline-placement-and = { $list }, { $last }
pipeline-placement-scaled = ×{ $factor } skaliert
pipeline-placement-turned = { $degrees }° gedreht
# $dx and $dy are lengths without their unit.
pipeline-placement-moved = um X{ $dx } Y{ $dy } verschoben
# After scaling: $within and $off are lengths, $factor the scale factor.
pipeline-scale-curves = { $count ->
    [one] { $count } Form aus Splines oder Ellipsen wurde
   *[other] { $count } Formen aus Splines oder Ellipsen wurden
} bei der vorherigen Größe auf { $within } genau angenähert; ×{ $factor } skaliert können sie bis zu { $off } von ihren Kurven abweichen. Aktualisieren Sie aus der DXF, um sie in dieser Größe neu anzunähern.
pipeline-scale-contours = Geschlossene Konturen: vorher { $was }, jetzt { $now }: in der neuen Größe werden Lücken bis { $within } geschlossen. Prüfen Sie den Importbericht und die Auswahl der Bearbeitungen.
# --units is an option of the command line.
pipeline-scale-units = Falls die Einheiten der DXF falsch sind, setzen Sie sie stattdessen beim Aktualisieren (--units), damit das Projekt sie festhält.

## Adding, updating and removing a DXF

# $file is the DXF's file, $id its number, $name its name in the project.
pipeline-dxf-added = { $file } als { $kind } { $id } „{ $name }“ hinzugefügt: { $count } { $count ->
    [one] Form
   *[other] Formen
}.
# $operations are operation numbers ("2, 5"); $name a DXF's name.
pipeline-dxf-pinned = { $count ->
    [one] Bearbeitung { $operations } fräst weiterhin nur { $name }: die neue DXF hat deren Layer auch.
   *[other] Die Bearbeitungen { $operations } fräsen weiterhin nur { $name }: die neue DXF hat deren Layer auch.
}
pipeline-dxf-removed = { $kind } { $id } „{ $name }“ und ihre { $count } { $count ->
    [one] Form
   *[other] Formen
} entfernt.
# $label is an operation's label; $shapes shape numbers ("S3, S4").
pipeline-dxf-shapes-gone = { $label }: die Formen { $shapes } gibt es nicht mehr; wählen Sie ihre Geometrie neu
pipeline-dxf-gone = { $label }: ihre DXF gibt es nicht mehr; wählen Sie ihre Geometrie neu
pipeline-dxf-updated = Aus { $file } aktualisiert: { $unchanged } unverändert, { $changed } geändert, { $added } hinzugefügt, { $removed } entfernt.
# $placement is a pipeline-placement text.
pipeline-dxf-placed-as-before = Platziert wie zuvor: { $placement }.
pipeline-dxf-geometry-changed = { $label }: Geometrie geändert (Formen { $shapes }); prüfen Sie ihren Werkzeugweg
pipeline-dxf-layer-updated = { $label }: Layer { $layer } verwendet jetzt die aktualisierte Zeichnung; prüfen Sie ihren Werkzeugweg

## Pictures and texts

# $number is an operation's number.
pipeline-left-out-found = Bearbeitung { $number }: { $count ->
    [one] ihre { $count } ausgelassene Form wiedergefunden.
   *[other] ihre { $count } ausgelassenen Formen wiedergefunden.
}
pipeline-left-out-lost = Bearbeitung { $number }: { $count ->
    [one] { $count } ausgelassene Form nicht wiedergefunden
   *[other] { $count } ausgelassene Formen nicht wiedergefunden
}: wählen Sie die auszulassenden Formen neu.
pipeline-no-image = es gibt kein Bild { $id }
# $error is the PNG writer's own words.
pipeline-image-png-failed = Schreiben des Bildes: { $error }
# drawing.images is a place in the project file; $error the decoder's own words.
pipeline-image-unreadable = drawing.images: die Bilddatei von Bild { $id } kann nicht gelesen werden: { $error }
# $file is the picture's file; $width a length without its unit, $height with it;
# $threshold a grey level (0–255); $picked "yes" when the threshold was picked from the
# picture; $lines "yes" for centre lines, "no" for outlines.
pipeline-image-traced = { $file } nachgezeichnet ({ $width } × { $height }, Schwellwert { $threshold }{ $picked ->
    [yes] , gewählt
   *[no] {""}
}): { $count } { $lines ->
    [yes] { $count ->
        [one] Mittellinie
       *[other] Mittellinien
    }
   *[no] { $count ->
        [one] Form
       *[other] Formen
    }
} auf Layer { $layer }.
pipeline-image-nothing-black = Nichts ist schwarz: ändern Sie den Schwellwert, oder invertieren Sie, falls das Bild hell auf dunkel ist.
# $narrow is a length without its unit, $wide with it.
pipeline-image-strokes = Striche { $narrow } bis { $wide } breit: fräsen Sie sie mit Pfad folgen.
pipeline-image-closed-lines = { $count ->
    [one] { $count } geschlossene Linie: ein Konturfräsen oder eine Tasche kann um sie herum fräsen, wie um jeden Umriss.
   *[other] { $count } geschlossene Linien: ein Konturfräsen oder eine Tasche kann um sie herum fräsen, wie um jeden Umriss.
}
pipeline-image-dots = { $count } { $count ->
    [one] Punkt
   *[other] Punkte
} zu klein für eine Mittellinie, ausgelassen.
# $area is the smallest piece's area in mm², without its unit.
pipeline-image-specks = { $count } { $count ->
    [one] Fleck
   *[other] Flecken
} kleiner als { $area } mm² verworfen.
pipeline-image-black-at-border = Schwarz berührt den Bildrand und wird entlang des Rands geschlossen; falls das der Hintergrund ist, invertieren Sie.
pipeline-image-without-arcs = { $count ->
    [one] { $count } Form als Linien behalten: ihre Bögen kamen einer anderen Form zu nahe.
   *[other] { $count } Formen als Linien behalten: ihre Bögen kamen einer anderen Form zu nahe.
}
pipeline-image-unsimplified = { $count } { $count ->
    [one] Form
   *[other] Formen
} wie nachgezeichnet behalten: das Glätten ließ sie sich kreuzen.
pipeline-no-text = es gibt keinen Text { $id }
# $text is the text's first line in quotes; $error why (a toolpath-error text).
pipeline-text-join-failed = { $text }: die überlappenden Umrisse der Buchstaben konnten nicht vereinigt werden: { $error }
# $text is the text's first line in quotes; $height its height; $width the box's width
# without its unit, $tall its height with it.
pipeline-text-wrote = { $text } in { $font } geschrieben, { $height } hoch ({ $width } × { $tall }): { $count } { $count ->
    [one] Form
   *[other] Formen
} auf Layer { $layer }.
# $missing are the characters the font has no letter for.
pipeline-text-missing = { $font } hat keinen Buchstaben für „{ $missing }“: ausgelassen.
pipeline-text-dropped = { $count } { $count ->
    [one] Umriss
   *[other] Umrisse
} zu klein zum Fräsen, ausgelassen.

## Errors: operations and tools

# `tool use` is a command of the command line: keep it as written.
pipeline-no-tool-use = das Projekt hat kein Werkzeug { $tool }; kopieren Sie eines mit `tool use` aus Ihrer Bibliothek hinein
pipeline-no-tool = das Projekt hat kein Werkzeug { $tool }
# $number is the operation's number as listed, $count how many the project has.
pipeline-no-operation = es gibt keine Bearbeitung { $number }; das Projekt hat { $count }
pipeline-tab-no-ring = Bearbeitung { $number } hat keine Kontur innerhalb eines Werkzeugdurchmessers von { $at }; `chipload op tabs` listet ihre Haltestege
pipeline-tab-none-there = Bearbeitung { $number } hat keinen Haltesteg innerhalb eines Werkzeugdurchmessers von { $at }; `chipload op tabs` listet ihre Haltestege
pipeline-tab-no-tabs = Bearbeitung { $number } hat keine Haltestege: nur Konturfräsen mit Haltestegen haben welche zu setzen
pipeline-corner-none-there = Bearbeitung { $number } hat keine Ecke bei { $at }; `chipload op corners` listet ihre Ecken
pipeline-corner-not-profile-or-pocket = Bearbeitung { $number } hat keine Ecken zum Wählen: nur Konturfräsen und Taschen haben Dogbones
pipeline-not-a-pocket = Bearbeitung { $number } ist keine Tasche; nur Taschen haben eine Restbearbeitung
# $reason is one of the pipeline-rest-… texts.
pipeline-operation-refused = Bearbeitung { $number }: { $reason }
pipeline-rest-dogbones = sie hat Dogbones, und Dogbones und Restbearbeitung lassen sich noch nicht kombinieren; Dogbone-Ecken brauchen keinen Rest
pipeline-rest-not-endmill = Werkzeug { $tool } ist kein Schaftfräser
pipeline-rest-before-not-endmill = Werkzeug { $tool } davor ist kein Schaftfräser
pipeline-rest-not-smaller = Werkzeug { $tool } ({ $diameter }) ist nicht kleiner als Werkzeug { $before } ({ $before_diameter }) davor
# $dxfs lists the project's DXFs by number and name ("1 part-a, 2 part-b").
pipeline-update-which-dxf = das Projekt hat { $count } DXFs ({ $dxfs }): geben Sie an, welche aktualisiert werden soll
# `tool use` is a command of the command line: keep it as written.
pipeline-surface-no-endmill = das Projekt hat keinen Schaftfräser: Planfräsen braucht einen (kopieren Sie einen mit `tool use` hinein)
# $type is the tool's type as written in the project file (drill, vbit).
pipeline-surface-not-endmill = Werkzeug { $tool } ist ein { $type }; Planfräsen braucht einen Schaftfräser

## Errors: drawn shapes

pipeline-shape-not-drawable = nur Kreise, Rechtecke und Polylinien können gezeichnet werden
# $kind is the shape's kind as written in the project file (circle, rectangle, polyline).
pipeline-shape-kind = Form { $id } ist vom Typ { $kind }; ihr Typ kann sich nicht ändern
pipeline-no-shapes-given = keine Formen angegeben
pipeline-shape-twice = Form { $id } ist zweimal angegeben
pipeline-no-shape = es gibt keine Form { $id }
pipeline-shape-imported-copy = Form { $id } wurde aus der DXF importiert; nur gezeichnete Formen können kopiert oder gespiegelt werden
pipeline-shapes-no-extent = die Formen haben keine Ausdehnung
pipeline-shape-imported-transform = Form { $id } ist keine gezeichnete Form: nur gezeichnete Formen lassen sich verschieben, drehen oder skalieren (lösen Sie zuerst ihre DXF auf)
pipeline-construction-imported = Form { $id } ist keine gezeichnete Form: nur gezeichnete Formen können Hilfsgeometrie sein (lösen Sie zuerst ihre DXF auf)
pipeline-construction-used = { $label } nennt Hilfsgeometrie, die nie gefräst wird: sie kann erst laufen, wenn sie andere Formen bearbeitet.
pipeline-transform-not-numbers = Verschiebung, Drehung und Mittelpunkt müssen Zahlen sein
pipeline-shapes-copy-imported = nur gezeichnete Formen können kopiert werden, keine importierten
# "Update drawing from DXF" is the command in the app's File menu.
pipeline-shape-imported-change = Form { $id } wurde aus der DXF importiert und kann hier nicht geändert werden; ändern Sie die Zeichnung und verwenden Sie Zeichnung aus DXF aktualisieren

## Errors: parts and copies

pipeline-no-part = Kein Teil { $id } im Projekt
pipeline-part-name-empty = Der Name eines Teils darf nicht leer sein
pipeline-part-name-taken = Ein anderes Teil heißt „{ $name }“
pipeline-last-part = Ein Projekt braucht mindestens ein Teil: dies ist das letzte
pipeline-part-has-operations = Teil { $name } hat { $count ->
    [one] { $count } Bearbeitung
   *[other] { $count } Bearbeitungen
}: verschieben Sie sie zuerst in ein anderes Teil, oder entfernen Sie sie mit ihm
pipeline-part-no-operation = Keine Bearbeitung { $number } im Projekt
pipeline-part-cuts-nothing = Das Teil fräst nichts
pipeline-copies-gap = Der Abstand muss 0 oder mehr sein, ist { $got }

## Errors: the simulation

# $min is a length without its unit, $max and $got with it.
pipeline-simulate-cell-size = die Zellgröße muss zwischen { $min } und { $max } liegen, ist { $got }
# $cell is the size of one cell, a length.
pipeline-simulate-too-many-cells = eine Zelle von { $cell } ergibt { $cells } Zellen für dieses Rohteil, mehr als { $most }; verwenden Sie eine größere Zelle
pipeline-simulate-stock-too-large = das Rohteil ist zu groß, um es mit Zellen von { $cell } zu simulieren (mehr als { $most } Zellen)
# $error is the PNG encoder's own words.
pipeline-simulate-png = PNG: { $error }

## Post-processors

pipeline-post-no-settings = kein Einstellungsordner, also kein Postprozessor „{ $name }“
pipeline-post-no-folder = Kein Postprozessor „{ $name }“: er ist nicht eingebaut, und es gibt keinen Ordner posts
# Why a project's copy of a user post can't be compared with its file.
pipeline-post-no-posts-folder = kein Ordner posts
# What a post-processor is, one line each (Settings, This computer). $name, $extension,
# the paths and the units (mm, inch) and modes (files, program) are written as in its file.
pipeline-post-name = Name: { $name }
pipeline-post-builtin = Eingebaut
pipeline-post-yours = Eigener
pipeline-post-settings = Einstellungen: { $path }
pipeline-post-template = Vorlage: { $path }
pipeline-post-extension = Dateiendung: { $extension }
pipeline-post-decimals = Nachkommastellen: { $mm } (mm), { $inch } (inch)
pipeline-post-units-list = G-Code-Einheiten: { $units }
pipeline-post-arcs = Bögen: { $arcs ->
    [arcs] G2/G3
   *[lines] als kurze Linien (G1)
}
pipeline-post-tool-changes = Mehrere Werkzeuge: { $modes }
# $seconds is a time in seconds, without its unit.
pipeline-post-dwell = Verweilzeit der Spindel: { $seconds } s
pipeline-post-no-dwell = Verweilzeit der Spindel: keine

## The tool library and machine profiles

# $error is why the library file couldn't be read (a job-library-error text).
pipeline-library-defaults = { $error } (eingebaute Standardwerte werden verwendet)
pipeline-library-no-profile = kein Maschinenprofil namens „{ $name }“
# What a new project started with. $how is "profile" (asked for) or "default".
pipeline-library-machine = Maschine: { $name } ({ $how ->
    [default] Standardprofil
   *[profile] Profil
})
pipeline-library-tool = Werkzeug { $tool }: { $name } (Standardwerkzeug)
# $error is why the post can't be used (a post-error text).
pipeline-library-post-unusable = Postprozessor „{ $name }“: { $error }
pipeline-library-unknown-post = Maschine „{ $name }“: dieses Chipload hat keinen Postprozessor „{ $post }“; Projekte, die ihn verwenden, können erst G-Code erzeugen, wenn es ihn hat

## Recovery copies

# $reason is the operating system's words, or the text below.
pipeline-recovery-io = Die Wiederherstellungskopie { $path } kann nicht geschrieben werden: { $reason }
pipeline-recovery-locked = bereits gesperrt

## Dragging a picture's or text's box

pipeline-box-turn-centre = der Drehgriff liegt auf der Mitte
pipeline-box-no-size = der Rahmen hat keine Größe
# $min is the smallest size, a length.
pipeline-box-too-small = er kann nicht kleiner als { $min } sein
pipeline-box-no-corner = ein Rahmen hat keine Ecke { $corner }

## G-code files

# One of several G-code files: $name is its file name, $operations the numbers of the
# operations it cuts ("1, 3").
pipeline-gcode-file = { $name } (Werkzeug { $tool }, Bearbeitungen { $operations })

## Explode a DXF, join close ends (docs/specs/explode-and-join.md)

pipeline-dxf-exploded = { $kind } { $id } „{ $name }“ in { $count } gezeichnete { $count ->
    [one] Form
   *[other] Formen
} aufgelöst. Das Projekt kennt die { $kind }-Datei nicht mehr.
# $shapes shape numbers ("S3, S4"); $tolerance a length.
pipeline-explode-split = { $count ->
    [one] Form { $shapes } bestand aus Stücken, die sich nicht treffen: Jedes Stück ist jetzt eine Form.
   *[other] Formen { $shapes } bestanden aus Stücken, die sich nicht treffen: Jedes Stück ist jetzt eine Form.
}
pipeline-explode-circle-start = { $count ->
    [one] Kreis { $shapes } beginnt jetzt an seinem rechten Punkt, gegen den Uhrzeigersinn, wie gezeichnete Kreise: Sein Schnitt kann woanders auf ihm beginnen.
   *[other] Kreise { $shapes } beginnen jetzt an ihren rechten Punkten, gegen den Uhrzeigersinn, wie gezeichnete Kreise: Ihre Schnitte können woanders auf ihnen beginnen.
}
pipeline-explode-dropped = { $count ->
    [one] Form { $shapes } hatte nur Segmente kürzer als { $tolerance } und ist entfernt.
   *[other] Formen { $shapes } hatten nur Segmente kürzer als { $tolerance } und sind entfernt.
}
# $label is an operation's label.
pipeline-explode-selects = { $label } wählt jetzt die Formen { $shapes }.
pipeline-explode-no-dxf = das Projekt hat keine DXF zum Auflösen
pipeline-explode-which-dxf = das Projekt hat { $count } DXFs ({ $dxfs }): Geben Sie an, welche aufgelöst werden soll
pipeline-join-gap-range = Der Abstand muss { $least } bis { $most } sein, war { $got }.
pipeline-join-imported = Form S{ $id } stammt aus einer DXF: Lösen Sie zuerst die DXF auf.
pipeline-join-none = Keine Enden verbunden (Abstand { $gap }).
pipeline-joined = { $pieces } Formen zu { $count } verbunden, davon { $closed } geschlossen; größte geschlossene Lücke { $widest }.
pipeline-joined-touching = { $pieces } Formen zu { $count } verbunden, davon { $closed } geschlossen; ihre Enden trafen sich.
pipeline-join-open = { $count } { $count ->
    [one] Ende bleibt
   *[other] Enden bleiben
} offen.
# $at a point, $nearest a length.
pipeline-join-open-end = Das Ende bei { $at }: Das nächste andere Ende ist { $nearest } entfernt.
pipeline-join-open-alone = Das Ende bei { $at }: kein anderes Ende auf seinem Layer.
pipeline-join-branch = Drei oder mehr Enden treffen sich bei { $at }: dort nicht verbunden.
pipeline-edit-imported = S{ $id } stammt aus einer DXF, einem Bild oder einem Text: die DXF auflösen oder die Form zeichnen, um sie zu ändern
pipeline-offset-distance = der Versatz muss größer als 0 sein, erhalten { $got }
pipeline-offset-vanishes = von S{ $id } bleibt { $distance } innen nichts übrig
pipeline-offset-side = S{ $id } ist { $closed ->
    [closed] geschlossen: nach außen oder innen versetzen
   *[open] offen: nach links oder rechts versetzen
}
pipeline-offset-failed = S{ $id } ließ sich nicht versetzen: { $why }
pipeline-new-shapes-cut = { $count ->
    [one] Die neue Form liegt
   *[other] Die { $count } neuen Formen liegen
} auf dem Layer { $layer }, den { $ops ->
    [one] Bearbeitung { $operations } schneidet
   *[other] die Bearbeitungen { $operations } schneiden
}.
pipeline-corner-none = S{ $id } hat keine Ecken zum Runden oder Fasen
pipeline-corner-not = Ecke { $corner } von S{ $id } liegt nicht zwischen zwei geraden Seiten
pipeline-corner-straight = S{ $id } läuft bei { $corner } gerade weiter: dort ist keine Ecke
pipeline-corner-too-large = das ist zu groß für die Seiten von S{ $id }: höchstens { $most }
pipeline-corner-size = Radius oder Abstand müssen größer als 0 sein
pipeline-trim-nothing-crosses = dort kreuzt nichts S{ $id } (eine geschlossene Form braucht zwei Kreuzungen): stattdessen löschen
pipeline-trim-everything = S{ $id } dort zu stutzen ließe nichts übrig: stattdessen löschen
pipeline-extend-not-straight = nur ein gerades Ende von S{ $id } lässt sich verlängern
pipeline-extend-nothing-ahead = vor diesem Ende von S{ $id } liegt nichts
pipeline-extend-closed = S{ $id } ist geschlossen: es hat kein Ende zum Verlängern
pipeline-break-at-end = das ist ein Ende von S{ $id }: dort gibt es nichts zu brechen
pipeline-combine-two = zum Kombinieren braucht es zwei Formen oder mehr
pipeline-combine-open = S{ $id } ist offen: nur geschlossene Formen lassen sich kombinieren
pipeline-combine-empty = { $op ->
    [subtract] von S{ $id } bleibt nichts übrig: die anderen Formen decken sie ganz ab
   *[other] die Formen haben keinen gemeinsamen Teil: es bleibt nichts übrig
}
pipeline-combine-failed = die Formen ließen sich nicht kombinieren: { $why }
pipeline-array-count = eine Anordnung hat 2 bis { $most } Formen samt dem Original, ist { $got }
pipeline-array-spacing = die Kopien lägen aufeinander: geben Sie einen Abstand ungleich 0 an, wo es mehr als eine Spalte oder Reihe gibt
pipeline-array-angle = der Winkel muss größer als 0 und höchstens 360 Grad sein, ist { $got }
pipeline-align-two = zum Ausrichten braucht es zwei Formen oder mehr, oder das Rohteil als Bezug
pipeline-distribute-three = zum Verteilen braucht es drei Formen oder mehr
pipeline-shape-gone = S{ $id } gibt es nicht mehr: { $ops ->
    [one] Operation { $operations } hat sie verwendet; wählen Sie ihre Geometrie neu
   *[other] die Operationen { $operations } haben sie verwendet; wählen Sie ihre Geometrie neu
}.

## DXF export (docs/specs/dxf-export.md)

pipeline-export-nothing = es gibt nichts zu exportieren: dort werden keine Formen gezeigt
pipeline-export-no-layer = die Zeichnung hat keinen Layer { $layer }
pipeline-export-unverified = die DXF wurde nicht geschrieben: S{ $id } kam beim Zurücklesen nicht gleich zurück ({ $why })
pipeline-export-summary = { $count ->
    [one] 1 Objekt
   *[other] { $count } Objekte
} auf { $layers ->
    [one] 1 Layer
   *[other] { $layers } Layern
}, in { $units ->
    [inch] Zoll
   *[mm] mm
}.
pipeline-export-renamed = Layer { $from } heißt in der DXF { $to }: ein DXF-Layername kann einige seiner Zeichen nicht enthalten.
