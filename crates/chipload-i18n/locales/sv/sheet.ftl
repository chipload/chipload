# The setup sheet for the operator (crates/chipload-pipeline/src/sheet.rs,
# docs/specs/setup-sheet.md): an HTML page printed next to the machine. A length argument
# brings its own unit ("6 mm", "0.25 in"); $start, $end, $x, $y, $width, $step, $radius and
# $feed are numbers alone, in the sheet's units, as are $z and $peck where said. Names
# (project, tools, parts, operations, layers, files) are never translated.

sheet-title = Uppsättningsblad: { $project }
# The line under the title, its parts joined with " · ".
sheet-post = postprocessor { $post }
sheet-written-by = skrivet av { $by }
# $length is "mm" or "in", $feed "mm/min" or "in/min".
sheet-units = längder i { $length }, matningar i { $feed }
sheet-gcode-inch = G-kod i tum (G20)

## Stock and zero
sheet-stock-and-zero = Ämne och nollpunkt
sheet-stock = Ämne
# This text is HTML: keep &times; (the × sign). $width is a number alone.
sheet-stock-size = { $width } &times; { $length }, { $thickness } tjockt
sheet-copies = Kopior
sheet-part = Detalj
sheet-dxf = DXF { $id }
# $placement says how the drawing was moved, turned and scaled.
sheet-dxf-placed = { $name } ({ $file }), { $placement }
sheet-image = Bild { $id }
sheet-image-placed = { $file } på lager { $layer }, { $width } bred vid X{ $x } Y{ $y }
sheet-text = Text { $id }
# $text is the text's first words in quotes, $font its font.
sheet-text-placed = { $text } i { $font } på lager { $layer }, { $height } hög vid X{ $x } Y{ $y }

## Where X0 Y0 is, said on the stock
sheet-zero-centre = ämnets centrum
sheet-zero-bottom-left = ämnets nedre vänstra hörn
sheet-zero-bottom-right = ämnets nedre högra hörn
sheet-zero-top-left = ämnets övre vänstra hörn
sheet-zero-top-right = ämnets övre högra hörn
# A zero away from the stock's bottom left corner: "25 mm right of and 10 mm above the
# stock's bottom left corner". $offset is one or two of the four below, joined with -and.
sheet-zero-from-corner = { $offset } ämnets nedre vänstra hörn
sheet-zero-right-of = { $distance } till höger om
sheet-zero-left-of = { $distance } till vänster om
sheet-zero-above = { $distance } ovanför
sheet-zero-below = { $distance } nedanför
sheet-zero-and = { $first } och { $second }
sheet-zero-outside = { $zero } (utanför ämnet)
# Where Z0 is.
sheet-z-top = ämnets ovansida
sheet-z-bottom = ämnets undersida (offerskivan)

## A file's tools. $what is the tool: "6 mm end mill, 2 flutes".
# $xy is where X0 Y0 is, $z where Z0 is (texts above).
sheet-load-tool = Sätt i verktyg T{ $tool } ({ $what }). Ställ in X0 Y0 i { $xy } och Z0 på { $z }.
sheet-tool-change-m6 = Verktygsbyte (M6) till T{ $tool } ({ $what }), så som din styrning hanterar det.
sheet-change-tool = Byt till verktyg T{ $tool } ({ $what }). Ställ in Z0 igen med detta verktyg på { $z }; behåll X0 Y0.
# These two texts are HTML: keep &ldquo; and &rdquo; (the quotes round the tool's name).
# $rpm is the spindle speed, $spindle one of the two -spindle- texts.
sheet-tool-named = Verktyg T{ $tool }: &ldquo;{ $name }&rdquo;, { $what }, { $rpm } rpm { $spindle }.
sheet-tool = Verktyg T{ $tool }: { $what }, { $rpm } rpm { $spindle }.
sheet-spindle-cw = medurs (M3)
sheet-spindle-ccw = moturs (M4)
# The parts of $what, joined with ", ".
sheet-end-mill = { $diameter } pinnfräs
sheet-drill = { $diameter } borr
# $angle in degrees.
sheet-vbit-angle = { $angle }° V-fräs, { $diameter }
sheet-vbit = V-fräs, { $diameter }
sheet-tip = spets { $diameter }
sheet-flutes = { $count } skär

## The table of operations
sheet-column-operation = Operation
sheet-column-depth = Djup
sheet-column-feeds = Matning / nedmatning
sheet-column-entry = Nedgång
sheet-column-tabs = Bryggor
sheet-column-time = Tid
# A part's row over its operations, numbered from 1.
sheet-part-row = Detalj { $number }: { $name }
sheet-part-row-copies = { $count ->
    [one] Detalj { $number }: { $name }, { $count } kopia
   *[other] Detalj { $number }: { $name }, { $count } kopior
}
# An operation without a name.
sheet-operation = Operation { $number }
# A drill's one feed: $feed is a number alone.
sheet-feed-down = { $feed } nedåt
# The kind column, its parts joined with ", ": "Profile, outside, climb, leaves 0.2 mm".
sheet-kind-profile = Konturfräsning
sheet-side-outside = utanför
sheet-side-inside = inuti
sheet-side-on = på linjen
sheet-kind-pocket = Ficka, slingor
sheet-climb = medfräsning
sheet-conventional = motfräsning
# $percent: the stepover in percent of the tool's diameter.
sheet-stepover = sidosteg { $percent } %
sheet-kind-vcarve = V-gravyr
sheet-kind-path = Följ bana, på linjen
sheet-kind-holes-drilled = Hål, borrade
sheet-kind-holes-bored = Hål, helixborrning
sheet-kind-surface = Planfräsning, hela ämnet, fram och tillbaka (med- och motfräsning), sidosteg { $percent } %
# $surface is the text above; $distance half the tool's diameter.
sheet-kind-surface-past = { $surface }; går { $distance } förbi ämnets kanter: håll fästen ur vägen
sheet-cuts-past = fräser { $distance } förbi linjen
sheet-leaves = lämnar { $distance }
sheet-rest-after = rest efter T{ $tool }
sheet-rest-after-size = rest efter T{ $tool } ({ $diameter })
# $degrees: the sharpest corner angle that gets one.
sheet-dogbones = dogbones i hörn på högst { $degrees }°
sheet-tbones = T-bones i hörn på högst { $degrees }°
sheet-corner-choices = { $count ->
    [one] { $count } hörn valt för sig
   *[other] { $count } hörn valda ett och ett
}
# The depth column, its parts joined with ", ": "0 to 6, 2 per pass".
sheet-depths = { $start } till { $end }
sheet-depths-drill = { $start } till { $end } (borrspetsen)
sheet-pecks = stegborrning { $peck }
sheet-one-go = i ett tag
sheet-per-turn = { $step } per varv
sheet-per-pass = skärdjup { $step }
sheet-one-pass = ett skär
sheet-finish = finskär { $stock }
sheet-skin = skikt { $thickness }
# The entry column.
sheet-entry-beside = ned bredvid ämnet
sheet-entry-straight = rakt ned
sheet-entry-plunge = nedmatning
sheet-entry-ramp = ramp { $degrees }°
sheet-entry-helix = helix { $degrees }°
sheet-entry-helix-radius = helix r { $radius }, { $degrees }°
sheet-lead = inkörning r { $radius }
# The tabs column: $count tabs, $width and $height numbers alone.
sheet-tabs = { $count ->
    [one] { $count } × { $width } bred, { $height } hög
   *[other] { $count } × { $width } breda, { $height } höga
}
sheet-flanks = flanker { $degrees }°
sheet-tabs-none = inga
# Under the table: how deep the tool goes. $z is a number alone; the parts are joined with ", ".
sheet-cuts-down = Fräser ned till Z{ $z }: { $depth } under ämnets ovansida
sheet-into-spoilboard = { $depth } ned i offerskivan
sheet-through = igenom ämnet
sheet-sticks-out = { $cut }. Kontrollera att verktyget sticker ut tillräckligt långt.

## Watch out: the notes and the checker's findings
sheet-watch-out = Se upp
sheet-nothing-reported = Inget rapporterades.
# $note is the post-processor's own note, as its file writes it.
sheet-post-note = Postprocessor { $post }: { $note }
sheet-check = Kontroll: { $finding }

## The picture's legend
sheet-legend-file = Fil { $number } (T{ $tool })
sheet-legend-tabs = bryggor
sheet-legend-grey = Grått: detaljerna. Bara skärande rörelser, inga snabbförflyttningar.

## The time
sheet-time = Tid
