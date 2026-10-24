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
