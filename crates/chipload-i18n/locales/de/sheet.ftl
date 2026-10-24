# The setup sheet for the operator (crates/chipload-pipeline/src/sheet.rs,
# docs/specs/setup-sheet.md): an HTML page printed next to the machine. A length argument
# brings its own unit ("6 mm", "0.25 in"); $start, $end, $x, $y, $width, $step, $radius and
# $feed are numbers alone, in the sheet's units, as are $z and $peck where said. Names
# (project, tools, parts, operations, layers, files) are never translated.

sheet-title = Einrichteblatt: { $project }
# The line under the title, its parts joined with " · ".
sheet-post = Postprozessor { $post }
sheet-written-by = erstellt von { $by }
# $length is "mm" or "in", $feed "mm/min" or "in/min".
sheet-units = Längen in { $length }, Vorschübe in { $feed }
sheet-gcode-inch = G-Code in Zoll (G20)

## Stock and zero
sheet-stock-and-zero = Rohteil und Nullpunkt
sheet-stock = Rohteil
# This text is HTML: keep &times; (the × sign). $width is a number alone.
sheet-stock-size = { $width } &times; { $length }, { $thickness } dick
sheet-copies = Kopien
sheet-part = Teil
sheet-dxf = DXF { $id }
# $placement says how the drawing was moved, turned and scaled.
sheet-dxf-placed = { $name } ({ $file }), { $placement }
sheet-image = Bild { $id }
sheet-image-placed = { $file } auf Layer { $layer }, { $width } breit bei X{ $x } Y{ $y }
sheet-text = Text { $id }
# $text is the text's first words in quotes, $font its font.
sheet-text-placed = { $text } in { $font } auf Layer { $layer }, { $height } hoch bei X{ $x } Y{ $y }

## Where X0 Y0 is, said on the stock
sheet-zero-centre = die Mitte des Rohteils
sheet-zero-bottom-left = die untere linke Ecke des Rohteils
sheet-zero-bottom-right = die untere rechte Ecke des Rohteils
sheet-zero-top-left = die obere linke Ecke des Rohteils
sheet-zero-top-right = die obere rechte Ecke des Rohteils
# A zero away from the stock's bottom left corner: "25 mm right of and 10 mm above the
# stock's bottom left corner". $offset is one or two of the four below, joined with -and.
sheet-zero-from-corner = { $offset } der unteren linken Ecke des Rohteils
sheet-zero-right-of = { $distance } rechts von
sheet-zero-left-of = { $distance } links von
sheet-zero-above = { $distance } oberhalb
sheet-zero-below = { $distance } unterhalb
sheet-zero-and = { $first } und { $second }
sheet-zero-outside = { $zero } (außerhalb des Rohteils)
# Where Z0 is.
sheet-z-top = die Rohteiloberkante
sheet-z-bottom = die Rohteilunterkante (die Opferplatte)

## A file's tools. $what is the tool: "6 mm end mill, 2 flutes".
# $xy is where X0 Y0 is, $z where Z0 is (texts above).
sheet-load-tool = Spannen Sie Werkzeug T{ $tool } ({ $what }) ein. X0 Y0: { $xy }. Z0: { $z }.
sheet-tool-change-m6 = Werkzeugwechsel (M6) auf T{ $tool } ({ $what }), so wie Ihre Steuerung ihn ausführt.
sheet-change-tool = Wechseln Sie auf Werkzeug T{ $tool } ({ $what }). Setzen Sie Z0 mit diesem Werkzeug neu, auf { $z }; X0 Y0 bleiben.
# These two texts are HTML: keep &ldquo; and &rdquo; (the quotes round the tool's name).
# $rpm is the spindle speed, $spindle one of the two -spindle- texts.
sheet-tool-named = Werkzeug T{ $tool }: &ldquo;{ $name }&rdquo;, { $what }, { $rpm } rpm { $spindle }.
sheet-tool = Werkzeug T{ $tool }: { $what }, { $rpm } rpm { $spindle }.
sheet-spindle-cw = im Uhrzeigersinn (M3)
sheet-spindle-ccw = gegen den Uhrzeigersinn (M4)
# The parts of $what, joined with ", ".
sheet-end-mill = { $diameter } Schaftfräser
sheet-drill = { $diameter } Bohrer
# $angle in degrees.
sheet-vbit-angle = { $angle }° V-Fräser, { $diameter }
sheet-vbit = V-Fräser, { $diameter }
sheet-tip = Spitze { $diameter }
sheet-flutes = { $count ->
    [one] { $count } Schneide
   *[other] { $count } Schneiden
}

## The table of operations
sheet-column-operation = Bearbeitung
sheet-column-depth = Tiefe
sheet-column-feeds = Vorschub / Eintauchen
sheet-column-entry = Eintauchen
sheet-column-tabs = Haltestege
sheet-column-time = Zeit
# A part's row over its operations, numbered from 1.
sheet-part-row = Teil { $number }: { $name }
sheet-part-row-copies = Teil { $number }: { $name }, { $count ->
    [one] { $count } Kopie
   *[other] { $count } Kopien
}
# An operation without a name.
sheet-operation = Bearbeitung { $number }
# A drill's one feed: $feed is a number alone.
sheet-feed-down = { $feed } abwärts
# The kind column, its parts joined with ", ": "Profile, outside, climb, leaves 0.2 mm".
sheet-kind-profile = Konturfräsen
sheet-side-outside = außen
sheet-side-inside = innen
sheet-side-on = auf der Linie
sheet-kind-pocket = Tasche, Ringe
sheet-climb = Gleichlauf
sheet-conventional = Gegenlauf
# $percent: the stepover in percent of the tool's diameter.
sheet-stepover = seitliche Zustellung { $percent } %
sheet-kind-vcarve = V-Gravur
sheet-kind-path = Pfad folgen, auf der Linie
sheet-kind-holes-drilled = Bohrungen, gebohrt
sheet-kind-holes-bored = Bohrungen, Helixbohren
sheet-kind-surface = Planfräsen, das ganze Rohteil, hin und her (Gleichlauf und Gegenlauf), seitliche Zustellung { $percent } %
# $surface is the text above; $distance half the tool's diameter.
sheet-kind-surface-past = { $surface }; fährt { $distance } über die Rohteilenden hinaus: Spannmittel freihalten
sheet-cuts-past = fräst { $distance } über die Linie hinaus
sheet-leaves = lässt { $distance } stehen
sheet-rest-after = Rest nach T{ $tool }
sheet-rest-after-size = Rest nach T{ $tool } ({ $diameter })
# $degrees: the sharpest corner angle that gets one.
sheet-dogbones = Dogbones an Ecken bis { $degrees }°
sheet-tbones = T-Bones an Ecken bis { $degrees }°
# The depth column, its parts joined with ", ": "0 to 6, 2 per pass".
sheet-depths = { $start } bis { $end }
sheet-depths-drill = { $start } bis { $end } (Bohrerspitze)
sheet-pecks = Entspanen alle { $peck }
sheet-one-go = in einem Zug
sheet-per-turn = { $step } pro Umdrehung
sheet-per-pass = { $step } pro Durchgang
sheet-one-pass = ein Durchgang
sheet-finish = Schlichten, Aufmaß { $stock }
sheet-skin = Restschicht { $thickness }
# The entry column.
sheet-entry-beside = neben dem Rohteil hinunter
sheet-entry-straight = senkrecht hinunter
sheet-entry-plunge = Eintauchen
sheet-entry-ramp = Rampe { $degrees }°
sheet-entry-helix = Helix { $degrees }°
sheet-entry-helix-radius = Helix r { $radius }, { $degrees }°
sheet-lead = Anfahrt r { $radius }
# The tabs column: $count tabs, $width and $height numbers alone.
sheet-tabs = { $count } × { $width } breit, { $height } hoch
sheet-flanks = Flanken { $degrees }°
sheet-tabs-none = keine
# Under the table: how deep the tool goes. $z is a number alone; the parts are joined with ", ".
sheet-cuts-down = Fräst bis Z{ $z }: { $depth } unter der Rohteiloberkante
sheet-into-spoilboard = { $depth } in die Opferplatte
sheet-through = durch das Rohteil
sheet-sticks-out = { $cut }. Prüfen Sie, ob das Werkzeug weit genug aus dem Spannfutter ragt.

## Watch out: the notes and the checker's findings
sheet-watch-out = Achtung
sheet-nothing-reported = Nichts gemeldet.
# $note is the post-processor's own note, as its file writes it.
sheet-post-note = Postprozessor { $post }: { $note }
sheet-check = Prüfung: { $finding }

## The picture's legend
sheet-legend-file = Datei { $number } (T{ $tool })
sheet-legend-tabs = Haltestege
sheet-legend-grey = Grau: die Teile. Nur Fräsbewegungen, keine Eilgänge.

## The time
sheet-time = Zeit
