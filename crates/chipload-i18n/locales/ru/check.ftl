# Команды G-code и числовые параметры программы не переводятся.
check-unbalanced-comment = Незакрытый или вложенный комментарий
check-cannot-parse = Не удалось разобрать { $text }
check-after-end = Код после конца программы (M30)
check-absolute-arc-centres = G90.1 (абсолютные координаты центров дуг) не поддерживается
check-not-supported = { $word } не поддерживается
check-word-twice = { $letter } встречается в строке дважды
check-unsupported-word = Неподдерживаемое слово { $letter }
check-several-motions = Несколько команд движения в одной строке
check-units-changed = { $code } меняет единицы на { $units ->
    [inch] дюймы
   *[mm] мм
} внутри программы
check-units-differ = { $code } ({ $units ->
    [inch] дюймы
   *[mm] мм
}), но единицы G-code станка — { $machine ->
    [inch] дюймы
   *[mm] мм
}
check-feed-not-positive = Подача F{ $feed } должна быть положительной
check-relative = G91 (относительные координаты) не поддерживается
check-tool-not-whole = T{ $tool } должен быть целым номером инструмента
check-start-without-speed = Шпиндель запущен после смены инструмента без задания оборотов (S)
check-dwell-without-p = Выдержка G4 без P
check-blend-and-dwell = G64 и G4 в одной строке: неоднозначный параметр P
check-blend-too-far = G64 P{ $p } допускает отклонение контроллера от траектории более чем на { $most }
check-blend-without-p = G64 без P разрешает неограниченное скругление углов контроллером (используйте P не более { $most })
check-no-motion-mode = Координаты осей без режима движения
check-change-without-tool = Смена инструмента (M6) без номера инструмента (T)
check-change-unknown-tool = Смена инструмента (M6) на T{ $tool }, которого нет в проекте
check-change-spindle-on = Смена инструмента (M6) при включённом шпинделе (сначала остановите его командой M5)
check-change-below-safe = Смена инструмента (M6) ниже безопасной высоты (сначала выполните отвод)
check-motion-before-units = Движение до G21 (мм) или G20 (дюймы)
check-motion-before-absolute = Движение до G90 (абсолютные координаты)
check-rapid-below-safe = Быстрый ход XY при Z ниже безопасной высоты (сначала выполните отвод)
check-rapid-into-material = Быстрый ход к Z{ $z }, ниже верха заготовки (быстрый ход в материал)
check-cut-without-feed = Рабочий ход без заданной подачи
check-cut-spindle-off = Рабочий ход при выключенном шпинделе
check-cut-unknown-position = Рабочий ход из неизвестного положения
check-below-stock-too-far = Z{ $z } ниже низа заготовки ({ $bottom }) более чем на допустимые { $allowed }
check-arc-without-centre = Дуга без смещений центра I/J
check-arc-unknown-position = Дуга из неизвестного положения
check-arc-full-circle = Дуга заканчивается в начальной точке (полная окружность одним ходом)
check-arc-radius-mismatch = Несовпадение радиусов дуги: в начале { $start }, в конце { $end }
check-below-stock = { $count ->
    [one] { $count } ход проходит
    [few] { $count } хода проходят
   *[other] { $count } ходов проходят
} ниже низа заготовки (самая низкая Z{ $deepest }, низ { $bottom }), в пределах допуска { $allowed }
check-outside-stock = { $count ->
    [one] { $count } рабочий ход заканчивается
    [few] { $count } рабочих хода заканчиваются
   *[other] { $count } рабочих ходов заканчиваются
} за пределами заготовки (расширенными на радиус инструмента)
check-no-travel = Ходы осей станка не заданы; пределы перемещений не проверялись
check-travel-exceeded = Размах по { $axis } { $span } превышает ход оси станка { $travel }
check-no-end = Нет конца программы (M30)
