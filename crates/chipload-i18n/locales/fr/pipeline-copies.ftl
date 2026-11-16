# Copies of a part in a grid (crates/chipload-pipeline, docs/specs/copies-grid.md and
# parts.md), as the command line and the setup sheet tell them.

# A part with its instances (docs/specs/instances.md): $count in all, the part itself
# counted; $turned and $mirrored how many of them are; $order one of the two below.
pipeline-instances = { $count } en tout{ $turned ->
    [0] {""}
   *[other] , { $turned } tournées
}{ $mirrored ->
    [0] {""}
   *[other] , { $mirrored } en miroir
}, { $order }
pipeline-copies-by-operation = opération par opération
pipeline-instances-by-instance = instance par instance
# A part without a grid: one copy.
pipeline-copies-one = une
# A part of the project, numbered from 1, with its name and its copies.
pipeline-copies-part = Pièce { $number } { $name } : { $copies }
# The same, for a part switched off.
pipeline-copies-part-off = Pièce { $number } { $name } : { $copies }, désactivée : non usinée
