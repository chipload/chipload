# Copies of a part in a grid (crates/chipload-pipeline, docs/specs/copies-grid.md and
# parts.md), as the command line and the setup sheet tell them.

# A part with its instances (docs/specs/instances.md): $count in all, the part itself
# counted; $turned and $mirrored how many of them are; $order one of the two below.
pipeline-instances = { $count } insgesamt{ $turned ->
    [0] {""}
   *[other] , { $turned } gedreht
}{ $mirrored ->
    [0] {""}
   *[other] , { $mirrored } gespiegelt
}, { $order }
pipeline-copies-by-operation = Bearbeitung für Bearbeitung
pipeline-instances-by-instance = Instanz für Instanz
# A part without a grid: one copy.
pipeline-copies-one = eine
# A part of the project, numbered from 1, with its name and its copies.
pipeline-copies-part = Teil { $number } { $name }: { $copies }
# The same, for a part switched off.
pipeline-copies-part-off = Teil { $number } { $name }: { $copies }, aus: wird nicht gefräst
