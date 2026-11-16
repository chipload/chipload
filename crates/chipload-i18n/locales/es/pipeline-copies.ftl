# A part's instances, from a grid or placed one by one (crates/chipload-pipeline, docs/specs/instances.md and
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
# A part without instances: cut once.
pipeline-copies-one = una
# A part of the project, numbered from 1, with its name and its instances.
pipeline-copies-part = Pieza { $number } { $name }: { $copies }
# The same, for a part switched off.
pipeline-copies-part-off = Pieza { $number } { $name }: { $copies }, desactivada: no se corta
