# Messages of the app services (crates/chipload-pipeline): problems found with the
# post-processor a project uses, planning notes, the time estimate, chip loads and the
# drawing's facts. A length or feed argument brings its own unit ("12.5 mm",
# "1200 mm/min").

# $post is the post-processor's name, $units the G-code units the machine is set to, as
# written in the project file ("inch"). "units" is a key of the post's config file.
pipeline-post-units = el postprocesador { $post } solo escribe código G en mm (su configuración no incluye "{ $units }" en units)

## Planning notes

pipeline-drawing-outside-stock = El dibujo se sale del material en bruto
# $count is the number of copies. $x0 and $y0 are numbers without their unit; $x1, $y1
# and $stock_y bring it; $stock_x is a number without it.
pipeline-copies-beyond-stock = Las { $count } copias cortan de X{ $x0 } a { $x1 }, de Y{ $y0 } a { $y1 }, fuera del material en bruto ({ $stock_x } × { $stock_y }); revisa el material en bruto o las copias de la pieza.
# As pipeline-copies-beyond-stock, for one part of several; $part is its name.
pipeline-part-copies-beyond-stock = Pieza { $part }: las { $count } copias cortan de X{ $x0 } a { $x1 }, de Y{ $y0 } a { $y1 }, fuera del material en bruto ({ $stock_x } × { $stock_y }); revisa el material en bruto o las copias de la pieza.
# $label is an operation's label, $layer a layer's name, $sources where the layer's shapes
# come from (DXF and picture names, joined with pipeline-and).
pipeline-layer-clash = { $label }: la capa "{ $layer }" está en { $sources }; se cortan todas. Limítala a una (selecciona la capa bajo ese DXF en el Navegador)
# Names joined: "a.dxf and b.dxf".
pipeline-and = { $list } y { $last }
# Where shapes come from: a DXF's or picture's name, or one without a name.
pipeline-source-named = { $name }
pipeline-source-dxf = DXF { $id }
pipeline-source-image = imagen { $id }
pipeline-source-drawn = formas dibujadas

## The time estimate (after planning and generating)

# A duration: "45 s", "12 min 30 s", "1 h 05 min". $minutes after hours has two digits.
pipeline-duration-seconds = { $seconds } s
pipeline-duration-minutes = { $minutes } min { $seconds } s
pipeline-duration-hours = { $hours } h { $minutes } min
# The durations are pipeline-duration-… texts; $changes is the number of tool changes.
pipeline-estimate = Tiempo estimado: { $total } (corte { $cutting }, rápidos { $rapids }, husillo { $spindle }){ $changes ->
    [0] {""}
    [one] , más { $changes } cambio de herramienta
   *[other] , más { $changes } cambios de herramienta
}
# The lines under it start with spaces, written {"  "}: keep them, they indent the line.
# $name is the part's name.
pipeline-estimate-part = {"  "}Pieza { $number } ({ $name }): { $time }
# $label is an operation's label.
pipeline-estimate-operation = {"  "}{ $label }: { $time }
# An operation under its part: indented deeper.
pipeline-estimate-part-operation = {"    "}{ $label }: { $time }
# One G-code file of a project written one file per tool.
pipeline-estimate-file = {"  "}Archivo { $number }: { $time }
# $rate is a feed ("5000 mm/min").
pipeline-estimate-rapids-assumed = {"  "}A los avances programados (rápidos a { $rate }, supuesto: fija el avance máximo de la máquina para una mejor estimación); la aceleración y las esquinas alargan el tiempo real.
pipeline-estimate-rapids = {"  "}A los avances programados (rápidos al avance máximo de la máquina, { $rate }); la aceleración y las esquinas alargan el tiempo real.

## Chip load

pipeline-chip-load-title = Avance por diente (al avance de corte):
# $label is an operation's label, $load a pipeline-chip-load… text.
pipeline-chip-load-line = {"  "}{ $label }: { $load }
# $load brings its unit ("0.042 mm/tooth").
pipeline-chip-load = { $load }
pipeline-chip-load-no-flutes = fija los filos de la herramienta para ver el avance por diente
pipeline-chip-load-no-rpm = fija la velocidad del husillo de la herramienta para ver el avance por diente
pipeline-chip-load-no-tool = sin herramienta
# The hover text next to an operation's chip load. Tool diameters $small_tool (a number)
# and $large_tool (with its unit); chip loads $wood_low, $aluminium_low and
# $aluminium_high (numbers) and $wood_high (with its unit).
pipeline-chip-load-hint = Avance por diente: el grosor de viruta que arranca cada filo, avance / (velocidad del husillo × filos). Demasiado fino y la herramienta roza y se calienta; demasiado grueso y puede romperse. Depende del material, la herramienta y la máquina; como guía aproximada, la madera y los plásticos con herramientas de { $small_tool }–{ $large_tool } suelen admitir { $wood_low }–{ $wood_high }, el aluminio { $aluminium_low }–{ $aluminium_high }. Consulta los datos del fabricante de la herramienta.

## The drawing's facts (after importing)

# $units is "mm" or "inch", as the DXF gives them or the user chose them.
pipeline-report-units = Unidades: { $units } ({ $from ->
    [dxf] del DXF
   *[user] elegidas por ti
})
# Numbers without their unit, except $height, which brings it.
pipeline-report-extents = Extensión: X { $x0 } .. { $x1 }, Y { $y0 } .. { $y1 } ({ $width } x { $height })
# $items lists DXF entity types with their counts ("LINE 4, ARC 2").
pipeline-report-imported = Importado: { $items }
pipeline-report-flattened = Aplanado: { $items }
pipeline-report-ignored = Ignorado: { $items }
pipeline-report-no-issues = Incidencias: ninguna
pipeline-report-issues = Incidencias ({ $count }):
# One issue of the import (a dxf-… text), indented.
pipeline-report-issue = {"  "}[{ $severity ->
    [warning] aviso
   *[error] error
}] { $issue }
pipeline-report-contours = Contornos ({ $count }):
# A row of the contour table, in a fixed-width font: $id, $state and $layers are padded
# with spaces to line up. The coordinates are numbers without their unit.
pipeline-report-contour = {"  "}#{ $id } { $state } capa { $layers } X { $x0 } .. { $x1 }  Y { $y0 } .. { $y1 }
# A contour's state in the table: keep it short.
pipeline-report-contour-state = { $state ->
    [open] abierto
    [invalid] NO VÁLIDO
    [hole] agujero
   *[outer] exterior
}

## The G-code checker's findings (check-… texts), placed in the program

# A finding on a line of the program; $message is the finding.
pipeline-check-line = línea { $line }: { $message }
# A finding in one of several G-code files: $file is the file's number.
pipeline-check-file = archivo { $file }: { $message }
# A finding on a line of one of several G-code files.
pipeline-check-file-line = archivo { $file }, línea { $line }: { $message }
