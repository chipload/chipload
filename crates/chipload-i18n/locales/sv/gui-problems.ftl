# The project's problems shown while editing: the Problems strip at the top of the
# properties panel, and the names of the fields they are about
# (crates/chipload-gui/src/app/problems.rs).

# The names of fields, put before a problem: "Feed: must be greater than 0, got 0".
# "Stock" is the material being machined.
problems-field-stock-x = Ämne X
problems-field-stock-y = Ämne Y
problems-field-thickness = Tjocklek
problems-field-max-overcut = Max djup under ämnet
problems-field-safe-z = Säker Z
# The machine's travel: how far its axes can move.
problems-field-travel = Slaglängd
problems-field-copies = Kopior
problems-field-part-name = Detaljnamn
problems-field-parts = Detaljer
problems-field-tool-diameter = Verktygsdiameter
problems-field-spindle-speed = Spindelvarvtal
problems-field-vbit-angle = V-fräsens vinkel
# A V-bit's tip diameter.
problems-field-tip = Spets
problems-field-feed = Matning
problems-field-plunge-feed = Nedmatning
problems-field-start-depth = Startdjup
problems-field-end-depth = Slutdjup
problems-field-step-down = Skärdjup
problems-field-stepover = Sidosteg
problems-field-clearance = Förskjutning
problems-field-entry = Nedgång
problems-field-ramp-angle = Rampvinkel
problems-field-lead = Inkörning
problems-field-tool = Verktyg
problems-field-part = Detalj
# The side of the line a profile cuts on.
problems-field-side = Sida
# The shapes an operation cuts.
problems-field-geometry = Geometri
problems-field-tabs = Bryggor
# $field: one of the names above; $problem: the problem's own words.
problems-with-field = { $field }: { $problem }

# The heading of the Problems strip.
problems-count = { $count ->
        [one] { $count } problem
       *[other] { $count } problem
    }
problems-updates-when-fixed = Verktygsbanan uppdateras när de är åtgärdade.
# Where a problem is: in operation 2, or in the job's settings.
problems-place-operation = Operation { $number }
problems-place-job = Jobb
# One line of the strip: $place is one of the two above.
problems-line = { $place } · { $problem }
problems-show-it-hover = Visa det
