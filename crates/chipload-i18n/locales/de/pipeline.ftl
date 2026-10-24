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
