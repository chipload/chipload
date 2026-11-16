# The setup sheet for the operator (crates/chipload-pipeline/src/sheet.rs,
# docs/specs/setup-sheet.md): an HTML page printed next to the machine. A length argument
# brings its own unit ("6 mm", "0.25 in"); $start, $end, $x, $y, $width, $step, $radius and
# $feed are numbers alone, in the sheet's units, as are $z and $peck where said. Names
# (project, tools, parts, operations, layers, files) are never translated.

sheet-title = Uppsättningsblad: { $project }
# Only some of the project's parts were exported.
sheet-title-parts = Uppsättningsblad: { $project }, { $count ->
    [one] detalj
   *[other] detaljer
} { $parts }
# The line under the title, its parts joined with " · ".
# With several setups, after the title: which one this sheet is for.
sheet-setup = uppspänning ”{ $name }”
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
sheet-back-side = Baksida
# $back is the setup that cuts the board's back.
sheet-back-side-value = { $back } fräser baksidan när skivan är vänd. Låt skivan ligga kvar tills den här uppspänningen är klar.
sheet-turn-heading = Vänd skivan
sheet-turn-left-right = Vänd skivan från vänster till höger, kring dess lodräta mittlinje: vänsterkanten hamnar till höger, och sidan som låg nedåt är nu uppåt.
sheet-turn-top-bottom = Vänd skivan nerifrån och upp, kring dess vågräta mittlinje: nederkanten hamnar upptill, och sidan som låg nedåt är nu uppåt.
sheet-turn-pins = Sätt en styrpinne på { $diameter } genom varje styrhål ner i offerskivan.
sheet-turn-pins-down = Slå ner pinnarna minst { $depth } under baksidan: baksidan planfräses över dem.
sheet-mirrored = Spegelvänd
sheet-mirrored-left-right = { $part } fräses spegelvänd från vänster till höger, kring mittlinjen av dess former: ritningens spegelbild.
sheet-mirrored-top-bottom = { $part } fräses spegelvänd uppifrån och ner, kring mittlinjen av dess former: ritningens spegelbild.
sheet-inlay = Inläggning
sheet-inlay-steps = När den är fräst: limma pluggen med framsidan ned i fickan i uppspänning { $setup } ({ $name }), tvinga eller pressa den tills den sitter på sina väggar, låt limmet härda, såga eller hyvla sedan bort pluggskivans baksida ned till grundskivans yta.
sheet-inlay-steps-no-pocket = När den är fräst: limma pluggen med framsidan ned i sin ficka, tvinga eller pressa den tills den sitter på sina väggar, låt limmet härda, såga eller hyvla sedan bort pluggskivans baksida ned till grundskivans yta.
sheet-turn-no-dowels = { $front } har inga styrhål för pinnarna: lägg till dem innan den fräses, eller lägg skivan mot ett hörn.
sheet-turn-corner = Skjut in skivan i samma hörn av anslaget som för { $front }. Den måste vara exakt { $width } × { $length }.
sheet-turn-picture = Bilden visar skivan som den ligger på maskinen, baksidan upp.
sheet-copies = Instanser
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
sheet-corner-radius = hörnradie { $radius }
sheet-drill = { $diameter } borr
# $angle in degrees.
sheet-vbit-angle = { $angle }° V-fräs, { $diameter }
sheet-vbit = V-fräs, { $diameter }
sheet-tip = spets { $diameter }
sheet-ball-nose = { $diameter } kulfräs
sheet-tapered-ball = { $angle }° konisk kulfräs, { $diameter }
sheet-ball = kula { $diameter }
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
sheet-part-row-copies = Detalj { $number }: { $name }, { $count ->
    [one] fräses en gång
   *[other] fräses { $count } gånger
}
# An operation without a name.
sheet-operation = Operation { $number }
# A drill's one feed: $feed is a number alone.
sheet-feed-down = { $feed } nedåt
# The kind column, its parts joined with ", ": "Profile, outside, climb, leaves 0.2 mm".
sheet-kind-profile = Konturfräsning
sheet-side-outside = utsida
sheet-side-inside = insida
sheet-side-on = på linjen
sheet-kind-pocket = Ficka, slingor
sheet-climb = medfräsning
sheet-conventional = motfräsning
# $percent: the stepover in percent of the tool's diameter.
sheet-stepover = sidosteg { $percent } %
sheet-kind-vcarve = V-gravyr
sheet-kind-path = Följ bana, på linjen
sheet-kind-chamfer = Fas { $width } bred
sheet-kind-flute = Räffla, ramper { $start } / { $end }
sheet-ramp-straight = raka ramper
sheet-ramp-curved = böjda ramper
sheet-pattern-lines = Mönster, linjer { $spacing } isär i { $angle }°
sheet-pattern-cross = Mönster, korsrutor { $spacing } isär i { $angle }°, { $between }° emellan
sheet-pattern-rings = Mönster, ringar { $spacing } isär
sheet-pattern-waves = Mönster, vågor { $spacing } isär i { $angle }°, { $wavelength } långa, { $height } höga
sheet-pattern-dashes = Mönster, streck { $length } långa med { $gap } mellanrum, rader { $spacing } isär i { $angle }°
sheet-pattern-herringbone = Mönster, fiskben av { $length }-streck { $spacing } isär i { $angle }°
sheet-pattern-scales = Mönster, fjäll med radien { $radius }, rader { $spacing } isär i { $angle }°
sheet-pattern-sunburst = Mönster, solfjäder av { $length }-streck, ringar { $spacing } isär, { $pitch } runt om
sheet-pattern-inside = innanför kanten
sheet-pattern-on-line = på linjen
sheet-pattern-ramp = ändar med ramp { $ramp }
sheet-kind-prism = Prisma
sheet-moulding-cove = Profil, hålkäl R { $radius }
sheet-moulding-roundover = Profil, rundning R { $radius }
sheet-moulding-ogee = Profil, karnis { $width } × { $depth }
sheet-moulding-bead = Profil, rundstav R { $radius }
sheet-moulding-flute = Profil, räffla R { $radius }
sheet-moulding-shape = Profil efter S{ $shape }
sheet-moulding-left = till vänster om banan
sheet-moulding-right = till höger om banan
sheet-moulding-round-ends = runda ändar
sheet-moulding-tapered-ends = ändar som klingar av över { $length }
sheet-stepover-mm = stegning { $stepover }
sheet-kind-holes-drilled = Hål, borrade
sheet-kind-holes-bored = Hål, helixborrning
sheet-kind-surface = Planfräsning, hela ämnet, fram och tillbaka (med- och motfräsning), sidosteg { $percent } %
# $surface is the text above; $distance half the tool's diameter.
sheet-kind-surface-past = { $surface }; går { $distance } förbi ämnets kanter: håll fästen ur vägen
sheet-kind-relief-zigzag = Relief av modell { $model }, banor i { $angle }° fram och tillbaka, sidosteg { $percent } %
sheet-kind-relief-one-way = Relief av modell { $model }, banor i { $angle }° alla åt samma håll, sidosteg { $percent } %
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
sheet-depths-at-most = { $start } till som mest { $end }
sheet-depths-drill = { $start } till { $end } (borrspetsen)
sheet-pecks = stegborrning { $peck }
sheet-one-go = i ett tag
sheet-per-turn = { $step } per varv
sheet-per-pass = skärdjup { $step }
sheet-groove-width = { $width } brett upptill
sheet-one-pass = ett skär
sheet-passes = { $count } skär
sheet-finish = finskär { $stock }
sheet-skin = skikt { $thickness }
sheet-pocket-finish = finbearbetning: väggar { $wall }, botten { $floor }
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
# Where the tabs are, when not evenly round the contour.
sheet-tabs-along-x = längs X (nedre och övre kant)
sheet-tabs-along-y = längs Y (vänster och höger kant)
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
sheet-pieces = Bitar
sheet-pieces-summary = { $count } bitar, { $columns } × { $rows }, sammanfogade med { $joint ->
    [knobs] pusselknoppar
   *[fingers] tappar
} på { $size }. Varje bit fräses ur en egen skiva, nedan.
sheet-piece = Bit { $name }
sheet-piece-mark = Skriv { $name } på baksidan med blyerts innan du spänner loss den.
sheet-pieces-glue-title = Limning
sheet-pieces-glue = Provpassa bitarna först: fogarna är { $clearance } lösa för limmet. Limma bitarna i varje rad, och sedan raderna ihop.
