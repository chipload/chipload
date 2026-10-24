# Messages of the app services (crates/chipload-pipeline): problems found with the
# post-processor a project uses, planning notes, the time estimate, chip loads and the
# drawing's facts. A length or feed argument brings its own unit ("12.5 mm",
# "1200 mm/min").

# $post is the post-processor's name, $units the G-code units the machine is set to, as
# written in the project file ("inch"). "units" is a key of the post's config file.
pipeline-post-units = le post-processeur { $post } n'écrit que du G-code en mm (sa configuration ne liste pas "{ $units }" sous units)

## Planning notes

pipeline-drawing-outside-stock = Le dessin dépasse du brut
# $count is the number of copies. $x0 and $y0 are numbers without their unit; $x1, $y1
# and $stock_y bring it; $stock_x is a number without it.
pipeline-copies-beyond-stock = Les { $count } copies usinent de X{ $x0 } à { $x1 }, de Y{ $y0 } à { $y1 }, au-delà du brut ({ $stock_x } × { $stock_y }) ; vérifiez le brut ou les copies de la pièce.
# As pipeline-copies-beyond-stock, for one part of several; $part is its name.
pipeline-part-copies-beyond-stock = Pièce { $part } : les { $count } copies usinent de X{ $x0 } à { $x1 }, de Y{ $y0 } à { $y1 }, au-delà du brut ({ $stock_x } × { $stock_y }) ; vérifiez le brut ou les copies de la pièce.
# $label is an operation's label, $layer a layer's name, $sources where the layer's shapes
# come from (DXF and picture names, joined with pipeline-and).
pipeline-layer-clash = { $label } : le calque "{ $layer }" est dans { $sources } ; l'opération les usine tous. Limitez-la à un seul (sélectionnez le calque sous ce DXF dans le Navigateur)
# Names joined: "a.dxf and b.dxf".
pipeline-and = { $list } et { $last }
# Where shapes come from: a DXF's or picture's name, or one without a name.
pipeline-source-named = { $name }
pipeline-source-dxf = DXF { $id }
pipeline-source-image = image { $id }
pipeline-source-drawn = formes dessinées

## The time estimate (after planning and generating)

# A duration: "45 s", "12 min 30 s", "1 h 05 min". $minutes after hours has two digits.
pipeline-duration-seconds = { $seconds } s
pipeline-duration-minutes = { $minutes } min { $seconds } s
pipeline-duration-hours = { $hours } h { $minutes } min
# The durations are pipeline-duration-… texts; $changes is the number of tool changes.
pipeline-estimate = Temps estimé : { $total } (coupe { $cutting }, rapides { $rapids }, broche { $spindle }){ $changes ->
    [0] {""}
    [one] , plus { $changes } changement d'outil
   *[other] , plus { $changes } changements d'outil
}
# The lines under it start with spaces, written {"  "}: keep them, they indent the line.
# $name is the part's name.
pipeline-estimate-part = {"  "}Pièce { $number } ({ $name }) : { $time }
# $label is an operation's label.
pipeline-estimate-operation = {"  "}{ $label } : { $time }
# An operation under its part: indented deeper.
pipeline-estimate-part-operation = {"    "}{ $label } : { $time }
# One G-code file of a project written one file per tool.
pipeline-estimate-file = {"  "}Fichier { $number } : { $time }
# $rate is a feed ("5000 mm/min").
pipeline-estimate-rapids-assumed = {"  "}Aux avances programmées (rapides à { $rate }, supposé : réglez l'avance max de la machine pour une meilleure estimation) ; l'accélération et les coins rallongent le temps réel.
pipeline-estimate-rapids = {"  "}Aux avances programmées (rapides à l'avance max de la machine, { $rate }) ; l'accélération et les coins rallongent le temps réel.

## Chip load

pipeline-chip-load-title = Avance par dent (à l'avance de coupe) :
# $label is an operation's label, $load a pipeline-chip-load… text.
pipeline-chip-load-line = {"  "}{ $label } : { $load }
# $load brings its unit ("0.042 mm/tooth").
pipeline-chip-load = { $load }
pipeline-chip-load-no-flutes = réglez le nombre de dents de l'outil pour voir l'avance par dent
pipeline-chip-load-no-rpm = réglez la vitesse de broche de l'outil pour voir l'avance par dent
pipeline-chip-load-no-tool = aucun outil
# The hover text next to an operation's chip load. Tool diameters $small_tool (a number)
# and $large_tool (with its unit); chip loads $wood_low, $aluminium_low and
# $aluminium_high (numbers) and $wood_high (with its unit).
pipeline-chip-load-hint = Avance par dent : l'épaisseur du copeau que prend chaque arête de coupe, avance / (vitesse de broche × dents). Trop fin, l'outil frotte et chauffe ; trop épais, il peut casser. Elle dépend de la matière, de l'outil et de la machine ; à titre indicatif, le bois et les plastiques avec des outils de { $small_tool }–{ $large_tool } prennent souvent { $wood_low }–{ $wood_high }, l'aluminium { $aluminium_low }–{ $aluminium_high }. Vérifiez les valeurs du fabricant de l'outil.

## The drawing's facts (after importing)

# $units is "mm" or "inch", as the DXF gives them or the user chose them.
pipeline-report-units = Unités : { $units } ({ $from ->
    [dxf] du dxf
   *[user] choix de l'utilisateur
})
# Numbers without their unit, except $height, which brings it.
pipeline-report-extents = Étendue : X { $x0 } .. { $x1 }, Y { $y0 } .. { $y1 } ({ $width } x { $height })
# $items lists DXF entity types with their counts ("LINE 4, ARC 2").
pipeline-report-imported = Importé : { $items }
pipeline-report-flattened = Aplati : { $items }
pipeline-report-ignored = Ignoré : { $items }
pipeline-report-no-issues = Problèmes : aucun
pipeline-report-issues = Problèmes ({ $count }) :
# One issue of the import (a dxf-… text), indented.
pipeline-report-issue = {"  "}[{ $severity ->
    [warning] avertissement
   *[error] erreur
}] { $issue }
pipeline-report-contours = Contours ({ $count }) :
# A row of the contour table, in a fixed-width font: $id, $state and $layers are padded
# with spaces to line up. The coordinates are numbers without their unit.
pipeline-report-contour = {"  "}#{ $id } { $state } calque { $layers } X { $x0 } .. { $x1 }  Y { $y0 } .. { $y1 }
# A contour's state in the table: keep it short.
pipeline-report-contour-state = { $state ->
    [open] ouvert
    [invalid] INVALIDE
    [hole] trou
   *[outer] extérieur
}

## The G-code checker's findings (check-… texts), placed in the program

# A finding on a line of the program; $message is the finding.
pipeline-check-line = line { $line }: { $message }
# A finding in one of several G-code files: $file is the file's number.
pipeline-check-file = file { $file }: { $message }
# A finding on a line of one of several G-code files.
pipeline-check-file-line = file { $file }, line { $line }: { $message }
