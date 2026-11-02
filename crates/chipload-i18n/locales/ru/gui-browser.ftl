browser-caption = Дерево проекта
browser-no-project = Нет открытого проекта.
browser-new-project = Новый проект
browser-eye-hide-hover = Скрыть в рабочей области; обработка остаётся включённой
browser-eye-show-hover = Скрыто в рабочей области (обрабатывается): показать
browser-tick-on-hover = Обрабатывается: снимите флажок, чтобы исключить из программы
browser-tick-off-hover = Не обрабатывается: установите флажок, чтобы включить обработку
browser-link-changed = { $name } (изменён)
browser-link-missing = (нет в библиотеке)
browser-profile = Обработка контура
browser-profile-hover = Обработка выделенных контуров
browser-pocket = Выборка кармана
browser-pocket-hover = Выборка кармана по выделенным контурам
browser-vcarve = V-гравировка
browser-vcarve-hover = V-образная фреза вдоль средней линии выделенных замкнутых контуров; глубина подбирается по ширине фигуры
browser-follow-path = Вдоль линии
browser-follow-path-hover = Инструмент идёт по линии выделенных объектов, замкнутых или разомкнутых: гравировка, пазы и надрезы
browser-holes = Отверстия
browser-holes-hover = Выделенные окружности: сверление сверлом или фрезерование по винтовой траектории концевой фрезой
browser-surface = Торцевание
browser-surface-hover = Выравнивание всей поверхности заготовки широкой концевой фрезой: обработка жертвенного стола или торцевание заготовки. Выделение не требуется
browser-job-settings = Станок
browser-job-settings-hover = Станок: постпроцессор, единицы G-code, безопасная Z, ходы осей и смена инструмента; общие для всех установов
browser-work-zero = Ноль детали
browser-work-zero-hover = Положение X0 Y0 Z0 в G-code. Нажмите для настройки или выделите и перетащите в рабочей области.
browser-job-settings-problems-hover = Станок, постпроцессор, единицы и ходы осей. { $count ->
    [one] { $count } проблема
    [few] { $count } проблемы
   *[other] { $count } проблем
}
browser-tools = Инструменты
browser-tools-hover = Инструменты проекта: нажмите, чтобы просмотреть все и добавить новый
browser-setups = Установы
browser-setups-hover = Заготовки проекта, обрабатываемые по очереди: у каждой своя программа, размеры и ноль детали. Нажмите, чтобы сделать установ активным
browser-add-setup = Добавить установ
browser-add-setup-hover = Другая заготовка: собственные размеры, ноль детали и детали, обрабатываемые отдельной программой
browser-setup-parts = { $count ->
    [one] { $count } деталь
    [few] { $count } детали
   *[other] { $count } деталей
}
browser-setup-active-hover = Активный установ: показан в полном цвете, рассчитывается и симулируется. Нажмите для настройки заготовки; ниже — её ноль детали
browser-setup-hover = Нажмите, чтобы сделать установ активным: для отображения, симуляции и расчёта
browser-move-to-setup = Переместить в установ
browser-export-setup = Экспортировать G-code этого установа…
browser-export-setup-hover = Программа и карта наладки с суффиксом -S1, -S2 при нескольких установах; установ становится активным и при необходимости сначала рассчитывается
browser-new-part = Новая деталь
browser-part-active = активна
browser-part-active-hover = Активная деталь: новые операции добавляются сюда. Нажмите для настройки копий; правая кнопка — дополнительные действия
browser-part-hover = Нажмите для настройки копий; двойной щелчок — сделать активной; правая кнопка — дополнительные действия
browser-activate = Активировать
browser-activate-hover = Новые операции добавляются в активную деталь
browser-export-part = Экспортировать G-code этой детали…
browser-export-part-hover = Записать G-code только этой детали, как если бы остальные были выключены; также карту наладки, если это включено в настройках.
browser-new-operation = Новая операция
browser-select-shapes-first = Сначала выделите геометрию
browser-move-up = Выше
browser-move-down = Ниже
browser-delete-part = { $count ->
    [0] Удалить деталь
    [one] Удалить деталь и { $count } её операцию
    [few] Удалить деталь и { $count } её операции
   *[other] Удалить деталь и { $count } её операций
}
browser-delete-part-disabled-hover = В проекте должна быть хотя бы одна деталь
browser-part-empty-active = Выделите геометрию и добавьте операцию
browser-part-empty = Нет операций: перетащите сюда операцию
browser-no-name = (без имени)
browser-op-hidden = скрыто: { $count }
browser-duplicate = Дублировать
browser-add-rest-pass = Добавить доработку остатка
browser-rest-project-tool = T{ $tool } · концевая фреза { $size }
browser-rest-library-tool = { $name } · концевая фреза { $size }
browser-from-library = Из библиотеки инструментов
browser-copied-into-project-hover = Копируется в инструменты проекта
browser-add-rest-pass-hover = Меньший инструмент выбирает только остаток после инструмента кармана, в его геометрии и глубинах. Выполняется после этого и соседних карманов с тем же инструментом
browser-make-rest-pass = Сделать доработкой остатка операции { $number }
browser-make-rest-pass-hover = Копирует этот карман: доработка следует за ним и выполняется после соседних карманов с тем же инструментом
browser-delete = Удалить
browser-rest = Доработка
browser-rest-pass-hover = Доработка остатка: выбирает материал после предыдущего инструмента, в геометрии и глубинах кармана, после соседних карманов с тем же инструментом
browser-add-floor-pass = Добавить обработку дна
browser-add-floor-pass-hover = Концевая фреза выбирает дно между стенками штрихов, слишком широких для V-гравировки, в её геометрии и глубинах. Выполняется перед V-гравировкой
browser-floor = Дно
browser-floor-pass-hover = Обработка дна: концевая фреза выбирает дно между стенками перед V-гравировкой; V-образная фреза дочищает недоступные места
browser-remove = Удалить
browser-op-rest-after = доработка после T{ $tool }
browser-op-not-cut = Не обрабатывается (флажок снят)
browser-op-hidden-hover = Обрабатывает { $count } { $count ->
    [one] скрытый контур
    [few] скрытых контура
   *[other] скрытых контуров
}: включите их в разделе «Чертёж», чтобы увидеть
browser-op-drag-hover = Перетащите для изменения порядка; правая кнопка — дополнительные действия
browser-selection = Выделение
browser-copy-as-selector = Копировать как селектор
browser-copy-as-selector-hover = Выделение в формате JSON для операции в файле проекта
browser-tool-change = смена инструмента → T{ $tool }
