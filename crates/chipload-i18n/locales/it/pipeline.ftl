# Messages of the app services (crates/chipload-pipeline): problems found with the
# post-processor a project uses, planning notes, the time estimate, chip loads and the
# drawing's facts. A length or feed argument brings its own unit ("12.5 mm",
# "1200 mm/min").

# $post is the post-processor's name, $units the G-code units the machine is set to, as
# written in the project file ("inch"). "units" is a key of the post's config file.
pipeline-post-units = il post-processore { $post } scrive solo G-code in mm (la sua configurazione non elenca "{ $units }" in units)

## Planning notes

pipeline-drawing-outside-stock = Il disegno esce dal grezzo
# $count is the number of copies. $x0 and $y0 are numbers without their unit; $x1, $y1
# and $stock_y bring it; $stock_x is a number without it.
pipeline-copies-beyond-stock = Le { $count } copie tagliano da X{ $x0 } a { $x1 }, da Y{ $y0 } a { $y1 }, oltre il grezzo ({ $stock_x } × { $stock_y }); controlla il grezzo o le copie del pezzo.
# As pipeline-copies-beyond-stock, for one part of several; $part is its name.
pipeline-part-copies-beyond-stock = Pezzo { $part }: le { $count } copie tagliano da X{ $x0 } a { $x1 }, da Y{ $y0 } a { $y1 }, oltre il grezzo ({ $stock_x } × { $stock_y }); controlla il grezzo o le copie del pezzo.
# $label is an operation's label, $layer a layer's name, $sources where the layer's shapes
# come from (DXF and picture names, joined with pipeline-and).
pipeline-layer-clash = { $label }: il layer "{ $layer }" è in { $sources }; li taglia tutti. Limitalo a uno (seleziona il layer sotto quel DXF nel Browser)
# Names joined: "a.dxf and b.dxf".
pipeline-and = { $list } e { $last }
# Where shapes come from: a DXF's or picture's name, or one without a name.
pipeline-source-named = { $name }
pipeline-source-dxf = DXF { $id }
pipeline-source-image = immagine { $id }
pipeline-source-drawn = forme disegnate

## The time estimate (after planning and generating)

# A duration: "45 s", "12 min 30 s", "1 h 05 min". $minutes after hours has two digits.
pipeline-duration-seconds = { $seconds } s
pipeline-duration-minutes = { $minutes } min { $seconds } s
pipeline-duration-hours = { $hours } h { $minutes } min
# The durations are pipeline-duration-… texts; $changes is the number of tool changes.
pipeline-estimate = Tempo stimato: { $total } (taglio { $cutting }, rapidi { $rapids }, mandrino { $spindle }){ $changes ->
    [0] {""}
    [one] , più { $changes } cambio utensile
   *[other] , più { $changes } cambi utensile
}
# The lines under it start with spaces, written {"  "}: keep them, they indent the line.
# $name is the part's name.
pipeline-estimate-part = {"  "}Pezzo { $number } ({ $name }): { $time }
# $label is an operation's label.
pipeline-estimate-operation = {"  "}{ $label }: { $time }
# An operation under its part: indented deeper.
pipeline-estimate-part-operation = {"    "}{ $label }: { $time }
# One G-code file of a project written one file per tool.
pipeline-estimate-file = {"  "}File { $number }: { $time }
# $rate is a feed ("5000 mm/min").
pipeline-estimate-rapids-assumed = {"  "}Agli avanzamenti programmati (rapidi a { $rate }, valore ipotizzato: imposta l'avanzamento massimo della macchina per una stima migliore); accelerazioni e angoli allungano il tempo reale.
pipeline-estimate-rapids = {"  "}Agli avanzamenti programmati (rapidi all'avanzamento massimo della macchina, { $rate }); accelerazioni e angoli allungano il tempo reale.

## Chip load

pipeline-chip-load-title = Avanzamento per dente (all'avanzamento di taglio):
# $label is an operation's label, $load a pipeline-chip-load… text.
pipeline-chip-load-line = {"  "}{ $label }: { $load }
# $load brings its unit ("0.042 mm/tooth").
pipeline-chip-load = { $load }
pipeline-chip-load-no-flutes = imposta i taglienti dell'utensile per vedere l'avanzamento per dente
pipeline-chip-load-no-rpm = imposta la velocità mandrino dell'utensile per vedere l'avanzamento per dente
pipeline-chip-load-no-tool = nessun utensile
# The hover text next to an operation's chip load. Tool diameters $small_tool (a number)
# and $large_tool (with its unit); chip loads $wood_low, $aluminium_low and
# $aluminium_high (numbers) and $wood_high (with its unit).
pipeline-chip-load-hint = Avanzamento per dente: lo spessore del truciolo asportato da ogni tagliente, avanzamento / (velocità mandrino × taglienti). Troppo sottile e l'utensile sfrega e si surriscalda; troppo spesso e può rompersi. Dipende dal materiale, dall'utensile e dalla macchina; come indicazione di massima, legno e plastiche con utensili da { $small_tool }–{ $large_tool } accettano spesso { $wood_low }–{ $wood_high }, l'alluminio { $aluminium_low }–{ $aluminium_high }. Verifica i valori del produttore dell'utensile.

## The drawing's facts (after importing)

# $units is "mm" or "inch", as the DXF gives them or the user chose them.
pipeline-report-units = Unità: { $units } (da { $from ->
    [dxf] dxf
   *[user] utente
})
# Numbers without their unit, except $height, which brings it.
pipeline-report-extents = Ingombro: X { $x0 } .. { $x1 }, Y { $y0 } .. { $y1 } ({ $width } x { $height })
# $items lists DXF entity types with their counts ("LINE 4, ARC 2").
pipeline-report-imported = Importate: { $items }
pipeline-report-flattened = Appiattite: { $items }
pipeline-report-ignored = Ignorate: { $items }
pipeline-report-no-issues = Anomalie: nessuna
pipeline-report-issues = Anomalie ({ $count }):
# One issue of the import (a dxf-… text), indented.
pipeline-report-issue = {"  "}[{ $severity ->
    [warning] avviso
   *[error] errore
}] { $issue }
pipeline-report-contours = Contorni ({ $count }):
# A row of the contour table, in a fixed-width font: $id, $state and $layers are padded
# with spaces to line up. The coordinates are numbers without their unit.
pipeline-report-contour = {"  "}#{ $id } { $state } layer { $layers } X { $x0 } .. { $x1 }  Y { $y0 } .. { $y1 }
# A contour's state in the table: keep it short.
pipeline-report-contour-state = { $state ->
    [open] aperto
    [invalid] NON VALIDO
    [hole] foro
   *[outer] esterno
}

## The G-code checker's findings (check-… texts), placed in the program

# A finding on a line of the program; $message is the finding.
pipeline-check-line = riga { $line }: { $message }
# A finding in one of several G-code files: $file is the file's number.
pipeline-check-file = file { $file }: { $message }
# A finding on a line of one of several G-code files.
pipeline-check-file-line = file { $file }, riga { $line }: { $message }
