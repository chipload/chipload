# Copies of a part in a grid (crates/chipload-pipeline, docs/specs/copies-grid.md and
# parts.md), as the command line and the setup sheet tell them.

# $columns × $rows copies; $dx × $dy the distance from copy to copy, numbers in $unit
# ("mm" or "in"); $order one of the two below.
pipeline-copies-grid = { $columns } × { $rows } copies, pas { $dx } × { $dy } { $unit }, { $order }
pipeline-copies-by-operation = opération par opération
pipeline-copies-by-copy = copie par copie
# A part without a grid: one copy.
pipeline-copies-one = une
# A part of the project, numbered from 1, with its name and its copies.
pipeline-copies-part = Pièce { $number } { $name } : { $copies }
# The same, for a part switched off.
pipeline-copies-part-off = Pièce { $number } { $name } : { $copies }, désactivée : non usinée
