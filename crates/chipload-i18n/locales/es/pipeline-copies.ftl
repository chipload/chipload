# Copies of a part in a grid (crates/chipload-pipeline, docs/specs/copies-grid.md and
# parts.md), as the command line and the setup sheet tell them.

# A part with its instances (docs/specs/instances.md): $count in all, the part itself
# counted; $turned and $mirrored how many of them are; $order one of the two below.
pipeline-instances = { $count } en total{ $turned ->
    [0] {""}
   *[other] , { $turned } giradas
}{ $mirrored ->
    [0] {""}
   *[other] , { $mirrored } en espejo
}, { $order }
pipeline-copies-by-operation = operación por operación
pipeline-instances-by-instance = instancia por instancia
# A part without a grid: one copy.
pipeline-copies-one = una
# A part of the project, numbered from 1, with its name and its copies.
pipeline-copies-part = Pieza { $number } { $name }: { $copies }
# The same, for a part switched off.
pipeline-copies-part-off = Pieza { $number } { $name }: { $copies }, desactivada: no se corta
