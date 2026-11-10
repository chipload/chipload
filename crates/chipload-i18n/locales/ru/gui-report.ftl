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
report-place-dowels = Разместить отверстия под штифты заново
report-place-dowels-hover = Переместить две окружности туда, где им место сейчас, в стороне от резов обеих сторон, одним шагом отмены. Не после обработки лицевой стороны: её отверстия уже просверлены
report-part = Деталь { $number } · { $name }
report-file = Файл { $number }
