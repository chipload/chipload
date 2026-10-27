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

## Errors: a change or run the project refuses

# The independent checker refused the program; each finding follows on its own line.
pipeline-error-check-failed = Verifica del G-code non riuscita:
# One more finding (a pipeline-check-… text) on a line of its own, indented, after $list.
pipeline-error-check-finding =
    { $list }
    {"  "}{ $finding }
# Words of the command line or another library, kept as they are.
pipeline-text = { $text }

## Reports of changes to the drawing

# $x0 and $y0 are lengths without their unit, $x1 and $y1 with it; $width and $length the
# stock's size, the first without its unit.
pipeline-stock-fit = Il disegno (da X{ $x0 } a { $x1 }, da Y{ $y0 } a { $y1 }) esce dal grezzo ({ $width } × { $length }); controlla il grezzo in Impostazioni lavoro.

## Placing a DXF: moving, turning and scaling it

# $count is how many DXFs the project has, $dxfs their numbers and names ("1 part-a, 2 part-b").
pipeline-place-which-dxf = il progetto ha { $count } DXF ({ $dxfs }): indica quale posizionare
pipeline-no-dxf = non esiste il DXF { $id }
pipeline-place-nothing-imported = il progetto non ha un disegno importato da spostare
# What is placed, when the project has a single DXF (otherwise it is the DXF's name).
pipeline-place-the-drawing = il disegno importato
# $dx and $dy are numbers as typed, without a unit.
pipeline-place-cannot-move = impossibile spostare il disegno di ({ $dx }, { $dy })
pipeline-place-already-there = Il disegno è già lì; nulla è stato spostato.
# $what is pipeline-place-the-drawing or a DXF's name; $degrees without its unit.
pipeline-place-turned = Ruotato { $what } di { $degrees }° in senso { $direction ->
    [ccw] antiorario
   *[cw] orario
}
pipeline-place-reset = Riportato { $what } dove lo colloca il DXF
# $factor is a scale factor ("2", "0.5").
pipeline-place-scaled = Scalato { $what } ×{ $factor }
# $scaled is pipeline-place-scaled; $width is a length without its unit, $height with it.
pipeline-place-scaled-size = { $scaled } (ora { $width } × { $height })
# $dx is a length without its unit, $dy with it.
pipeline-place-moved = Spostato { $what } di X{ $dx } Y{ $dy }
# $done is one of the texts above; $placement a pipeline-placement text.
pipeline-place-done = { $done }; ora è { $placement }.
pipeline-place-moved-starts = { $count ->
    [one] Spostato con esso anche { $count } punto iniziale.
   *[other] Spostati con esso anche { $count } punti iniziali.
}
pipeline-place-moved-corners = { $count ->
    [one] Spostata con esso anche { $count } scelta d'angolo.
   *[other] Spostate con esso anche { $count } scelte d'angolo.
}
pipeline-place-moved-tabs = { $count ->
    [one] Spostato con esso anche { $count } ponticello posto a mano.
   *[other] Spostati con esso anche { $count } ponticelli posti a mano.
}
pipeline-place-drawn-stay = Le forme disegnate in Chipload restano dove sono.
pipeline-place-other-dxfs-stay = Gli altri DXF restano dove sono.
pipeline-place-work-zero-stays = Lo zero pezzo resta nel punto X{ $x }, Y{ $y }; controlla che sia ancora dove lo vuoi.
pipeline-place-margin = il margine deve essere 0 o più, trovato { $got }
# $got is a number as typed, in degrees, without a unit.
pipeline-place-rotation = la rotazione deve essere un numero di gradi, trovato { $got }
pipeline-place-scale = la scala deve essere un numero maggiore di 0, trovato { $got }
# Scale factors: the result, and the smallest and largest allowed.
pipeline-place-scale-range = così il disegno sarebbe scalato ×{ $factor } rispetto al DXF; deve restare tra ×{ $min } e ×{ $max }
# $what is "width" or "height"; $got a length without its unit.
pipeline-place-size = { $what ->
    [height] l'altezza
   *[width] la larghezza
} deve essere maggiore di 0, trovato { $got }
pipeline-place-no-size = il disegno importato non ha { $what ->
    [height] altezza
   *[width] larghezza
} da scalare
# How a DXF is placed: "scaled ×2, turned 90°, moved X12.000 Y-3.500 from the DXF".
pipeline-placement-as-dxf = come nel DXF
pipeline-placement-from-dxf = { $parts } rispetto al DXF
pipeline-placement-and = { $list }, { $last }
pipeline-placement-scaled = scalato ×{ $factor }
pipeline-placement-turned = ruotato di { $degrees }°
# $dx and $dy are lengths without their unit.
pipeline-placement-moved = spostato di X{ $dx } Y{ $dy }
# After scaling: $within and $off are lengths, $factor the scale factor.
pipeline-scale-curves = { $count ->
    [one] { $count } forma da spline o ellissi era approssimata entro { $within } alla dimensione precedente; scalata ×{ $factor }, può discostarsi fino a { $off } dalla sua curva. Aggiorna dal DXF per approssimarla di nuovo a questa dimensione.
   *[other] { $count } forme da spline o ellissi erano approssimate entro { $within } alla dimensione precedente; scalate ×{ $factor }, possono discostarsi fino a { $off } dalle loro curve. Aggiorna dal DXF per approssimarle di nuovo a questa dimensione.
}
pipeline-scale-contours = { $was } contorni chiusi prima, { $now } ora: le interruzioni vengono unite entro { $within } alla nuova dimensione. Controlla il report di importazione e le selezioni delle lavorazioni.
# --units is an option of the command line.
pipeline-scale-units = Se le unità del DXF sono sbagliate, impostale invece all'aggiornamento (--units), così il progetto le registra.

## Adding, updating and removing a DXF

# $file is the DXF's file, $id its number, $name its name in the project.
pipeline-dxf-added = Aggiunto { $file } come DXF { $id } "{ $name }": { $count ->
    [one] { $count } forma
   *[other] { $count } forme
}.
# $operations are operation numbers ("2, 5"); $name a DXF's name.
pipeline-dxf-pinned = { $count ->
    [one] La lavorazione { $operations } continua a tagliare solo { $name }: anche il nuovo DXF ha i suoi layer.
   *[other] Le lavorazioni { $operations } continuano a tagliare solo { $name }: anche il nuovo DXF ha i loro layer.
}
pipeline-dxf-removed = Rimosso il DXF { $id } "{ $name }" e { $count ->
    [one] la sua { $count } forma
   *[other] le sue { $count } forme
}.
# $label is an operation's label; $shapes shape numbers ("S3, S4").
pipeline-dxf-shapes-gone = { $label }: le forme { $shapes } non esistono più; scegli di nuovo la sua geometria
pipeline-dxf-gone = { $label }: il suo DXF non c'è più; scegli di nuovo la sua geometria
pipeline-dxf-updated = Aggiornato da { $file }: { $unchanged } invariate, { $changed } cambiate, { $added } aggiunte, { $removed } rimosse.
# $placement is a pipeline-placement text.
pipeline-dxf-placed-as-before = Posizionato come prima: { $placement }.
pipeline-dxf-geometry-changed = { $label }: geometria cambiata (forme { $shapes }); controlla il suo percorso utensile
pipeline-dxf-layer-updated = { $label }: il layer { $layer } ora usa il disegno aggiornato; controlla il suo percorso utensile

## Pictures and texts

# $number is an operation's number.
pipeline-left-out-found = Lavorazione { $number }: { $count ->
    [one] la sua { $count } forma esclusa è stata ritrovata.
   *[other] le sue { $count } forme escluse sono state ritrovate.
}
pipeline-left-out-lost = Lavorazione { $number }: { $count ->
    [one] { $count } forma esclusa non ritrovata
   *[other] { $count } forme escluse non ritrovate
}: scegli di nuovo le forme da escludere.
pipeline-no-image = non esiste l'immagine { $id }
# $error is the PNG writer's own words.
pipeline-image-png-failed = scrittura dell'immagine: { $error }
# drawing.images is a place in the project file; $error the decoder's own words.
pipeline-image-unreadable = drawing.images: impossibile leggere il file dell'immagine { $id }: { $error }
# $file is the picture's file; $width a length without its unit, $height with it;
# $threshold a grey level (0–255); $picked "yes" when the threshold was picked from the
# picture; $lines "yes" for centre lines, "no" for outlines.
pipeline-image-traced = Vettorizzata { $file } ({ $width } × { $height }, soglia { $threshold }{ $picked ->
    [yes] , scelta dall'immagine
   *[no] {""}
}): { $count } { $lines ->
    [yes] { $count ->
        [one] linea mediana
       *[other] linee mediane
    }
   *[no] { $count ->
        [one] forma
       *[other] forme
    }
} sul layer { $layer }.
pipeline-image-nothing-black = Niente è nero: cambia la soglia, o inverti se l'immagine è chiara su scuro.
# $narrow is a length without its unit, $wide with it.
pipeline-image-strokes = Tratti larghi da { $narrow } a { $wide }: tagliali con un segui percorso.
pipeline-image-closed-lines = { $count ->
    [one] { $count } linea chiusa: una contornatura o una tasca può tagliare attorno a essa, come attorno a qualsiasi contorno.
   *[other] { $count } linee chiuse: una contornatura o una tasca può tagliare attorno a esse, come attorno a qualsiasi contorno.
}
pipeline-image-dots = { $count ->
    [one] { $count } punto troppo piccolo per una linea mediana escluso.
   *[other] { $count } punti troppo piccoli per una linea mediana esclusi.
}
# $area is the smallest piece's area in mm², without its unit.
pipeline-image-specks = { $count ->
    [one] { $count } macchiolina più piccola di { $area } mm² scartata.
   *[other] { $count } macchioline più piccole di { $area } mm² scartate.
}
pipeline-image-black-at-border = Il nero tocca il bordo dell'immagine e viene chiuso lungo di esso; se è lo sfondo, inverti.
pipeline-image-without-arcs = { $count ->
    [one] { $count } forma lasciata come linee: i suoi archi arrivavano troppo vicino a un'altra forma.
   *[other] { $count } forme lasciate come linee: i loro archi arrivavano troppo vicino a un'altra forma.
}
pipeline-image-unsimplified = { $count ->
    [one] { $count } forma lasciata com'è stata vettorizzata: la levigatura la faceva incrociare.
   *[other] { $count } forme lasciate come sono state vettorizzate: la levigatura le faceva incrociare.
}
pipeline-no-text = non esiste il testo { $id }
# $text is the text's first line in quotes; $error why (a toolpath-error text).
pipeline-text-join-failed = { $text }: impossibile unire i contorni sovrapposti delle lettere: { $error }
# $text is the text's first line in quotes; $height its height; $width the box's width
# without its unit, $tall its height with it.
pipeline-text-wrote = Scritto { $text } in { $font }, alto { $height } ({ $width } × { $tall }): { $count ->
    [one] { $count } forma
   *[other] { $count } forme
} sul layer { $layer }.
# $missing are the characters the font has no letter for.
pipeline-text-missing = { $font } non ha una lettera per "{ $missing }": esclusa.
pipeline-text-dropped = { $count ->
    [one] { $count } contorno troppo piccolo da tagliare escluso.
   *[other] { $count } contorni troppo piccoli da tagliare esclusi.
}

## Errors: operations and tools

# `tool use` is a command of the command line: keep it as written.
pipeline-no-tool-use = il progetto non ha l'utensile { $tool }; copiane uno dalla tua libreria con `tool use`
pipeline-no-tool = il progetto non ha l'utensile { $tool }
# $number is the operation's number as listed, $count how many the project has.
pipeline-no-operation = non esiste la lavorazione { $number }; il progetto ne ha { $count }
pipeline-tab-no-ring = la lavorazione { $number } non ha contorni entro un diametro utensile da { $at }; `chipload op tabs` elenca i suoi ponticelli
pipeline-tab-none-there = la lavorazione { $number } non ha ponticelli entro un diametro utensile da { $at }; `chipload op tabs` elenca i suoi ponticelli
pipeline-tab-no-tabs = la lavorazione { $number } non ha ponticelli: solo una contornatura con ponticelli ne ha da posizionare
pipeline-corner-none-there = la lavorazione { $number } non ha un angolo in { $at }; `chipload op corners` elenca i suoi angoli
pipeline-corner-not-profile-or-pocket = la lavorazione { $number } non ha angoli da scegliere: solo contornature e tasche hanno dogbone
pipeline-not-a-pocket = la lavorazione { $number } non è una tasca; solo le tasche fanno ripresa dei residui
# $reason is one of the pipeline-rest-… texts.
pipeline-operation-refused = lavorazione { $number }: { $reason }
pipeline-rest-dogbones = ha dei dogbone, e dogbone e ripresa dei residui non si possono ancora combinare; gli angoli dogbone non richiedono ripresa
pipeline-rest-not-endmill = l'utensile { $tool } non è una fresa
pipeline-rest-before-not-endmill = l'utensile { $tool } che la precede non è una fresa
pipeline-rest-not-smaller = l'utensile { $tool } ({ $diameter }) non è più piccolo dell'utensile { $before } ({ $before_diameter }) che lo precede
# $dxfs lists the project's DXFs by number and name ("1 part-a, 2 part-b").
pipeline-update-which-dxf = il progetto ha { $count } DXF ({ $dxfs }): indica quale aggiornare
# `tool use` is a command of the command line: keep it as written.
pipeline-surface-no-endmill = il progetto non ha una fresa: una spianatura ne richiede una (copiane una con `tool use`)
# $type is the tool's type as written in the project file (drill, vbit).
pipeline-surface-not-endmill = l'utensile { $tool } è di tipo { $type }; una spianatura richiede una fresa

## Errors: drawn shapes

pipeline-shape-not-drawable = si possono disegnare solo cerchi, rettangoli e polilinee
# $kind is the shape's kind as written in the project file (circle, rectangle, polyline).
pipeline-shape-kind = la forma { $id } è di tipo { $kind }; il suo tipo non può cambiare
pipeline-no-shapes-given = nessuna forma indicata
pipeline-shape-twice = la forma { $id } è indicata due volte
pipeline-no-shape = non esiste la forma { $id }
pipeline-shape-imported-copy = la forma { $id } è stata importata dal DXF; solo le forme disegnate si possono copiare o specchiare
pipeline-shapes-no-extent = le forme non hanno estensione
pipeline-shape-imported-transform = la forma { $id } non è una forma disegnata: solo le forme disegnate si possono spostare, ruotare o scalare (esplodi prima il suo DXF)
pipeline-transform-not-numbers = spostamento, rotazione e centro devono essere numeri
pipeline-shapes-copy-imported = si possono copiare solo le forme disegnate, non quelle importate
# "Update drawing from DXF" is the command in the app's File menu.
pipeline-shape-imported-change = la forma { $id } è stata importata dal DXF e non si può cambiare qui; cambia il disegno e usa Aggiorna disegno da DXF

## Errors: parts and copies

pipeline-no-part = Nessun pezzo { $id } nel progetto
pipeline-part-name-empty = Il nome di un pezzo non può essere vuoto
pipeline-part-name-taken = Un altro pezzo si chiama "{ $name }"
pipeline-last-part = Un progetto richiede almeno un pezzo: questo è l'ultimo
pipeline-part-has-operations = Il pezzo { $name } ha { $count ->
    [one] { $count } lavorazione: spostala prima in un altro pezzo, o rimuovila insieme a esso
   *[other] { $count } lavorazioni: spostale prima in un altro pezzo, o rimuovile insieme a esso
}
pipeline-part-no-operation = Nessuna lavorazione { $number } nel progetto
pipeline-part-cuts-nothing = Il pezzo non taglia nulla
pipeline-copies-gap = La distanza deve essere 0 o più, trovato { $got }

## Errors: the simulation

# $min is a length without its unit, $max and $got with it.
pipeline-simulate-cell-size = la dimensione della cella deve essere tra { $min } e { $max }, trovato { $got }
# $cell is the size of one cell, a length.
pipeline-simulate-too-many-cells = una cella da { $cell } dà { $cells } celle per questo grezzo, più di { $most }; usa una cella più grande
pipeline-simulate-stock-too-large = il grezzo è troppo grande da simulare con celle da { $cell } (più di { $most } celle)
# $error is the PNG encoder's own words.
pipeline-simulate-png = PNG: { $error }

## Post-processors

pipeline-post-no-settings = nessuna cartella impostazioni, quindi nessun post "{ $name }"
pipeline-post-no-folder = Nessun post-processore "{ $name }": non è integrato e non c'è una cartella posts
# Why a project's copy of a user post can't be compared with its file.
pipeline-post-no-posts-folder = nessuna cartella posts
# What a post-processor is, one line each (Settings, This computer). $name, $extension,
# the paths and the units (mm, inch) and modes (files, program) are written as in its file.
pipeline-post-name = Nome: { $name }
pipeline-post-builtin = Integrato
pipeline-post-yours = Tuo
pipeline-post-settings = Impostazioni: { $path }
pipeline-post-template = Template: { $path }
pipeline-post-extension = Estensione file: { $extension }
pipeline-post-decimals = Decimali: { $mm } (mm), { $inch } (inch)
pipeline-post-units-list = Unità G-code: { $units }
pipeline-post-arcs = Archi: { $arcs ->
    [arcs] G2/G3
   *[lines] come brevi linee (G1)
}
pipeline-post-tool-changes = Più utensili: { $modes }
# $seconds is a time in seconds, without its unit.
pipeline-post-dwell = Sosta del mandrino: { $seconds } s
pipeline-post-no-dwell = Sosta del mandrino: nessuna

## The tool library and machine profiles

# $error is why the library file couldn't be read (a job-library-error text).
pipeline-library-defaults = { $error } (uso i valori predefiniti integrati)
pipeline-library-no-profile = nessun profilo macchina di nome "{ $name }"
# What a new project started with. $how is "profile" (asked for) or "default".
pipeline-library-machine = macchina: { $name } ({ $how ->
    [default] profilo predefinito
   *[profile] profilo
})
pipeline-library-tool = utensile { $tool }: { $name } (utensile predefinito)
# $error is why the post can't be used (a post-error text).
pipeline-library-post-unusable = post-processore "{ $name }": { $error }
pipeline-library-unknown-post = Macchina "{ $name }": questo Chipload non ha il post-processore "{ $post }"; i progetti che la usano non possono generare G-code finché non lo avrà

## Recovery copies

# $reason is the operating system's words, or the text below.
pipeline-recovery-io = Impossibile scrivere la copia di recupero { $path }: { $reason }
pipeline-recovery-locked = già bloccata

## Dragging a picture's or text's box

pipeline-box-turn-centre = la maniglia di rotazione è sul centro
pipeline-box-no-size = il riquadro non ha dimensione
# $min is the smallest size, a length.
pipeline-box-too-small = non può essere più piccolo di { $min }
pipeline-box-no-corner = un riquadro non ha l'angolo { $corner }

## G-code files

# One of several G-code files: $name is its file name, $operations the numbers of the
# operations it cuts ("1, 3").
pipeline-gcode-file = { $name } (utensile { $tool }, lavorazioni { $operations })

## Explode a DXF, join close ends (docs/specs/explode-and-join.md)

pipeline-dxf-exploded = DXF { $id } "{ $name }" esploso in { $count } { $count ->
    [one] forma disegnata
   *[other] forme disegnate
}. Il progetto non conosce più il file DXF.
# $shapes shape numbers ("S3, S4"); $tolerance a length.
pipeline-explode-split = { $count ->
    [one] La forma { $shapes } era in pezzi che non si toccano: ora ogni pezzo è una forma.
   *[other] Le forme { $shapes } erano in pezzi che non si toccano: ora ogni pezzo è una forma.
}
pipeline-explode-circle-start = { $count ->
    [one] Il cerchio { $shapes } ora inizia dal suo punto più a destra, in senso antiorario, come i cerchi disegnati: il suo taglio può iniziare altrove.
   *[other] I cerchi { $shapes } ora iniziano dal loro punto più a destra, in senso antiorario, come i cerchi disegnati: i loro tagli possono iniziare altrove.
}
pipeline-explode-dropped = { $count ->
    [one] La forma { $shapes } aveva solo segmenti più corti di { $tolerance } ed è stata eliminata.
   *[other] Le forme { $shapes } avevano solo segmenti più corti di { $tolerance } e sono state eliminate.
}
# $label is an operation's label.
pipeline-explode-selects = { $label } ora seleziona le forme { $shapes }.
pipeline-explode-no-dxf = il progetto non ha un DXF da esplodere
pipeline-explode-which-dxf = il progetto ha { $count } DXF ({ $dxfs }): indica quale esplodere
pipeline-join-gap-range = La distanza deve essere da { $least } a { $most }, trovato { $got }.
pipeline-join-imported = La forma S{ $id } viene da un DXF: esplodi prima il DXF.
pipeline-join-none = Nessun estremo unito (distanza { $gap }).
pipeline-joined = Unite { $pieces } forme in { $count }, { $closed } delle quali chiuse; distanza più larga chiusa { $widest }.
pipeline-joined-touching = Unite { $pieces } forme in { $count }, { $closed } delle quali chiuse; i loro estremi si toccavano.
pipeline-join-open = { $count } { $count ->
    [one] estremo resta aperto
   *[other] estremi restano aperti
}.
# $at a point, $nearest a length.
pipeline-join-open-end = L'estremo in { $at }: l'altro estremo più vicino è a { $nearest }.
pipeline-join-open-alone = L'estremo in { $at }: nessun altro estremo sul suo layer.
pipeline-join-branch = Tre o più estremi si incontrano in { $at }: lì non sono stati uniti.
pipeline-edit-imported = S{ $id } viene da un DXF, un'immagine o un testo: esplodi il DXF, o disegna la forma, per modificarla
pipeline-offset-distance = l'offset deve essere maggiore di 0, ricevuto { $got }
pipeline-offset-vanishes = di S{ $id } non resta nulla a { $distance } verso l'interno
pipeline-offset-side = S{ $id } è { $closed ->
    [closed] chiusa: fai l'offset all'esterno o all'interno
   *[open] aperta: fai l'offset a sinistra o a destra
}
pipeline-offset-failed = non è stato possibile fare l'offset di S{ $id }: { $why }
pipeline-new-shapes-cut = { $count ->
    [one] La nuova forma è
   *[other] Le { $count } nuove forme sono
} sul layer { $layer }, che { $ops ->
    [one] l'operazione { $operations } taglia
   *[other] le operazioni { $operations } tagliano
}.
pipeline-corner-none = S{ $id } non ha angoli da raccordare o smussare
pipeline-corner-not = l'angolo { $corner } di S{ $id } non è tra due lati diritti
pipeline-corner-straight = S{ $id } prosegue diritta in { $corner }: non c'è un angolo
pipeline-corner-too-large = è troppo grande per i lati di S{ $id }: al massimo { $most }
pipeline-corner-size = il raggio o la distanza deve essere maggiore di 0
