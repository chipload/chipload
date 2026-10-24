# The project's problems shown while editing: the Problems strip at the top of the
# properties panel, and the names of the fields they are about
# (crates/chipload-gui/src/app/problems.rs).

# The names of fields, put before a problem: "Feed: must be greater than 0, got 0".
# "Stock" is the material being machined.
problems-field-stock-x = Brut X
problems-field-stock-y = Brut Y
problems-field-thickness = Épaisseur
problems-field-max-overcut = Prof. max sous le brut
problems-field-safe-z = Z de sécurité
# The machine's travel: how far its axes can move.
problems-field-travel = Course
problems-field-copies = Copies
problems-field-part-name = Nom de la pièce
problems-field-parts = Pièces
problems-field-tool-diameter = Diamètre de l'outil
problems-field-spindle-speed = Vitesse de broche
problems-field-vbit-angle = Angle de la fraise en V
# A V-bit's tip diameter.
problems-field-tip = Pointe
problems-field-feed = Avance
problems-field-plunge-feed = Avance plongée
problems-field-start-depth = Prof. de départ
problems-field-end-depth = Prof. finale
problems-field-step-down = Prof. de passe
problems-field-stepover = Pas latéral
problems-field-clearance = Surcote
problems-field-entry = Entrée
problems-field-ramp-angle = Angle de rampe
problems-field-lead = Approche
problems-field-tool = Outil
problems-field-part = Pièce
# The side of the line a profile cuts on.
problems-field-side = Côté
# The shapes an operation cuts.
problems-field-geometry = Géométrie
problems-field-tabs = Attaches
# $field: one of the names above; $problem: the problem's own words.
problems-with-field = { $field } : { $problem }

# The heading of the Problems strip.
problems-count = { $count ->
        [one] { $count } problème
       *[other] { $count } problèmes
    }
problems-updates-when-fixed = Le parcours d'outil se met à jour une fois qu'ils sont corrigés.
# Where a problem is: in operation 2, or in the job's settings.
problems-place-operation = Opération { $number }
problems-place-job = Travail
# One line of the strip: $place is one of the two above.
problems-line = { $place } · { $problem }
problems-show-it-hover = L'afficher
