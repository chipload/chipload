# A part's instances, from a grid or placed one by one (crates/chipload-pipeline, docs/specs/instances.md and
# parts.md), as the command line and the setup sheet tell them.

# A part with its instances (docs/specs/instances.md): $count in all, the part itself
# counted; $turned and $mirrored how many of them are; $order one of the two below.
pipeline-instances = { $count } in tutto{ $turned ->
    [0] {""}
   *[other] , { $turned } ruotate
}{ $mirrored ->
    [0] {""}
   *[other] , { $mirrored } specchiate
}, { $order }
pipeline-copies-by-operation = lavorazione per lavorazione
pipeline-instances-by-instance = istanza per istanza
# A part without instances: cut once.
pipeline-copies-one = una
# A part of the project, numbered from 1, with its name and its instances.
pipeline-copies-part = Pezzo { $number } { $name }: { $copies }
# The same, for a part switched off.
pipeline-copies-part-off = Pezzo { $number } { $name }: { $copies }, disattivato: non lavorato
