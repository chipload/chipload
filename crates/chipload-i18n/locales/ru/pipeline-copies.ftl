# A part with its instances (docs/specs/instances.md): $count in all, the part itself
# counted; $turned and $mirrored how many of them are; $order one of the two below.
pipeline-instances = всего { $count }{ $turned ->
    [0] {""}
   *[other] , повёрнуто { $turned }
}{ $mirrored ->
    [0] {""}
   *[other] , отражено { $mirrored }
}, { $order }
pipeline-copies-by-operation = по операциям
pipeline-instances-by-instance = по экземплярам
pipeline-copies-one = одна
pipeline-copies-part = Деталь { $number } { $name }: { $copies }
pipeline-copies-part-off = Деталь { $number } { $name }: { $copies }, выключена: не обрабатывается
