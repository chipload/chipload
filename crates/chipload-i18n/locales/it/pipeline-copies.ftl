# Copies of a part in a grid (crates/chipload-pipeline, docs/specs/copies-grid.md and
# parts.md), as the command line and the setup sheet tell them.

# $columns × $rows copies; $dx × $dy the distance from copy to copy, numbers in $unit
# ("mm" or "in"); $order one of the two below.
pipeline-copies-grid = { $columns } × { $rows } copie, passo { $dx } × { $dy } { $unit }, { $order }
pipeline-copies-by-operation = lavorazione per lavorazione
pipeline-copies-by-copy = copia per copia
# A part without a grid: one copy.
pipeline-copies-one = una
# A part of the project, numbered from 1, with its name and its copies.
pipeline-copies-part = Pezzo { $number } { $name }: { $copies }
# The same, for a part switched off.
pipeline-copies-part-off = Pezzo { $number } { $name }: { $copies }, disattivato: non lavorato
