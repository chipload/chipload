# The project's problems shown while editing: the Problems strip at the top of the
# properties panel, and the names of the fields they are about
# (crates/chipload-gui/src/app/problems.rs).

# The names of fields, put before a problem: "Feed: must be greater than 0, got 0".
# "Stock" is the material being machined.
problems-field-stock-x = Grezzo X
problems-field-stock-y = Grezzo Y
problems-field-thickness = Spessore
problems-field-max-overcut = Prof. max sotto grezzo
problems-field-safe-z = Z sicura
# The machine's travel: how far its axes can move.
problems-field-travel = Corse
problems-field-copies = Copie
problems-field-part-name = Nome pezzo
problems-field-parts = Pezzi
problems-field-tool-diameter = Diametro utensile
problems-field-spindle-speed = Velocità mandrino
problems-field-vbit-angle = Angolo fresa a V
# A V-bit's tip diameter.
problems-field-tip = Punta
problems-field-feed = Avanzamento
problems-field-plunge-feed = Affondamento
problems-field-start-depth = Prof. iniziale
problems-field-end-depth = Prof. finale
problems-field-step-down = Prof. di passata
problems-field-stepover = Passo laterale
problems-field-clearance = Maggiorazione
problems-field-entry = Ingresso
problems-field-ramp-angle = Angolo rampa
problems-field-lead = Raccordo
problems-field-tool = Utensile
problems-field-part = Pezzo
# The side of the line a profile cuts on.
problems-field-side = Lato
# The shapes an operation cuts.
problems-field-geometry = Geometria
problems-field-tabs = Ponticelli
# $field: one of the names above; $problem: the problem's own words.
problems-with-field = { $field }: { $problem }

# The heading of the Problems strip.
problems-count = { $count ->
        [one] { $count } problema
       *[other] { $count } problemi
    }
problems-updates-when-fixed = Il percorso utensile si aggiorna quando sono risolti.
# Where a problem is: in operation 2, or in the job's settings.
problems-place-operation = Lavorazione { $number }
problems-place-job = Lavoro
# One line of the strip: $place is one of the two above.
problems-line = { $place } · { $problem }
problems-show-it-hover = Mostralo
