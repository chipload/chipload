pipeline-post-units = постпроцессор { $post } записывает G-code только в мм (в конфигурации нет «{ $units }» в units)
pipeline-drawing-outside-stock = Чертёж выходит за пределы заготовки
pipeline-copies-beyond-stock = Копии ({ $count }) обрабатывают X{ $x0 } … { $x1 }, Y{ $y0 } … { $y1 }, за пределами заготовки ({ $stock_x } × { $stock_y }); проверьте заготовку или копии детали.
pipeline-part-copies-beyond-stock = Деталь { $part }: копии ({ $count }) обрабатывают X{ $x0 } … { $x1 }, Y{ $y0 } … { $y1 }, за пределами заготовки ({ $stock_x } × { $stock_y }); проверьте заготовку или копии детали.
pipeline-layer-clash = { $label }: слой «{ $layer }» есть в { $sources }; обрабатываются все эти источники. Ограничьте одним (выберите слой под нужным DXF в дереве проекта)
pipeline-and = { $list } и { $last }
pipeline-source-named = { $name }
pipeline-source-dxf = DXF { $id }
pipeline-source-image = изображение { $id }
pipeline-source-drawn = начерченная геометрия
pipeline-duration-seconds = { $seconds } с
pipeline-duration-minutes = { $minutes } мин { $seconds } с
pipeline-duration-hours = { $hours } ч { $minutes } мин
pipeline-estimate = Расчётное время: { $total } (резание { $cutting }, быстрые ходы { $rapids }, шпиндель { $spindle }){ $changes ->
    [0] {""}
    [one] , плюс { $changes } смена инструмента
    [few] , плюс { $changes } смены инструмента
   *[other] , плюс { $changes } смен инструмента
}
pipeline-estimate-part = {"  "}Деталь { $number } ({ $name }): { $time }
pipeline-estimate-operation = {"  "}{ $label }: { $time }
pipeline-estimate-part-operation = {"    "}{ $label }: { $time }
pipeline-estimate-file = {"  "}Файл { $number }: { $time }
pipeline-estimate-rapids-assumed = {"  "}При заданных подачах (быстрые ходы приняты за { $rate }; задайте максимальную подачу станка для точности); разгон и углы увеличивают реальное время.
pipeline-estimate-rapids = {"  "}При заданных подачах (быстрые ходы на максимальной подаче станка, { $rate }); разгон и углы увеличивают реальное время.
pipeline-chip-load-title = Подача на зуб (при рабочей подаче):
pipeline-chip-load-line = {"  "}{ $label }: { $load }
pipeline-chip-load = { $load }
pipeline-chip-load-no-flutes = задайте число зубьев инструмента для расчёта подачи на зуб
pipeline-chip-load-no-rpm = задайте обороты шпинделя инструмента для расчёта подачи на зуб
pipeline-chip-load-no-tool = нет инструмента
pipeline-chip-load-hint = Подача на зуб: толщина стружки, снимаемой каждой режущей кромкой, подача / (обороты шпинделя × число зубьев). При слишком малой инструмент трёт и греется, при слишком большой может сломаться. Зависит от материала, инструмента и станка; грубый ориентир для дерева и пластика с инструментом { $small_tool }–{ $large_tool }: { $wood_low }–{ $wood_high }, для алюминия: { $aluminium_low }–{ $aluminium_high }. Сверяйтесь с данными производителя инструмента.
pipeline-report-units = Единицы: { $units } (источник: { $from ->
    [dxf] dxf
    [svg] svg
   *[user] пользователь
})
pipeline-report-extents = Габариты: X { $x0 } .. { $x1 }, Y { $y0 } .. { $y1 } ({ $width } x { $height })
pipeline-report-imported = Импортировано: { $items }
pipeline-report-flattened = Приведено к плоскости: { $items }
pipeline-report-ignored = Пропущено: { $items }
pipeline-report-no-issues = Замечаний нет
pipeline-report-issues = Замечания ({ $count }):
pipeline-report-issue = {"  "}[{ $severity ->
    [warning] предупреждение
   *[error] ошибка
}] { $issue }
pipeline-report-contours = Контуры ({ $count }):
pipeline-report-contour = {"  "}№{ $id } { $state } слой { $layers } X { $x0 } .. { $x1 }  Y { $y0 } .. { $y1 }
pipeline-report-contour-state = { $state ->
    [open] разомкн.
    [invalid] ОШИБКА
    [hole] отверстие
   *[outer] внешний
}
pipeline-check-line = строка { $line }: { $message }
pipeline-check-file = файл { $file }: { $message }
pipeline-check-file-line = файл { $file }, строка { $line }: { $message }
pipeline-error-check-failed = Проверка G-code не пройдена:
pipeline-error-check-finding =
    { $list }
    {"  "}{ $finding }
pipeline-text = { $text }
pipeline-stock-fit = Чертёж (X{ $x0 } … { $x1 }, Y{ $y0 } … { $y1 }) выходит за пределы заготовки ({ $width } × { $length }); проверьте заготовку установа.
pipeline-place-which-dxf = в проекте { $count } DXF ({ $dxfs }): укажите, какой разместить
pipeline-no-dxf = нет DXF { $id }
pipeline-place-nothing-imported = в проекте нет импортированного чертежа для перемещения
pipeline-place-the-drawing = импортированный чертёж
pipeline-place-cannot-move = невозможно переместить чертёж на ({ $dx }, { $dy })
pipeline-place-already-there = Чертёж уже находится здесь; ничего не перемещено.
pipeline-place-turned = Поворот: { $what }, { $degrees }° { $direction ->
    [ccw] против часовой стрелки
   *[cw] по часовой стрелке
}
pipeline-place-reset = Восстановлено исходное положение DXF: { $what }
pipeline-place-scaled = Масштабирование: { $what }, ×{ $factor }
pipeline-place-scaled-size = { $scaled } (теперь { $width } × { $height })
pipeline-place-moved = Перемещение: { $what }, на X{ $dx } Y{ $dy }
pipeline-place-done = { $done }; теперь { $placement }.
pipeline-place-moved-starts = Вместе перемещено начальных точек: { $count }.
pipeline-place-moved-corners = Вместе перемещено настроек углов: { $count }.
pipeline-place-moved-tabs = Вместе перемещено вручную установленных перемычек: { $count }.
pipeline-place-drawn-stay = Начерченная в Chipload геометрия остаётся на месте.
pipeline-place-other-dxfs-stay = Остальные DXF остаются на месте.
pipeline-place-work-zero-stays = Ноль детали остаётся в точке X{ $x }, Y{ $y }; проверьте, что это нужное положение.
pipeline-place-margin = поле должно быть не меньше 0, получено { $got }
pipeline-place-stock-follows = заготовка расположена по центру чертежа и следует за ним, поэтому выравнивание ничего не меняет: сначала выберите положение заготовки у начала координат чертежа
pipeline-cut-outline = Габариты реза: { $width } × { $length } (X { $x0 } … { $x1 }, Y { $y0 } … { $y1 }, с учётом инструмента): минимальная заготовка для задания.
pipeline-place-rotation = угол поворота должен быть числом в градусах, получено { $got }
pipeline-place-scale = масштаб должен быть числом больше 0, получено { $got }
pipeline-place-scale-range = это даст масштаб чертежа ×{ $factor } относительно DXF; допустимо от ×{ $min } до ×{ $max }
pipeline-place-size = { $what ->
    [height] Высота
   *[width] Ширина
} должна быть больше 0, получено { $got }
pipeline-place-no-size = У импортированного чертежа нет { $what ->
    [height] высоты
   *[width] ширины
} для масштабирования
pipeline-placement-as-dxf = как в DXF
pipeline-placement-from-dxf = { $parts } относительно DXF
pipeline-placement-and = { $list }, { $last }
pipeline-placement-scaled = масштаб ×{ $factor }
pipeline-placement-turned = поворот { $degrees }°
pipeline-placement-moved = смещение X{ $dx } Y{ $dy }
pipeline-scale-curves = Объекты из сплайнов или эллипсов ({ $count }) были аппроксимированы с точностью { $within } в прежнем размере; после масштаба ×{ $factor } отклонение от кривых может достигать { $off }. Обновите из DXF для повторной аппроксимации в новом размере.
pipeline-scale-contours = Замкнутых контуров было { $was }, теперь { $now }: в новом размере зазоры соединяются в пределах { $within }. Проверьте отчёт импорта и выбранную геометрию операций.
pipeline-scale-units = Если единицы DXF неверны, задайте их при обновлении (--units), чтобы проект сохранил их.
pipeline-dxf-added = Добавлен { $file } как { $kind } { $id } «{ $name }»: { $count } { $count ->
    [one] объект
    [few] объекта
   *[other] объектов
}.
pipeline-dxf-pinned = { $count ->
    [one] Операция { $operations } продолжает обрабатывать только { $name }: в новом DXF тоже есть её слои.
   *[other] Операции { $operations } продолжают обрабатывать только { $name }: в новом DXF тоже есть их слои.
}
pipeline-dxf-removed = Удалён { $kind } { $id } «{ $name }» и { $count } { $count ->
    [one] его объект
    [few] его объекта
   *[other] его объектов
}.
pipeline-dxf-shapes-gone = { $label }: объектов { $shapes } больше нет; выберите геометрию заново
pipeline-dxf-gone = { $label }: исходный DXF удалён; выберите геометрию заново
pipeline-dxf-updated = Обновлено из { $file }: без изменений { $unchanged }, изменено { $changed }, добавлено { $added }, удалено { $removed }.
pipeline-dxf-placed-as-before = Размещено как прежде: { $placement }.
pipeline-dxf-geometry-changed = { $label }: геометрия изменилась (объекты { $shapes }); проверьте траекторию
pipeline-dxf-layer-updated = { $label }: слой { $layer } теперь использует обновлённый чертёж; проверьте траекторию
pipeline-left-out-found = Операция { $number }: повторно { $count ->
    [one] найден { $count } исключённый объект
    [few] найдены { $count } исключённых объекта
   *[other] найдены { $count } исключённых объектов
}.
pipeline-left-out-lost = Операция { $number }: { $count ->
    [one] { $count } исключённый объект не найден
    [few] { $count } исключённых объекта не найдены
   *[other] { $count } исключённых объектов не найдены
} повторно: заново выберите исключаемую геометрию.
pipeline-no-image = нет изображения { $id }
pipeline-image-png-failed = запись изображения: { $error }
pipeline-image-unreadable = drawing.images: не удалось прочитать изображение { $id }: { $error }
pipeline-image-traced = Векторизовано { $file } ({ $width } × { $height }, порог { $threshold }{ $picked ->
    [yes] , выбран автоматически
   *[no] {""}
}): { $count } { $lines ->
    [yes] { $count ->
        [one] средняя линия
        [few] средние линии
       *[other] средних линий
    }
   *[no] { $count ->
        [one] объект
        [few] объекта
       *[other] объектов
    }
} на слое { $layer }.
pipeline-image-nothing-black = Нет чёрных областей: измените порог или включите инверсию, если изображение светлое на тёмном фоне.
pipeline-image-strokes = Ширина штрихов от { $narrow } до { $wide }: используйте обработку вдоль линии.
pipeline-image-closed-lines = { $count ->
    [one] { $count } замкнутая линия: можно обработать контуром или карманом, как любую границу.
    [few] { $count } замкнутые линии: можно обработать контуром или карманом, как любые границы.
   *[other] { $count } замкнутых линий: можно обработать контуром или карманом, как любые границы.
}
pipeline-image-dots = Пропущено { $count } { $count ->
    [one] точка
    [few] точки
   *[other] точек
}, слишком мелких для средней линии.
pipeline-image-specks = Удалено { $count } { $count ->
    [one] вкрапление
    [few] вкрапления
   *[other] вкраплений
} меньше { $area } mm².
pipeline-image-black-at-border = Чёрная область касается края изображения и замыкается по нему; если это фон, включите инверсию.
pipeline-image-without-arcs = { $count ->
    [one] { $count } объект оставлен отрезками: дуги слишком близко подходили к другой геометрии.
    [few] { $count } объекта оставлены отрезками: дуги слишком близко подходили к другой геометрии.
   *[other] { $count } объектов оставлены отрезками: дуги слишком близко подходили к другой геометрии.
}
pipeline-image-unsimplified = { $count } { $count ->
    [one] объект оставлен
    [few] объекта оставлены
   *[other] объектов оставлены
} без упрощения: сглаживание вызвало пересечения.
pipeline-no-text = нет текста { $id }
pipeline-text-join-failed = { $text }: не удалось объединить перекрывающиеся границы букв: { $error }
pipeline-text-font-missing = Шрифта «{ $font }» нет на этом компьютере. Геометрия букв и обработка текста { $text } сохранены; для изменения содержимого или размера выберите другой шрифт.
pipeline-text-no-font = На этом компьютере нет шрифта «{ $font }». Встроенные: { $fonts }; `chipload text fonts` также показывает шрифты компьютера.
pipeline-text-moved = Перемещён { $text }: его { $count } { $count ->
    [one] буква без изменений
    [few] буквы без изменений
   *[other] букв без изменений
} (шрифт отсутствует на этом компьютере)
pipeline-text-wrote = Создан текст { $text }, шрифт { $font }, высота { $height } ({ $width } × { $tall }): { $count } { $count ->
    [one] объект
    [few] объекта
   *[other] объектов
} на слое { $layer }.
pipeline-text-missing = В { $font } нет символов «{ $missing }»: пропущены.
pipeline-text-dropped = Пропущено { $count } { $count ->
    [one] контур
    [few] контура
   *[other] контуров
}, слишком мелких для обработки.
pipeline-no-tool-use = в проекте нет инструмента { $tool }; скопируйте из библиотеки командой `tool use`
pipeline-no-tool = в проекте нет инструмента { $tool }
pipeline-no-operation = нет операции { $number }; всего в проекте { $count }
pipeline-tab-no-ring = у операции { $number } нет контура в пределах диаметра инструмента от { $at }; `chipload op tabs` выводит её перемычки
pipeline-tab-none-there = у операции { $number } нет перемычки в пределах диаметра инструмента от { $at }; `chipload op tabs` выводит её перемычки
pipeline-tab-no-tabs = у операции { $number } нет перемычек: разместить их можно только при обработке контура с перемычками
pipeline-corner-none-there = у операции { $number } нет угла в { $at }; `chipload op corners` выводит её углы
pipeline-corner-not-profile-or-pocket = у операции { $number } нет настраиваемых углов: разгрузки dogbone есть только у контуров и карманов
pipeline-no-setup = в проекте нет установа { $id }
pipeline-setup-name-empty = установу нужно имя
pipeline-last-setup = в проекте должен быть хотя бы один установ
pipeline-setup-has-parts = в установе { $name } деталей: { $count }; сначала переместите их в другой установ или удалите вместе с ним
pipeline-no-setup-cut = нет включённых установов с геометрией для обработки
pipeline-not-a-pocket = операция { $number } не является карманом; доработка остатка есть только у карманов
pipeline-operation-refused = операция { $number }: { $reason }
pipeline-rest-dogbones = есть разгрузки dogbone, пока несовместимые с доработкой остатка; углы с dogbone не требуют доработки
pipeline-rest-not-endmill = инструмент { $tool } не является концевой фрезой
pipeline-not-a-vcarve = операция { $number } не является V-гравировкой; обработка дна есть только у V-гравировки
pipeline-floor-not-endmill = инструмент { $tool } не является концевой фрезой, необходимой для обработки дна
pipeline-rest-before-not-endmill = предыдущий инструмент { $tool } не является концевой фрезой
pipeline-rest-not-smaller = инструмент { $tool } ({ $diameter }) не меньше предыдущего инструмента { $before } ({ $before_diameter })
pipeline-update-which-dxf = в проекте { $count } DXF ({ $dxfs }): укажите, какой обновить
pipeline-surface-no-endmill = в проекте нет концевой фрезы для торцевания (скопируйте командой `tool use`)
pipeline-surface-not-endmill = инструмент { $tool } — { $type }; для торцевания нужна концевая фреза
pipeline-shape-not-drawable = можно начертить только окружности, прямоугольники и полилинии
pipeline-shape-kind = объект { $id } имеет тип { $kind }; изменить тип нельзя
pipeline-no-shapes-given = геометрия не задана
pipeline-shape-twice = объект { $id } задан дважды
pipeline-no-shape = нет объекта { $id }
pipeline-shape-imported-copy = объект { $id } импортирован из DXF; копировать и отражать можно только начерченную геометрию
pipeline-shapes-no-extent = у геометрии нет габаритов
pipeline-shape-imported-transform = объект { $id } не является начерченным: перемещение, поворот и масштаб доступны только для начерченных объектов (сначала разбейте DXF)
pipeline-construction-imported = объект { $id } не является начерченным: только начерченные объекты могут быть вспомогательными (сначала разбейте DXF)
pipeline-construction-used = { $label } выбирает вспомогательную геометрию, которая не обрабатывается: выберите другие объекты для выполнения.
pipeline-transform-not-numbers = смещение, угол поворота и центр должны быть числами
pipeline-shapes-copy-imported = копировать можно только начерченную геометрию, не импортированную
pipeline-shape-imported-change = объект { $id } импортирован из DXF и здесь не редактируется; измените чертёж и используйте «Обновить чертёж из DXF»
pipeline-no-part = В проекте нет детали { $id }
pipeline-part-name-empty = Имя детали не может быть пустым
pipeline-part-name-taken = Другая деталь уже называется «{ $name }»
pipeline-last-part = В проекте должна быть хотя бы одна деталь: это последняя
pipeline-part-has-operations = У детали { $name } { $count ->
    [one] { $count } операция
    [few] { $count } операции
   *[other] { $count } операций
}: сначала переместите их в другую деталь или удалите вместе с ней
pipeline-part-no-operation = В проекте нет операции { $number }
pipeline-part-cuts-nothing = Деталь ничего не обрабатывает
pipeline-copies-gap = Зазор должен быть не меньше 0, получено { $got }
pipeline-simulate-cell-size = размер ячейки должен быть от { $min } до { $max }, получено { $got }
pipeline-simulate-too-many-cells = при размере { $cell } для этой заготовки получается { $cells } ячеек, больше { $most }; увеличьте размер ячейки
pipeline-simulate-stock-too-large = заготовка слишком велика для симуляции с ячейками { $cell } (больше { $most } ячеек)
pipeline-simulate-png = PNG: { $error }
pipeline-post-no-settings = нет папки настроек, поэтому нет постпроцессора «{ $name }»
pipeline-post-no-folder = Нет постпроцессора «{ $name }»: он не встроенный, а папка posts отсутствует
pipeline-post-no-posts-folder = нет папки posts
pipeline-post-name = Имя: { $name }
pipeline-post-builtin = Встроенный
pipeline-post-yours = Ваш
pipeline-post-settings = Настройки: { $path }
pipeline-post-template = Шаблон: { $path }
pipeline-post-extension = Расширение файла: { $extension }
pipeline-post-decimals = Знаков после точки: { $mm } (мм), { $inch } (дюймы)
pipeline-post-units-list = Единицы G-code: { $units }
pipeline-post-arcs = Дуги: { $arcs ->
    [arcs] G2/G3
   *[lines] короткими отрезками (G1)
}
pipeline-post-tool-changes = Несколько инструментов: { $modes }
pipeline-post-dwell = Выдержка шпинделя: { $seconds } с
pipeline-post-no-dwell = Выдержка шпинделя: нет
pipeline-library-defaults = { $error } (используются встроенные значения по умолчанию)
pipeline-library-no-profile = нет профиля станка с именем «{ $name }»
pipeline-library-machine = станок: { $name } ({ $how ->
    [default] профиль по умолчанию
   *[profile] профиль
})
pipeline-library-tool = инструмент { $tool }: { $name } (по умолчанию)
pipeline-library-material = материал: { $name } (по умолчанию)
pipeline-library-post-unusable = постпроцессор «{ $name }»: { $error }
pipeline-library-unknown-post = Станок «{ $name }»: в этой версии Chipload нет постпроцессора «{ $post }»; проекты с ним не смогут создать G-code, пока он не появится
pipeline-material-chosen = Материал: { $name }
pipeline-material-none = Материал { $name } больше не выбран; подачи не изменены
pipeline-material-op-set = Операция { $op } ({ $name }): подачи из { $material }
pipeline-material-op-kept = Операция { $op } ({ $name }): для инструмента { $tool } нет режимов в { $material }; подачи не изменены
pipeline-material-tool-not-in-library = Инструмента { $tool } нет в библиотеке: сначала сохраните его туда (tool save), затем можно сохранить его режимы в материале
pipeline-recovery-io = Не удалось записать копию для восстановления { $path }: { $reason }
pipeline-recovery-locked = уже заблокировано
pipeline-box-turn-centre = маркер поворота находится в центре
pipeline-box-no-size = рамка нулевого размера
pipeline-box-too-small = размер не может быть меньше { $min }
pipeline-box-no-corner = у рамки нет угла { $corner }
pipeline-gcode-file = { $name } (инструмент { $tool }, операции { $operations })
pipeline-dxf-exploded = { $kind } { $id } «{ $name }» разбит на { $count } { $count ->
    [one] начерченный объект
    [few] начерченных объекта
   *[other] начерченных объектов
}. Связь проекта с файлом { $kind } удалена.
pipeline-explode-split = { $count ->
    [one] Объект { $shapes } состоял из несоединённых фрагментов: теперь каждый — отдельный объект.
   *[other] Объекты { $shapes } состояли из несоединённых фрагментов: теперь каждый — отдельный объект.
}
pipeline-explode-circle-start = { $count ->
    [one] Окружность { $shapes } теперь начинается в правой точке и идёт против часовой стрелки, как начерченные окружности: рез может начинаться в другом месте.
   *[other] Окружности { $shapes } теперь начинаются в правых точках и идут против часовой стрелки, как начерченные окружности: резы могут начинаться в других местах.
}
pipeline-explode-dropped = { $count ->
    [one] Объект { $shapes } содержал только сегменты короче { $tolerance } и удалён.
   *[other] Объекты { $shapes } содержали только сегменты короче { $tolerance } и удалены.
}
pipeline-explode-selects = { $label } теперь выбирает объекты { $shapes }.
pipeline-explode-no-dxf = в проекте нет DXF для разбиения
pipeline-explode-which-dxf = в проекте { $count } DXF ({ $dxfs }): укажите, какой разбить
pipeline-join-gap-range = Зазор должен быть от { $least } до { $most }, получено { $got }.
pipeline-join-imported = Объект S{ $id } из DXF: сначала разбейте DXF.
pipeline-join-none = Концы не соединены (зазор { $gap }).
pipeline-joined = Объединено объектов: { $pieces } в { $count }, из них замкнутых { $closed }; наибольший закрытый зазор { $widest }.
pipeline-joined-touching = Объединено объектов: { $pieces } в { $count }, из них замкнутых { $closed }; концы совпадали.
pipeline-join-open = { $count } { $count ->
    [one] конец остаётся разомкнутым
    [few] конца остаются разомкнутыми
   *[other] концов остаются разомкнутыми
}.
pipeline-join-open-end = Конец в { $at }: ближайший другой конец на расстоянии { $nearest }.
pipeline-join-open-alone = Конец в { $at }: других концов на этом слое нет.
pipeline-join-branch = В { $at } сходятся три или более концов: здесь не соединено.
pipeline-edit-imported = S{ $id } из DXF, изображения или текста: разбейте DXF или начертите объект для редактирования
pipeline-offset-distance = расстояние смещения должно быть больше 0, получено { $got }
pipeline-offset-vanishes = на { $distance } внутрь от S{ $id } ничего не остаётся
pipeline-offset-side = S{ $id } { $closed ->
    [closed] замкнут: смещайте наружу или внутрь
   *[open] разомкнут: смещайте влево или вправо
}
pipeline-offset-failed = Не удалось сместить S{ $id }: { $why }
pipeline-new-shapes-cut = { $count ->
    [one] Новый объект находится
    [few] { $count } новых объекта находятся
   *[other] { $count } новых объектов находятся
} на слое { $layer }, который { $ops ->
    [one] обрабатывает операция { $operations }
   *[other] обрабатывают операции { $operations }
}.
pipeline-corner-none = у S{ $id } нет углов для скругления или фаски
pipeline-corner-not = угол { $corner } объекта S{ $id } не образован двумя прямыми сторонами
pipeline-corner-straight = S{ $id } идёт прямо в { $corner }: угла нет
pipeline-corner-too-large = слишком велико для сторон S{ $id }: не больше { $most }
pipeline-corner-size = радиус или расстояние должны быть больше 0
pipeline-trim-nothing-crosses = ничто не пересекает S{ $id } здесь (замкнутому объекту нужны два пересечения): удалите его целиком
pipeline-trim-everything = обрезка S{ $id } здесь удалит его целиком: используйте удаление
pipeline-extend-not-straight = можно продлить только прямой конец S{ $id }
pipeline-extend-nothing-ahead = впереди этого конца S{ $id } ничего нет
pipeline-extend-closed = S{ $id } замкнут: нет конца для продления
pipeline-break-at-end = это конец S{ $id }: разрывать нечего
pipeline-combine-two = для объединения нужны хотя бы два объекта
pipeline-combine-open = S{ $id } разомкнут: объединять можно только замкнутые объекты
pipeline-combine-empty = { $op ->
    [subtract] от S{ $id } ничего не осталось: другие объекты покрывают его целиком
   *[other] у объектов нет общей области: ничего не осталось
}
pipeline-combine-failed = не удалось объединить объекты: { $why }
pipeline-array-count = массив должен содержать от 2 до { $most } объектов с учётом исходного, получено { $got }
pipeline-array-spacing = копии совпадут: задайте ненулевой шаг, если столбцов или строк больше одного
pipeline-array-angle = угол должен быть больше 0 и не больше 360 градусов, получено { $got }
pipeline-align-two = для выравнивания нужны хотя бы два объекта или заготовка как ориентир
pipeline-distribute-three = для распределения нужны хотя бы три объекта
pipeline-shape-gone = S{ $id } удалён: { $ops ->
    [one] использовался в операции { $operations }; выберите её геометрию заново
   *[other] использовался в операциях { $operations }; выберите их геометрию заново
}.
pipeline-export-nothing = нечего экспортировать: в этой области нет видимой геометрии
pipeline-export-no-layer = на чертеже нет слоя { $layer }
pipeline-export-unverified = DXF не записан: при повторном чтении S{ $id } изменился ({ $why })
pipeline-export-summary = { $count ->
    [one] { $count } примитив
    [few] { $count } примитива
   *[other] { $count } примитивов
} на { $layers ->
    [one] { $layers } слое
   *[other] { $layers } слоях
}, в { $units ->
    [inch] дюймах
   *[mm] мм
}.
pipeline-export-renamed = Слой { $from } записан в DXF как { $to }: часть символов недопустима в имени слоя DXF.
pipeline-frame-borders = у рамки не больше { $most } окантовок, получено { $got }
pipeline-frame-gap = зазор между окантовками должен быть больше 0, получено { $got }
pipeline-frame-margin = поле должно быть не меньше 0, получено { $got }
pipeline-frame-nothing = нечего обрамлять: ни одного из этих объектов нет на чертеже
pipeline-frame-size = ширина и высота рамки должны быть больше 0
pipeline-frame-r = размер угла этой рамки не больше { $largest }
pipeline-frame-no-fit = рамка этого типа не помещается вокруг этой геометрии
pipeline-frame-borders-fit = { $count ->
    [one] окантовка с отступом { $gap } внутрь не помещается в рамке
   *[other] окантовки ({ $count }) с шагом { $gap } не помещаются в рамке
}
