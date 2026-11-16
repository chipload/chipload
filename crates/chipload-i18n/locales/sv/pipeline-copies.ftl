# A part's instances, from a grid or placed one by one (crates/chipload-pipeline, docs/specs/instances.md and
# parts.md), as the command line and the setup sheet tell them.

# A part with its instances (docs/specs/instances.md): $count in all, the part itself
# counted; $turned and $mirrored how many of them are; $order one of the two below.
pipeline-instances = { $count } totalt{ $turned ->
    [0] {""}
   *[other] , { $turned } vridna
}{ $mirrored ->
    [0] {""}
   *[other] , { $mirrored } speglade
}, { $order }
pipeline-copies-by-operation = operation för operation
pipeline-instances-by-instance = instans för instans
# A part without instances: cut once.
pipeline-copies-one = en
# A part of the project, numbered from 1, with its name and its instances.
pipeline-copies-part = Detalj { $number } { $name }: { $copies }
# The same, for a part switched off.
pipeline-copies-part-off = Detalj { $number } { $name }: { $copies }, avstängd: fräses inte
