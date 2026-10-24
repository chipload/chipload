# The project's problems shown while editing: the Problems strip at the top of the
# properties panel, and the names of the fields they are about
# (crates/chipload-gui/src/app/problems.rs).

# The names of fields, put before a problem: "Feed: must be greater than 0, got 0".
# "Stock" is the material being machined.
problems-field-stock-x = Rohteil X
problems-field-stock-y = Rohteil Y
problems-field-thickness = Dicke
problems-field-max-overcut = Max. Tiefe unter Rohteil
problems-field-safe-z = Sicherheitshöhe
# The machine's travel: how far its axes can move.
problems-field-travel = Verfahrweg
problems-field-copies = Kopien
problems-field-part-name = Name des Teils
problems-field-parts = Teile
problems-field-tool-diameter = Werkzeugdurchmesser
problems-field-spindle-speed = Drehzahl
problems-field-vbit-angle = Winkel des V-Fräsers
# A V-bit's tip diameter.
problems-field-tip = Spitze
problems-field-feed = Vorschub
problems-field-plunge-feed = Eintauchvorschub
problems-field-start-depth = Starttiefe
problems-field-end-depth = Endtiefe
problems-field-step-down = Zustellung
problems-field-stepover = Seitl. Zustellung
problems-field-clearance = Übermaß
problems-field-entry = Eintauchen
problems-field-ramp-angle = Rampenwinkel
problems-field-lead = Anfahrt
problems-field-tool = Werkzeug
problems-field-part = Teil
# The side of the line a profile cuts on.
problems-field-side = Seite
# The shapes an operation cuts.
problems-field-geometry = Geometrie
problems-field-tabs = Haltestege
# $field: one of the names above; $problem: the problem's own words.
problems-with-field = { $field }: { $problem }

# The heading of the Problems strip.
problems-count = { $count ->
        [one] { $count } Problem
       *[other] { $count } Probleme
    }
problems-updates-when-fixed = Der Werkzeugweg wird aktualisiert, sobald sie behoben sind.
# Where a problem is: in operation 2, or in the job's settings.
problems-place-operation = Bearbeitung { $number }
problems-place-job = Job
# One line of the strip: $place is one of the two above.
problems-line = { $place } · { $problem }
problems-show-it-hover = Anzeigen
