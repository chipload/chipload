report-heading = Отчёт
report-details = Подробности ({ $count })
report-drawing-title = Чертёж: { $contours ->
    [one] { $contours } контур
    [few] { $contours } контура
   *[other] { $contours } контуров
}, { $issues ->
    [0] нет замечаний
    [one] { $issues } замечание
    [few] { $issues } замечания
   *[other] { $issues } замечаний
}
report-select-operation-hover = Выбрать операцию { $number }
report-indented = {"  "}{ $line }
report-select-tool-hover = Открыть инструмент T{ $tool }
report-update-tools-hover = Дать этим инструментам значения из библиотеки, одним шагом отмены
report-part = Деталь { $number } · { $name }
report-file = Файл { $number }
