# Messages of the app services (crates/chipload-pipeline): problems found with the
# post-processor a project uses.

# $post is the post-processor's name, $units the G-code units the machine is set to, as
# written in the project file ("inch"). "units" is a key of the post's config file.
pipeline-post-units = postprocessorn { $post } skriver bara G-kod i mm (dess konfiguration har inte "{ $units }" under units)

## Planning notes

pipeline-drawing-outside-stock = Ritningen sträcker sig utanför ämnet
# $count is the number of copies. $x0 and $y0 are numbers without their unit; $x1, $y1
# and $stock_y bring it; $stock_x is a number without it.
pipeline-copies-beyond-stock = De { $count } kopiorna fräser X{ $x0 } till { $x1 }, Y{ $y0 } till { $y1 }, utanför ämnet ({ $stock_x } × { $stock_y }); kontrollera ämnet eller detaljens kopior.
# As pipeline-copies-beyond-stock, for one part of several; $part is its name.
pipeline-part-copies-beyond-stock = Detalj { $part }: de { $count } kopiorna fräser X{ $x0 } till { $x1 }, Y{ $y0 } till { $y1 }, utanför ämnet ({ $stock_x } × { $stock_y }); kontrollera ämnet eller detaljens kopior.
# $label is an operation's label, $layer a layer's name, $sources where the layer's shapes
# come from (DXF and picture names, joined with pipeline-and).
pipeline-layer-clash = { $label }: lagret "{ $layer }" finns i { $sources }; operationen fräser alla. Begränsa den till ett (markera lagret under den DXF:en i bläddraren)
# Names joined: "a.dxf and b.dxf".
pipeline-and = { $list } och { $last }
# Where shapes come from: a DXF's or picture's name, or one without a name.
pipeline-source-named = { $name }
pipeline-source-dxf = DXF { $id }
pipeline-source-image = bild { $id }
pipeline-source-drawn = ritade former

## The time estimate (after planning and generating)

# A duration: "45 s", "12 min 30 s", "1 h 05 min". $minutes after hours has two digits.
pipeline-duration-seconds = { $seconds } s
pipeline-duration-minutes = { $minutes } min { $seconds } s
pipeline-duration-hours = { $hours } h { $minutes } min
# The durations are pipeline-duration-… texts; $changes is the number of tool changes.
pipeline-estimate = Uppskattad tid: { $total } (fräsning { $cutting }, snabbförflyttningar { $rapids }, spindel { $spindle }){ $changes ->
    [0] {""}
    [one] , plus { $changes } verktygsbyte
   *[other] , plus { $changes } verktygsbyten
}
# The lines under it start with spaces, written {"  "}: keep them, they indent the line.
# $name is the part's name.
pipeline-estimate-part = {"  "}Detalj { $number } ({ $name }): { $time }
# $label is an operation's label.
pipeline-estimate-operation = {"  "}{ $label }: { $time }
# An operation under its part: indented deeper.
pipeline-estimate-part-operation = {"    "}{ $label }: { $time }
# One G-code file of a project written one file per tool.
pipeline-estimate-file = {"  "}Fil { $number }: { $time }
# $rate is a feed ("5000 mm/min").
pipeline-estimate-rapids-assumed = {"  "}Vid de programmerade matningarna (snabbförflyttningar med { $rate }, antaget: ange maskinens maxmatning för en bättre uppskattning); acceleration och hörn gör den verkliga tiden längre.
pipeline-estimate-rapids = {"  "}Vid de programmerade matningarna (snabbförflyttningar med maskinens maxmatning, { $rate }); acceleration och hörn gör den verkliga tiden längre.

## Chip load

pipeline-chip-load-title = Matning per tand (vid skärmatningen):
# $label is an operation's label, $load a pipeline-chip-load… text.
pipeline-chip-load-line = {"  "}{ $label }: { $load }
# $load brings its unit ("0.042 mm/tooth").
pipeline-chip-load = { $load }
pipeline-chip-load-no-flutes = ange verktygets antal skär för att se matningen per tand
pipeline-chip-load-no-rpm = ange verktygets spindelvarvtal för att se matningen per tand
pipeline-chip-load-no-tool = inget verktyg
# The hover text next to an operation's chip load. Tool diameters $small_tool (a number)
# and $large_tool (with its unit); chip loads $wood_low, $aluminium_low and
# $aluminium_high (numbers) and $wood_high (with its unit).
pipeline-chip-load-hint = Matning per tand: hur tjock spån varje skär tar, matning / (spindelvarvtal × antal skär). För tunn spån och verktyget gnider och blir varmt; för tjock och det kan gå av. Den beror på materialet, verktyget och maskinen; som en grov riktlinje tar trä och plast med { $small_tool }–{ $large_tool } verktyg ofta { $wood_low }–{ $wood_high }, aluminium { $aluminium_low }–{ $aluminium_high }. Kontrollera verktygstillverkarens värden.

## The drawing's facts (after importing)

# $units is "mm" or "inch", as the DXF gives them or the user chose them.
pipeline-report-units = Enheter: { $units } ({ $from ->
    [dxf] från DXF:en
   *[user] valda av dig
})
# Numbers without their unit, except $height, which brings it.
pipeline-report-extents = Utsträckning: X { $x0 } .. { $x1 }, Y { $y0 } .. { $y1 } ({ $width } x { $height })
# $items lists DXF entity types with their counts ("LINE 4, ARC 2").
pipeline-report-imported = Importerat: { $items }
pipeline-report-flattened = Plattat till 2D: { $items }
pipeline-report-ignored = Ignorerat: { $items }
pipeline-report-no-issues = Anmärkningar: inga
pipeline-report-issues = Anmärkningar ({ $count }):
# One issue of the import (a dxf-… text), indented.
pipeline-report-issue = {"  "}[{ $severity ->
    [warning] varning
   *[error] fel
}] { $issue }
pipeline-report-contours = Konturer ({ $count }):
# A row of the contour table, in a fixed-width font: $id, $state and $layers are padded
# with spaces to line up. The coordinates are numbers without their unit.
pipeline-report-contour = {"  "}#{ $id } { $state } lager { $layers } X { $x0 } .. { $x1 }  Y { $y0 } .. { $y1 }
# A contour's state in the table: keep it short.
pipeline-report-contour-state = { $state ->
    [open] öppen
    [invalid] OGILTIG
    [hole] hål
   *[outer] yttre
}

## The G-code checker's findings (check-… texts), placed in the program

# A finding on a line of the program; $message is the finding.
pipeline-check-line = rad { $line }: { $message }
# A finding in one of several G-code files: $file is the file's number.
pipeline-check-file = fil { $file }: { $message }
# A finding on a line of one of several G-code files.
pipeline-check-file-line = fil { $file }, rad { $line }: { $message }

## Errors: a change or run the project refuses

# The independent checker refused the program; each finding follows on its own line.
pipeline-error-check-failed = G-kodkontrollen underkände programmet:
# One more finding (a pipeline-check-… text) on a line of its own, indented, after $list.
pipeline-error-check-finding =
    { $list }
    {"  "}{ $finding }
# Words of the command line or another library, kept as they are.
pipeline-text = { $text }

## Reports of changes to the drawing

# $x0 and $y0 are lengths without their unit, $x1 and $y1 with it; $width and $length the
# stock's size, the first without its unit.
pipeline-stock-fit = Ritningen (X{ $x0 } till { $x1 }, Y{ $y0 } till { $y1 }) sträcker sig utanför ämnet ({ $width } × { $length }); kontrollera ämnet under Jobbinställningar.

## Placing a DXF: moving, turning and scaling it

# $count is how many DXFs the project has, $dxfs their numbers and names ("1 part-a, 2 part-b").
pipeline-place-which-dxf = projektet har { $count } DXF:er ({ $dxfs }): ange vilken som ska placeras
pipeline-no-dxf = det finns ingen DXF { $id }
pipeline-place-nothing-imported = projektet har ingen importerad ritning att flytta
# What is placed, when the project has a single DXF (otherwise it is the DXF's name).
pipeline-place-the-drawing = den importerade ritningen
# $dx and $dy are numbers as typed, without a unit.
pipeline-place-cannot-move = kan inte flytta ritningen med ({ $dx }, { $dy })
pipeline-place-already-there = Ritningen är redan där; inget flyttades.
# $what is pipeline-place-the-drawing or a DXF's name; $degrees without its unit.
pipeline-place-turned = Vred { $what } { $degrees }° { $direction ->
    [ccw] moturs
   *[cw] medurs
}
pipeline-place-reset = Satte tillbaka { $what } där DXF:en har den
# $factor is a scale factor ("2", "0.5").
pipeline-place-scaled = Skalade { $what } ×{ $factor }
# $scaled is pipeline-place-scaled; $width is a length without its unit, $height with it.
pipeline-place-scaled-size = { $scaled } (nu { $width } × { $height })
# $dx is a length without its unit, $dy with it.
pipeline-place-moved = Flyttade { $what } med X{ $dx } Y{ $dy }
# $done is one of the texts above; $placement a pipeline-placement text.
pipeline-place-done = { $done }; den är nu { $placement }.
pipeline-place-moved-starts = { $count ->
    [one] Flyttade { $count } startpunkt med den.
   *[other] Flyttade { $count } startpunkter med den.
}
pipeline-place-moved-corners = Flyttade { $count } hörnval med den.
pipeline-place-moved-tabs = { $count ->
    [one] Flyttade { $count } handplacerad brygga med den.
   *[other] Flyttade { $count } handplacerade bryggor med den.
}
pipeline-place-drawn-stay = Former ritade i Chipload ligger kvar där de är.
pipeline-place-other-dxfs-stay = De andra DXF:erna ligger kvar där de är.
pipeline-place-work-zero-stays = Arbetsnollpunkten ligger kvar i punkten X{ $x }, Y{ $y }; kontrollera att den fortfarande är där du vill ha den.
pipeline-place-margin = marginalen måste vara 0 eller mer, fick { $got }
# $got is a number as typed, in degrees, without a unit.
pipeline-place-rotation = vridningen måste vara ett antal grader, fick { $got }
pipeline-place-scale = skalan måste vara ett tal större än 0, fick { $got }
# Scale factors: the result, and the smallest and largest allowed.
pipeline-place-scale-range = det skulle skala ritningen ×{ $factor } från DXF:en; den måste hålla sig mellan ×{ $min } och ×{ $max }
# $what is "width" or "height"; $got a length without its unit.
pipeline-place-size = { $what ->
    [height] höjden
   *[width] bredden
} måste vara större än 0, fick { $got }
pipeline-place-no-size = den importerade ritningen har ingen { $what ->
    [height] höjd
   *[width] bredd
} att skala
# How a DXF is placed: "scaled ×2, turned 90°, moved X12.000 Y-3.500 from the DXF".
pipeline-placement-as-dxf = som i DXF:en
pipeline-placement-from-dxf = { $parts } från DXF:en
pipeline-placement-and = { $list }, { $last }
pipeline-placement-scaled = skalad ×{ $factor }
pipeline-placement-turned = vriden { $degrees }°
# $dx and $dy are lengths without their unit.
pipeline-placement-moved = flyttad X{ $dx } Y{ $dy }
# After scaling: $within and $off are lengths, $factor the scale factor.
pipeline-scale-curves = { $count ->
    [one] { $count } form från splines eller ellipser approximerades
   *[other] { $count } former från splines eller ellipser approximerades
} inom { $within } i den tidigare storleken; skalade ×{ $factor } kan de avvika upp till { $off } från sina kurvor. Uppdatera från DXF:en för att approximera dem igen i den här storleken.
pipeline-scale-contours = Slutna konturer: { $was } före, { $now } nu; glapp sammanfogas inom { $within } i den nya storleken. Kontrollera importrapporten och operationernas urval.
# --units is an option of the command line.
pipeline-scale-units = Om DXF:ens enheter är fel, ange dem i stället vid uppdatering (--units), så att projektet sparar dem.

## Adding, updating and removing a DXF

# $file is the DXF's file, $id its number, $name its name in the project.
pipeline-dxf-added = Lade till { $file } som DXF { $id } "{ $name }": { $count } { $count ->
    [one] form
   *[other] former
}.
# $operations are operation numbers ("2, 5"); $name a DXF's name.
pipeline-dxf-pinned = { $count ->
    [one] Operation { $operations } fortsätter att bara fräsa { $name }: den nya DXF:en har också dess lager.
   *[other] Operationerna { $operations } fortsätter att bara fräsa { $name }: den nya DXF:en har också deras lager.
}
pipeline-dxf-removed = Tog bort DXF { $id } "{ $name }" och { $count ->
    [one] dess { $count } form
   *[other] dess { $count } former
}.
# $label is an operation's label; $shapes shape numbers ("S3, S4").
pipeline-dxf-shapes-gone = { $label }: formerna { $shapes } finns inte längre; välj dess geometri igen
pipeline-dxf-gone = { $label }: dess DXF finns inte längre; välj dess geometri igen
pipeline-dxf-updated = Uppdaterad från { $file }: { $unchanged } oförändrade, { $changed } ändrade, { $added } tillagda, { $removed } borttagna.
# $placement is a pipeline-placement text.
pipeline-dxf-placed-as-before = Placerad som tidigare: { $placement }.
pipeline-dxf-geometry-changed = { $label }: geometrin har ändrats (formerna { $shapes }); kontrollera dess verktygsbana
pipeline-dxf-layer-updated = { $label }: lagret { $layer } använder nu den uppdaterade ritningen; kontrollera dess verktygsbana

## Pictures and texts

# $number is an operation's number.
pipeline-left-out-found = Operation { $number }: { $count ->
    [one] dess { $count } utelämnade form hittades igen.
   *[other] dess { $count } utelämnade former hittades igen.
}
pipeline-left-out-lost = Operation { $number }: { $count } utelämnade { $count ->
    [one] form
   *[other] former
} hittades inte igen: välj formerna som ska utelämnas igen.
pipeline-no-image = det finns ingen bild { $id }
# $error is the PNG writer's own words.
pipeline-image-png-failed = när bilden skrevs: { $error }
# drawing.images is a place in the project file; $error the decoder's own words.
pipeline-image-unreadable = drawing.images: bilden för bild { $id } kan inte läsas: { $error }
# $file is the picture's file; $width a length without its unit, $height with it;
# $threshold a grey level (0–255); $picked "yes" when the threshold was picked from the
# picture; $lines "yes" for centre lines, "no" for outlines.
pipeline-image-traced = Spårade { $file } ({ $width } × { $height }, tröskel { $threshold }{ $picked ->
    [yes] , vald
   *[no] {""}
}): { $count } { $lines ->
    [yes] { $count ->
        [one] mittlinje
       *[other] mittlinjer
    }
   *[no] { $count ->
        [one] form
       *[other] former
    }
} på lager { $layer }.
pipeline-image-nothing-black = Inget är svart: ändra tröskeln, eller invertera om bilden är ljus på mörkt.
# $narrow is a length without its unit, $wide with it.
pipeline-image-strokes = Streck { $narrow } till { $wide } breda: fräs dem med följ bana.
pipeline-image-closed-lines = { $count ->
    [one] { $count } sluten linje: en konturfräsning eller ficka kan fräsa runt den, som runt vilken kontur som helst.
   *[other] { $count } slutna linjer: en konturfräsning eller ficka kan fräsa runt dem, som runt vilken kontur som helst.
}
pipeline-image-dots = { $count } { $count ->
    [one] prick för liten för en mittlinje utelämnad.
   *[other] prickar för små för en mittlinje utelämnade.
}
# $area is the smallest piece's area in mm², without its unit.
pipeline-image-specks = { $count } { $count ->
    [one] fläck mindre än { $area } mm² borttagen.
   *[other] fläckar mindre än { $area } mm² borttagna.
}
pipeline-image-black-at-border = Svart når bildens kant och sluts längs den; om det är bakgrunden, invertera.
pipeline-image-without-arcs = { $count ->
    [one] { $count } form behölls som linjer: dess bågar kom för nära en annan form.
   *[other] { $count } former behölls som linjer: deras bågar kom för nära en annan form.
}
pipeline-image-unsimplified = { $count } { $count ->
    [one] form behölls som spårad: utjämningen fick den att korsa sig.
   *[other] former behölls som spårade: utjämningen fick dem att korsa varandra.
}
pipeline-no-text = det finns ingen text { $id }
# $text is the text's first line in quotes; $error why (a toolpath-error text).
pipeline-text-join-failed = { $text }: kunde inte sammanfoga bokstävernas överlappande konturer: { $error }
# $text is the text's first line in quotes; $height its height; $width the box's width
# without its unit, $tall its height with it.
pipeline-text-wrote = Skrev { $text } i { $font }, { $height } hög ({ $width } × { $tall }): { $count } { $count ->
    [one] form
   *[other] former
} på lager { $layer }.
# $missing are the characters the font has no letter for.
pipeline-text-missing = { $font } har ingen bokstav för "{ $missing }": utelämnad.
pipeline-text-dropped = { $count } { $count ->
    [one] kontur för liten att fräsa utelämnad.
   *[other] konturer för små att fräsa utelämnade.
}

## Errors: operations and tools

# `tool use` is a command of the command line: keep it as written.
pipeline-no-tool-use = projektet har inget verktyg { $tool }; kopiera in ett från ditt bibliotek med `tool use`
pipeline-no-tool = projektet har inget verktyg { $tool }
# $number is the operation's number as listed, $count how many the project has.
pipeline-no-operation = det finns ingen operation { $number }; projektet har { $count }
pipeline-tab-no-ring = operation { $number } har ingen kontur inom en verktygsdiameter från { $at }; `chipload op tabs` listar dess bryggor
pipeline-tab-none-there = operation { $number } har ingen brygga inom en verktygsdiameter från { $at }; `chipload op tabs` listar dess bryggor
pipeline-tab-no-tabs = operation { $number } har inga bryggor: bara en konturfräsning med bryggor har några att placera
pipeline-corner-none-there = operation { $number } har inget hörn vid { $at }; `chipload op corners` listar dess hörn
pipeline-corner-not-profile-or-pocket = operation { $number } har inga hörn att välja: bara konturfräsningar och fickor har dogbones
pipeline-not-a-pocket = operation { $number } är inte en ficka; bara fickor kan restbearbetas
# $reason is one of the pipeline-rest-… texts.
pipeline-operation-refused = operation { $number }: { $reason }
pipeline-rest-dogbones = den har dogbones, och dogbones och restbearbetning kan inte kombineras än; dogbone-hörn behöver ingen restbearbetning
pipeline-rest-not-endmill = verktyg { $tool } är inte en pinnfräs
pipeline-rest-before-not-endmill = verktyg { $tool } före den är inte en pinnfräs
pipeline-rest-not-smaller = verktyg { $tool } ({ $diameter }) är inte mindre än verktyg { $before } ({ $before_diameter }) före den
# $dxfs lists the project's DXFs by number and name ("1 part-a, 2 part-b").
pipeline-update-which-dxf = projektet har { $count } DXF:er ({ $dxfs }): ange vilken som ska uppdateras
# `tool use` is a command of the command line: keep it as written.
pipeline-surface-no-endmill = projektet har ingen pinnfräs: en planfräsning behöver en (kopiera in en med `tool use`)
# $type is the tool's type as written in the project file (drill, vbit).
pipeline-surface-not-endmill = verktyg { $tool } är av typen { $type }; en planfräsning behöver en pinnfräs

## Errors: drawn shapes

pipeline-shape-not-drawable = bara cirklar, rektanglar och polylinjer kan ritas
# $kind is the shape's kind as written in the project file (circle, rectangle, polyline).
pipeline-shape-kind = formen { $id } är av typen { $kind }; dess typ kan inte ändras
pipeline-no-shapes-given = inga former angivna
pipeline-shape-twice = formen { $id } är angiven två gånger
pipeline-no-shape = det finns ingen form { $id }
pipeline-shape-imported-copy = formen { $id } importerades från DXF:en; bara ritade former kan kopieras eller speglas
pipeline-shapes-no-extent = formerna har ingen utsträckning
pipeline-shape-imported-transform = form { $id } är inte en ritad form: bara ritade former kan flyttas, vridas eller skalas (bryt upp dess DXF först)
pipeline-transform-not-numbers = flytten, vridningen och mittpunkten måste vara tal
pipeline-shapes-copy-imported = bara ritade former kan kopieras, inte importerade
# "Update drawing from DXF" is the command in the app's File menu.
pipeline-shape-imported-change = formen { $id } importerades från DXF:en och kan inte ändras här; ändra ritningen och använd Uppdatera ritning från DXF

## Errors: parts and copies

pipeline-no-part = Det finns ingen detalj { $id } i projektet
pipeline-part-name-empty = En detalj måste ha ett namn
pipeline-part-name-taken = En annan detalj heter "{ $name }"
pipeline-last-part = Ett projekt behöver minst en detalj: det här är den sista
pipeline-part-has-operations = Detaljen { $name } har { $count ->
    [one] { $count } operation
   *[other] { $count } operationer
}: flytta dem till en annan detalj först, eller ta bort dem tillsammans med den
pipeline-part-no-operation = Det finns ingen operation { $number } i projektet
pipeline-part-cuts-nothing = Detaljen fräser ingenting
pipeline-copies-gap = Mellanrummet måste vara 0 eller mer, fick { $got }

## Errors: the simulation

# $min is a length without its unit, $max and $got with it.
pipeline-simulate-cell-size = cellstorleken måste vara mellan { $min } och { $max }, fick { $got }
# $cell is the size of one cell, a length.
pipeline-simulate-too-many-cells = en cell på { $cell } ger { $cells } celler för det här ämnet, fler än { $most }; använd en större cell
pipeline-simulate-stock-too-large = ämnet är för stort för att simulera med celler på { $cell } (fler än { $most } celler)
# $error is the PNG encoder's own words.
pipeline-simulate-png = PNG: { $error }

## Post-processors

pipeline-post-no-settings = ingen inställningsmapp, alltså ingen postprocessor "{ $name }"
pipeline-post-no-folder = Ingen postprocessor "{ $name }": den är inte inbyggd, och det finns ingen posts-mapp
# Why a project's copy of a user post can't be compared with its file.
pipeline-post-no-posts-folder = ingen posts-mapp
# What a post-processor is, one line each (Settings, This computer). $name, $extension,
# the paths and the units (mm, inch) and modes (files, program) are written as in its file.
pipeline-post-name = Namn: { $name }
pipeline-post-builtin = Inbyggd
pipeline-post-yours = Din egen
pipeline-post-settings = Inställningar: { $path }
pipeline-post-template = Mall: { $path }
pipeline-post-extension = Filändelse: { $extension }
pipeline-post-decimals = Decimaler: { $mm } (mm), { $inch } (inch)
pipeline-post-units-list = G-kodens enheter: { $units }
pipeline-post-arcs = Bågar: { $arcs ->
    [arcs] G2/G3
   *[lines] som korta linjer (G1)
}
pipeline-post-tool-changes = Flera verktyg: { $modes }
# $seconds is a time in seconds, without its unit.
pipeline-post-dwell = Spindelns väntetid: { $seconds } s
pipeline-post-no-dwell = Spindelns väntetid: ingen

## The tool library and machine profiles

# $error is why the library file couldn't be read (a job-library-error text).
pipeline-library-defaults = { $error } (använder inbyggda standardvärden)
pipeline-library-no-profile = det finns ingen maskinprofil som heter "{ $name }"
# What a new project started with. $how is "profile" (asked for) or "default".
pipeline-library-machine = maskin: { $name } ({ $how ->
    [default] standardprofil
   *[profile] profil
})
pipeline-library-tool = verktyg { $tool }: { $name } (standardverktyg)
# $error is why the post can't be used (a post-error text).
pipeline-library-post-unusable = postprocessor "{ $name }": { $error }
pipeline-library-unknown-post = Maskinen "{ $name }": den här Chipload har ingen postprocessor "{ $post }"; projekt som använder den kan inte göra G-kod förrän den har det

## Recovery copies

# $reason is the operating system's words, or the text below.
pipeline-recovery-io = Kan inte skriva återställningskopian { $path }: { $reason }
pipeline-recovery-locked = redan låst

## Dragging a picture's or text's box

pipeline-box-turn-centre = vridhandtaget ligger på centrum
pipeline-box-no-size = ramen har ingen storlek
# $min is the smallest size, a length.
pipeline-box-too-small = den kan inte vara mindre än { $min }
pipeline-box-no-corner = en ram har inget hörn { $corner }

## G-code files

# One of several G-code files: $name is its file name, $operations the numbers of the
# operations it cuts ("1, 3").
pipeline-gcode-file = { $name } (verktyg { $tool }, operationerna { $operations })

## Explode a DXF, join close ends (docs/specs/explode-and-join.md)

pipeline-dxf-exploded = Bröt upp DXF { $id } "{ $name }" i { $count } ritade { $count ->
    [one] form
   *[other] former
}. Projektet känner inte längre till DXF-filen.
# $shapes shape numbers ("S3, S4"); $tolerance a length.
pipeline-explode-split = { $count ->
    [one] Form { $shapes } bestod av delar som inte möts: varje del är nu en egen form.
   *[other] Formerna { $shapes } bestod av delar som inte möts: varje del är nu en egen form.
}
pipeline-explode-circle-start = { $count ->
    [one] Cirkel { $shapes } börjar nu i sin högra punkt, moturs, som ritade cirklar gör: fräsningen kan börja på ett annat ställe på den.
   *[other] Cirklarna { $shapes } börjar nu i sina högra punkter, moturs, som ritade cirklar gör: fräsningen kan börja på ett annat ställe på dem.
}
pipeline-explode-dropped = { $count ->
    [one] Form { $shapes } hade bara segment kortare än { $tolerance } och är borta.
   *[other] Formerna { $shapes } hade bara segment kortare än { $tolerance } och är borta.
}
# $label is an operation's label.
pipeline-explode-selects = { $label } markerar nu formerna { $shapes }.
pipeline-explode-no-dxf = projektet har ingen DXF att bryta upp
pipeline-explode-which-dxf = projektet har { $count } DXF:er ({ $dxfs }): ange vilken som ska brytas upp
pipeline-join-gap-range = Glappet måste vara { $least } till { $most }, fick { $got }.
pipeline-join-imported = Form S{ $id } kommer från en DXF: bryt upp DXF:en först.
pipeline-join-none = Inga ändar fogades ihop (glapp { $gap }).
pipeline-joined = Fogade ihop { $pieces } former till { $count }, varav { $closed } slutna; största glappet som slöts var { $widest }.
pipeline-joined-touching = Fogade ihop { $pieces } former till { $count }, varav { $closed } slutna; deras ändar möttes.
pipeline-join-open = { $count } { $count ->
    [one] ände förblir öppen
   *[other] ändar förblir öppna
}.
# $at a point, $nearest a length.
pipeline-join-open-end = Änden vid { $at }: närmaste andra ände ligger { $nearest } bort.
pipeline-join-open-alone = Änden vid { $at }: ingen annan ände på dess lager.
pipeline-join-branch = Tre eller fler ändar möts vid { $at }: inte ihopfogade där.
pipeline-edit-imported = S{ $id } kommer från en DXF, en bild eller en text: lös upp DXF:en, eller rita formen, för att ändra den
pipeline-offset-distance = förskjutningen måste vara större än 0, fick { $got }
pipeline-offset-vanishes = ingenting av S{ $id } finns kvar { $distance } innanför
pipeline-offset-side = S{ $id } är { $closed ->
    [closed] sluten: förskjut den utåt eller inåt
   *[open] öppen: förskjut den åt vänster eller höger
}
pipeline-offset-failed = S{ $id } kunde inte förskjutas: { $why }
pipeline-new-shapes-cut = { $count ->
    [one] Den nya formen ligger
   *[other] De { $count } nya formerna ligger
} på lagret { $layer }, som { $ops ->
    [one] operation { $operations } skär
   *[other] operationerna { $operations } skär
}.
pipeline-corner-none = S{ $id } har inga hörn att runda eller fasa
pipeline-corner-not = hörn { $corner } på S{ $id } ligger inte mellan två raka sidor
pipeline-corner-straight = S{ $id } går rakt fram vid { $corner }: där finns inget hörn
pipeline-corner-too-large = det är för stort för sidorna på S{ $id }: högst { $most }
pipeline-corner-size = radien eller avståndet måste vara större än 0
