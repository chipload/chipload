# A part's instances, from a grid or placed one by one (crates/chipload-pipeline, docs/specs/instances.md and
# parts.md), as the command line and the setup sheet tell them.

# A part with its instances (docs/specs/instances.md): $count in all, the part itself
# counted; $turned and $mirrored how many of them are; $order one of the two below.
pipeline-instances = { $count } in all{ $turned ->
    [0] {""}
   *[other] , { $turned } turned
}{ $mirrored ->
    [0] {""}
   *[other] , { $mirrored } mirrored
}, { $order }
pipeline-copies-by-operation = operation by operation
pipeline-instances-by-instance = instance by instance
# A part without instances: cut once.
pipeline-copies-one = one
# A part of the project, numbered from 1, with its name and its instances.
pipeline-copies-part = Part { $number } { $name }: { $copies }
# The same, for a part switched off.
pipeline-copies-part-off = Part { $number } { $name }: { $copies }, off: not cut
