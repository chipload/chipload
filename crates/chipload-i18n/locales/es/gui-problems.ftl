# The project's problems shown while editing: the Problems strip at the top of the
# properties panel, and the names of the fields they are about
# (crates/chipload-gui/src/app/problems.rs).

# The names of fields, put before a problem: "Feed: must be greater than 0, got 0".
# "Stock" is the material being machined.
problems-field-stock-x = Material X
problems-field-stock-y = Material Y
problems-field-thickness = Espesor
problems-field-max-overcut = Prof. máx. bajo el material
problems-field-safe-z = Z segura
# The machine's travel: how far its axes can move.
problems-field-travel = Recorrido
problems-field-copies = Copias
problems-field-part-name = Nombre de la pieza
problems-field-parts = Piezas
problems-field-tool-diameter = Diámetro de la herramienta
problems-field-spindle-speed = Vel. del husillo
problems-field-vbit-angle = Ángulo de la fresa en V
# A V-bit's tip diameter.
problems-field-tip = Punta
problems-field-feed = Avance
problems-field-plunge-feed = Avance de penetración
problems-field-start-depth = Prof. inicial
problems-field-end-depth = Prof. final
problems-field-step-down = Prof. de pasada
problems-field-stepover = Paso lateral
problems-field-clearance = Holgura
problems-field-entry = Entrada
problems-field-ramp-angle = Ángulo de rampa
problems-field-lead = Aproximación
problems-field-tool = Herramienta
problems-field-part = Pieza
# The side of the line a profile cuts on.
problems-field-side = Lado
# The shapes an operation cuts.
problems-field-geometry = Geometría
problems-field-tabs = Puentes
# $field: one of the names above; $problem: the problem's own words.
problems-with-field = { $field }: { $problem }

# The heading of the Problems strip.
problems-count = { $count ->
        [one] { $count } problema
       *[other] { $count } problemas
    }
problems-updates-when-fixed = La trayectoria se actualiza en cuanto se corrijan.
# Where a problem is: in operation 2, or in the job's settings.
problems-place-operation = Operación { $number }
problems-place-job = Trabajo
# One line of the strip: $place is one of the two above.
problems-line = { $place } · { $problem }
problems-show-it-hover = Mostrarlo
