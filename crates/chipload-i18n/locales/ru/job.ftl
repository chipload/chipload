job-problem-at = { $path }: { $problem }
job-operation-problem = { $label }: { $problem }
job-shape-problem = { $at } (S{ $id }).{ $problem }
job-operation-label = Операция { $number }
job-operation-label-named = Операция { $number } ({ $name })
job-operation-label-rest = Операция { $number } (доработка остатка)
job-operation-label-named-rest = Операция { $number } ({ $name }, доработка остатка)
job-operation-label-floor = Операция { $number } (дно)
job-operation-label-named-floor = Операция { $number } ({ $name }, дно)
job-operation-label-roughing = Операция { $number } (черновая)
job-operation-label-named-roughing = Операция { $number } ({ $name }, черновая)
job-operation-label-smoothing = Операция { $number } (сглаживание)
job-operation-label-named-smoothing = Операция { $number } ({ $name }, сглаживание)
job-tool-type = { $type ->
    [drill] сверло
    [ballnose] сферическая фреза
    [vbit] V-образная фреза
   *[endmill] концевая фреза
}
job-tool-type-article = { $type ->
    [drill] сверло
    [ballnose] сферическая фреза
    [vbit] V-образная фреза
   *[endmill] концевая фреза
}
job-must-be-positive = должно быть больше 0, получено { $got }
job-must-be-positive-number = должно быть положительным числом, получено { $got }
job-vcarve-passes = должно быть целым числом от 1 до { $most }, получено { $got }
job-vcarve-step-down = должно быть не меньше { $least }, чтобы проходов было не больше { $most }, получено { $got }
job-vcarve-ignore-kinks = должно быть от 0 до { $most } градусов, получено { $got }
job-vcarve-plug-start = должно быть больше начальной глубины ({ $start }) и меньше конечной ({ $end }), получено { $got }
job-vcarve-plug-kinks = должно быть 0 у вставки инкрустации, чтобы она входила в свой карман в каждом углу, получено { $got }
job-must-be-more-than-0 = должно быть больше 0, получено { $got }
job-must-be-0-or-more = должно быть не меньше 0, получено { $got }
job-through-stale = сквозная обработка, поэтому конечная глубина — { $depth }
job-stock-fit-needs-centred = заготовка по размеру чертежа должна быть расположена по его центру (stock.origin: drawing)
job-must-be-1-or-more = должно быть не меньше 1
job-must-be-at-least-1 = должно быть не меньше 1
job-must-not-be-empty = не должно быть пустым
job-must-be-finite = должно быть конечным числом
job-must-be-finite-got = должно быть конечным числом, получено { $got }
job-must-be-number = должно быть числом, получено { $got }
job-must-be-numbers = должны быть числами, получено ({ $x }, { $y })
job-xy-must-be-numbers = x и y должны быть числами, получено ({ $x }, { $y })
job-id-twice = { $id } используется дважды
job-frame-shape-twice = фигура { $id } входит в две рамки
job-more-than-0-at-most = должно быть больше 0 и не больше { $most }, получено { $got }
job-rotation-range = должно быть не меньше 0 и меньше 360 градусов, получено { $got }
job-angle-below-180 = должно быть больше 0 и меньше 180 градусов, получено { $got } градусов
job-ramp-angle-range = должно быть больше 0 и не больше 30 градусов, получено { $got } градусов
job-flank-angle-range = должно быть больше 0 и не больше 90 градусов, получено { $got } градусов
job-copies-per-side = должно быть от 1 до { $most }, получено { $got }
job-copies-too-many = всего не больше { $most } копий, получено { $got }
job-copies-pitch-zero = не может быть 0 при нескольких копиях в этом направлении
job-image-range-px = должно быть от { $low } до { $high } px, получено { $got } px
job-image-range-area = должно быть от { $low } до { $high } mm², получено { $got } mm²
job-image-range-length = должно быть от { $low } до { $high }, получено { $got }
job-image-threshold = должно быть от 1 до 254 или «auto», получено { $got }
job-image-missing = изображение отсутствует
job-model-missing = файл модели отсутствует
job-model-keep = должно быть больше 0 и не больше 1 (доля модели, сохраняемая сверху), получено { $got }
job-text-empty = пусто: введите текст для обработки
job-text-height = должно быть больше 0 и не больше { $most }, получено { $got }
job-text-spacing = должно быть не больше { $heights } высот в любую сторону ({ $most }), получено { $got }
job-shape-id-range = должно быть от 1 до next_shape_id - 1 ({ $most }), получено { $got }
job-no-image = нет изображения { $id } в drawing.images
job-no-model = нет модели { $id } в drawing.models
job-relief-stepover = должно быть не больше { $most } инструмента, а не { $got }
job-relief-allowance = должно быть 0 или больше, а не { $got }
job-relief-smoothing-alone = проход сглаживания идёт по поверхности за один раз, поэтому перед ним нужен включённый черновой проход рельефа; добавьте или включите черновой проход либо выключите сглаживание
job-relief-ramp = должен быть от { $least }° до 90°, а не { $got }°
job-relief-rim-corner = должен быть от 0 до ширины бортика ({ $width }), а не { $got }
job-no-dxf = нет DXF { $id } в drawing.dxfs
job-no-text = нет текста { $id } в drawing.texts
job-no-group = нет группы { $id } в drawing.groups
job-group-empty = группа должна содержать хотя бы один объект или текст
job-group-not-drawn = объект { $shape } не является начерченным: группировать можно только начерченные объекты и тексты
job-group-shape-twice = объект { $shape } входит в две группы
job-group-text-twice = текст { $text } входит в две группы
job-shape-one-source = объект может происходить только из одного изображения, DXF или текста
job-path-no-segments = объект должен содержать хотя бы один сегмент
job-segment-not-finite = координаты должны быть конечными, радиусы — положительными
job-corner-radius-tiny = 0 для острых углов или не меньше { $least }, получено { $got }
job-corner-radius-too-large = не больше { $most }, половины короткой стороны, получено { $got }
job-polyline-points = Для { $closed ->
    [closed] замкнутой
   *[open] разомкнутой
} полилинии нужно хотя бы { $least } точек, получено { $got }
job-point-on-previous = совпадает с предыдущей точкой (ближе { $tolerance })
job-bulges-count = по одному на сегмент, для этой полилинии нужно { $wanted }, получено { $got }
job-bulge-range = должно быть от -1 до 1 (не больше полуокружности), получено { $got }
job-work-zero-not-finite = x и y должны быть конечными числами, получено { $x }, { $y }
job-tip-diameter = должно быть не меньше 0 и меньше диаметра ({ $diameter }), получено { $got }
job-tapered-ball-needs-tip = конической сферической фрезе нужен диаметр шарика на кончике (tip_diameter)
job-no-parts = в проекте должна быть хотя бы одна деталь
job-no-setups = в проекте нет установа; нужен хотя бы один
job-setup-twice = идентификатор установа { $id } используется дважды
job-no-such-setup = нет установа { $id } в setups
job-back-of-itself = установ не может быть своей же обратной стороной
job-back-of-a-back = установ { $id } сам является обратной стороной: обратную сторону нельзя перевернуть ещё раз
job-two-backs = у установа { $id } две обратные стороны: у заготовки одна обратная сторона
job-dowels-no-back = отверстия под штифты сверлятся с лицевой стороны заготовки, у которой есть обратная сторона, а у установа { $id } её нет
job-dowels-not-through = отверстия под штифты проходят сквозь заготовку в жертвенный стол: задайте «Ниже заготовки» равным глубине штифтов
job-dowel-not-circle = объект { $shape } не нарисованная окружность: отверстие под штифт должно ею быть
job-dowel-off-axis = отверстие под штифт { $shape } смещено на { $off } от линии, вокруг которой переворачивают заготовку: на перевёрнутой заготовке оно не попадёт на свой штифт
job-part-twice = деталь { $id } указана дважды
job-part-name-taken = другая деталь уже называется «{ $name }»
job-no-part = нет детали { $id } в parts
job-part-order = все операции детали { $part } должны предшествовать операциям детали { $before }
job-scale-range = должно быть от { $low } до { $high }, получено { $got }
job-start-below-end = должно быть меньше end_depth ({ $end }), получено { $got }
job-on-line-clearance = у обработки по линии нет стороны для перереза; должно быть 0, получено { $got }
job-surface-stepover = должно быть от { $low } до { $high } диаметра инструмента, получено { $got }
job-surface-too-deep = торцевание не может идти ниже низа заготовки (толщина { $thickness }), получено { $got }
job-selector-empty = селектор должен задавать DXF, изображение, текст, слой или геометрию
job-exclude-no-source = исключать объекты можно только из целого DXF, изображения или текста
job-exclude-missing = объекта { $shape } больше нет в { $source ->
    [dxf] DXF
    [image] изображении
   *[text] тексте
} { $id }: заново выберите исключаемые объекты
job-selector-one-source = селектор задаёт только один DXF, изображение или текст
job-select-letter = объект { $shape } — буква текста и меняется при каждом его пересоздании: выберите весь текст ({ "{" }"text": { $text }{ "}" })
job-select-traced = объект { $shape } получен векторизацией и меняется при каждой повторной векторизации: выберите изображение ({ "{" }"image": { $image }{ "}" })
job-select-centre-lines = изображение { $image } векторизовано разомкнутыми средними линиями: используйте обработку вдоль линии или векторизуйте границы
job-on-line-corners = у обработки по линии нет внутренних углов для разгрузки
job-tbone-wall = для T-bone нужно задать стенку: длинную или короткую
job-wall-only-tbone = стенка задаётся только для T-bone
job-corner-chosen-twice = две настройки угла в { $at }
job-starts-not-finite = каждая начальная точка должна иметь конечные x и y
job-stepover-at-most-1 = должно быть не больше 1 (доля диаметра инструмента), получено { $got }
job-on-line-finish = у обработки по линии нет стенки для чистового прохода; выберите внутри или снаружи
job-skin-above-step-down = должно быть не больше глубины за проход ({ $step_down }), получено { $got }
job-skin-above-cut = должно быть меньше глубины резания ({ $cut }, конечная глубина минус начальная), получено { $got }
job-skin-above-tabs = должно быть меньше высоты перемычки ({ $height }), получено { $got }
job-semicolons = { $list }; { $last }
job-commas = { $list }, { $last }
job-error-version = Версия schema_version { $found } задания не поддерживается: эта версия Chipload читает от { $oldest } до { $current }. Обновите Chipload или проверьте файл.
job-error-too-old = Задание schema_version { $found } создано до появления проектов и больше не читается. Откройте и сохраните его в Chipload 0.3.0 или заново импортируйте DXF.
job-error-cannot-migrate = Невозможно открыть задание (schema_version { $from }): { $reason }
job-error-unreadable = Не удалось прочитать файл задания { $path }: { $reason }
job-error-invalid = Некорректный файл задания { $path }: { $reason }
job-error-not-object = должен содержать объект JSON
job-error-cannot-write = невозможно записать: { $error }
job-error-no-backup = невозможно сохранить предыдущую версию как .bak: { $error }
job-set-would-not-load = проект не загрузится после задания { $paths }: { $reason }
job-set-back-follows-front = { $path } нельзя изменить: { $back } — это { $front }, перевёрнутая, с общей заготовкой и нулём детали. Измените { $instead }.
job-library-error-version = { $path }: schema_version { $found } не поддерживается: эта версия Chipload читает { $current }. Обновите Chipload или проверьте файл.
job-library-error-unreadable = Не удалось прочитать { $path }: { $reason }
job-library-error-invalid = Некорректный файл { $path }: { $reason }
job-library-error-unwritable = Не удалось записать { $path }: { $reason }
job-library-error-refused = { $reason }
job-library-no-tool = нет инструмента с именем «{ $name }»
job-library-no-machine = нет станка с именем «{ $name }»
job-library-no-material = нет материала с именем «{ $name }»
job-library-material-exists = материал с именем «{ $name }» уже существует
job-library-no-entry = Нет { $entry ->
    [machine] станка
   *[tool] инструмента
} с именем «{ $name }»
job-library-other-file = { $path }: { $hint }
job-library-holds-machines = файл содержит профили станков, не инструменты: импортируйте как станки
job-library-holds-tools = файл содержит инструменты, не профили станков: импортируйте как инструменты
job-library-change = { $what } { $from } -> { $to }
job-library-field = { $field ->
    [type] тип
    [diameter] диаметр
    [flutes] число зубьев
    [centre-cutting] режущий центр
    [rpm] обороты
    [angle] угол
    [tip] вершина
    [id] id
    [cutting-values] режимы резания
    [post] постпроцессор
    [G-code-units] единицы G-code
    [tool-changes] смена инструмента
    [safe-Z] безопасная Z
    [spindle] шпиндель
    [max-feed] макс. подача
    [travel] ходы осей
    [work-zero] ноль детали
    [max-overcut] макс. глубина под заготовкой
    [tool-values] параметры инструмента
    [style-tool] инструмент
    [style-settings] операция
   *[other] { $field }
}
job-library-degrees = { $degrees }°
job-library-none = нет
job-library-yes-no = { $value ->
    [yes] да
    [no] нет
   *[unknown] неизвестно
}
job-library-travel = X{ $x } Y{ $y } Z{ $z }
job-library-work-zero-origin = начало координат чертежа
job-library-work-zero-stock = заготовка { $corner }
job-library-work-zero-point = точка X{ $x } Y{ $y }
job-library-cutting = паз { $slot_feed } / { $slot_step_down }, выборка { $feed } / { $step_down } × { $stepover }, врезание { $plunge }
job-library-values-moved = { $material }: значения для { $tool } теперь его режимы резания для { $material }
job-library-values-dropped = { $material }: значения для инструмента, которого нет в библиотеке (uid { $uid }), удалены
job-library-other-values = другие параметры
job-shapes-no-entry = в библиотеке геометрии нет такой записи
job-shapes-bad-name = «{ $name }» не может быть именем: от 1 до { $most } символов, без / \ : * ? " < > |, не начинается с точки или пробела
job-library-tool-there = инструмент { $id } (уже в проекте)
job-library-tool-added = инструмент { $id } (добавлен)
job-library-tool-added-as = инструмент { $id } (добавлен как { $id }; номер { $taken } занят)
job-link-none = не из вашей библиотеки
job-link-missing-named = { $entry ->
    [profile] Профиля
   *[tool] Инструмента
} «{ $name }» нет в библиотеке
job-link-missing = { $entry ->
    [profile] Исходного профиля
   *[tool] Исходного инструмента
} нет в библиотеке
job-link-same = Из { $entry ->
    [profile] профиля
   *[tool] инструмента библиотеки
} «{ $name }»
job-link-differs = Отличается от { $entry ->
    [profile] профиля
   *[tool] инструмента библиотеки
} «{ $name }»: { $changes }
job-import-empty = В файле нет { $list ->
    [machines] станков
   *[tools] инструментов
}.
job-import-added = Добавлено: { $names }
job-import-new = Новое: { $names }
job-import-same = Уже в библиотеке: { $names }
job-import-replaced = Заменено: { $name } (файл: { $differences })
job-import-kept = Сохранены ваши: { $name } (файл: { $differences })
job-import-differs = Отличается от ваших: { $name } (файл: { $differences })
job-import-default = По умолчанию: { $name }
job-import-becomes-default = Станет значением по умолчанию: { $name }
job-polygon-sides = многоугольник должен иметь от { $least } до { $most } сторон, получено { $got }

job-style-unreadable = стиль «{ $name }» не удаётся прочитать: { $reason }
job-style-exists = стиль с именем «{ $name }» уже есть (замените его или выберите другое имя)
job-style-none = нет стиля с именем «{ $name }»
job-style-no-operation = в проекте нет операции { $number }
job-style-tool-not-in-library = инструмента { $tool } нет в вашей библиотеке: сначала сохраните его в библиотеку
job-style-these-settings = эти настройки
job-style-other-settings = другие настройки
