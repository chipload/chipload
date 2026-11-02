toolpath-no-operations = В задании нет операций
toolpath-all-disabled = Все операции отключены: нечего обрабатывать
toolpath-tool-undefined = Инструмент { $tool } не задан в проекте
toolpath-no-rpm-short = обороты шпинделя не заданы
toolpath-no-rpm = У инструмента { $tool } не заданы обороты шпинделя (об/мин)
toolpath-safe-z-short = должна быть выше верха заготовки (Z { $top })
toolpath-safe-z = safe_z { $safe } должна быть выше верха заготовки (Z { $top }); z_zero — '{ $zero }'
toolpath-too-deep = конечная глубина { $end } на { $below } ниже заготовки (толщина { $thickness }), больше допуска { $allowed } (макс. глубина под заготовкой)
toolpath-clearance-radius = перерез { $clearance } должен быть меньше радиуса инструмента { $radius }
toolpath-finish-radius = чистовой припуск { $stock } должен быть меньше радиуса инструмента { $radius }
toolpath-tab-height = высота перемычки { $height } должна быть меньше глубины резания { $cut } (конечная глубина минус начальная)
toolpath-max-feed = { $field } { $feed } превышает max_feed станка { $max }
toolpath-helix-for-path = винтовое врезание применяется в карманах; для обработки вдоль линии доступно наклонное (entry: ramp)
toolpath-helix-radius = радиус винтовой линии { $radius } должен быть меньше радиуса инструмента { $tool_radius }, иначе в центре останется стержень
toolpath-lead-radius = радиус подвода { $radius } больше { $most } × диаметр инструмента ({ $longest })
toolpath-surface-endmill = инструмент { $tool } — { $type }; для торцевания нужна концевая фреза
toolpath-vcarve-vbit = для V-гравировки нужна V-образная фреза, инструмент { $tool } — { $type }
toolpath-vbit-no-angle = инструмент { $tool } — V-образная фреза без заданного угла; задайте угол
toolpath-pocket-endmill = для карманов нужна концевая фреза, инструмент { $tool } — { $type }
toolpath-path-drill = инструмент { $tool } — сверло и не может обрабатывать вдоль линии; используйте концевую или V-образную фрезу
toolpath-holes-vbit = инструмент { $tool } — V-образная фреза и не может обрабатывать отверстия; используйте сверло или концевую фрезу
toolpath-holes-method = Способ { $method ->
    [drill] drill требует сверла
   *[bore] bore требует концевой фрезы
}; инструмент { $tool } — { $type }
toolpath-flank-not-centre-cutting = инструмент { $tool } без режущего центра, а склоны перемычек ({ $flank }°) круче угла врезания ({ $ramp }°); спуск по склону — наклонное врезание, поэтому задайте угол не больше { $ramp }°
toolpath-plunge-not-centre-cutting = инструмент { $tool } без режущего центра, а операция врезается вертикально; используйте наклонное врезание (entry: ramp) или инструмент с режущим центром
toolpath-rest-no-tool = доработка после инструмента { $tool }, которого нет в проекте
toolpath-rest-not-endmill = доработка после инструмента { $tool } ({ $type }); инструмент кармана должен быть концевой фрезой
toolpath-rest-not-larger = доработка после инструмента { $tool } ({ $large }) требует инструмента больше, чем у этого кармана ({ $small })
toolpath-rest-dogbones = разгрузки dogbone пока несовместимы с доработкой остатка; добавьте их в карман большего инструмента
toolpath-rest-passes-dogbones = разгрузки dogbone пока несовместимы с доработкой остатка; углы с dogbone не требуют доработки
toolpath-rest-pass-no-tool = инструмент { $tool } не задан в проекте
toolpath-rest-pass-endmill = инструмент { $tool } — { $type }; для доработки остатка нужна концевая фреза
toolpath-rest-pass-larger = инструмент { $tool } ({ $diameter }) должен быть меньше предыдущего инструмента { $before } ({ $before_diameter })
toolpath-rest-pass-no-rpm = у инструмента { $tool } не заданы обороты шпинделя (об/мин)
toolpath-floor-pass-endmill = инструмент { $tool } — { $type }; для обработки дна нужна концевая фреза
toolpath-floor-no-fit = контуры { $contours }: концевая фреза { $diameter } не помещается на дне; V-образная фреза дочищает по средней линии
toolpath-labelled = { $label }: { $note }
toolpath-labelled-contour = { $label }, контур №{ $contour }: { $note }
toolpath-comma = { $list }, { $item }
toolpath-and = { $list } и { $last }
toolpath-number = { $number }
toolpath-operation-kind = { $number } ({ $kind ->
    [profile] обработка контура
    [pocket] выборка кармана
    [vcarve] V-гравировка
    [path] вдоль линии
    [holes] отверстия
   *[surface] торцевание
})
toolpath-skipped-open = пропущен разомкнутый контур №{ $contour } на { $layers }, от { $from } до { $to }
toolpath-into-spoilboard = рез на { $below } ниже заготовки (в жертвенный стол)
toolpath-plunge-unknown = Инструмент { $tool } не отмечен как center_cutting, а { $count ->
    [one] операция { $operations } врезается
   *[other] операции { $operations } врезаются
} вертикально: убедитесь, что у инструмента есть режущий центр, или используйте наклонное врезание (entry: ramp)
toolpath-surface-after = { $label } торцует заготовку после того, как { $count ->
    [one] операция { $operations } уже обработала её
   *[other] операции { $operations } уже обработали её
}: торцевание обычно выполняется первым
toolpath-rest-no-earlier = доработка после инструмента { $tool }, но ни один предыдущий карман не обрабатывает им эту геометрию на такую глубину; остальная часть кармана останется необработанной
toolpath-rest-pass-plunge = вертикальное врезание инструментом { $tool }, не отмеченным как имеющий режущий центр; если центра нет, используйте винтовое или наклонное врезание
toolpath-tool-returns = Инструмент { $tool } используется в { $groups } группах (операции { $operations }); разместите их рядом, чтобы сократить число смен инструмента
toolpath-vbit-no-tip = у инструмента { $tool } не задан диаметр вершины; V-гравировка считает её острой
toolpath-flank-unknown-centre-cutting = склоны перемычек ({ $flank }°) круче угла врезания ({ $ramp }°); инструмент { $tool } не отмечен как center_cutting, убедитесь, что у него есть режущий центр
toolpath-narrow-loops = инструмент не проходит узкий участок; обработка разделена на { $loops } контуров, между ними остаётся материал
toolpath-start-far-contour = начальная точка { $point } находится в { $distance } от контура №{ $contour }; используется ближайшая точка
toolpath-start-twice-contour = начальная точка { $point } также лежит на контуре №{ $contour }, у которого уже есть начало; она игнорируется
toolpath-tabs-inside = перемычки на внутреннем контуре будут удерживать вырезаемую внутреннюю часть
toolpath-tabs-on-curves = некоторые перемычки на контурах { $contours } находятся на углу или кривой (рядом нет прямого участка)
toolpath-tabs-joined = перемычки { $pairs } соединяются ниже вершин (склоны { $flank }°): они удерживают как одна длинная перемычка
toolpath-finish-lap-cuts = чистовой обход снимает до { $most } в углах или узких участках (чистовой припуск { $stock })
toolpath-corner-unreached = угол в { $point }: инструмент не достигает его; { $style ->
    [dogbone] без разгрузки dogbone
   *[tbone] без разгрузки T-bone
}
toolpath-corners-not-rounded = контуры { $contours }: углы колец не скруглены, иначе остался бы материал
toolpath-pocket-narrow = контуры { $contours }: инструмент не проходит узкий участок; выбрано областей: { $areas }, между ними остаётся материал
toolpath-uncut = контуры { $contours }: необработанный материал в { $count ->
    [one] { $count } месте
   *[other] { $count } местах
}, наибольшая область около { $width } x { $height } рядом с { $near }; уменьшите боковой шаг для её выборки
toolpath-lead-reduced = контуры { $contours }: радиус подвода уменьшен до { $used } (нет места для { $radius })
toolpath-no-lead = контуры { $contours }: нет места для подвода (даже { $least }); стенка обрабатывается без него
toolpath-helix-reduced = контуры { $contours }: радиус винтового врезания уменьшен до { $used } (нет места для { $radius })
toolpath-no-helix-room = контуры { $contours }: нет места для винтового врезания (даже { $least }); используется наклонное
toolpath-no-helix-narrow = контуры { $contours }: нет места для винтового врезания (карман слишком узкий); используется наклонное
toolpath-no-helix-way = контуры { $contours }: нет места для винтового врезания (переход к первой траектории пересекает стенку или остров); используется наклонное
toolpath-start-far-wall = контуры { $contours }: начальная точка { $point } в { $distance } от стенки; используется ближайшая точка
toolpath-start-twice-wall = контуры { $contours }: начальная точка { $point } лежит на стенке, у которой уже есть начало; она игнорируется
toolpath-rest-nothing-left = больший инструмент достигает всего кармана; остатка для доработки нет
toolpath-rest-too-small = доработка остатка: материал в { $count ->
    [one] { $count } месте
   *[other] { $count } местах
} слишком мал и для этого инструмента (наибольшая область около { $width } × { $height })
toolpath-vcarve-floor-left-bit = контуры { $contours }: шире, чем может обработать V-образная фреза (диаметр { $across }) при глубине не более { $cap } ({ $wide } на поверхности): обрабатываются стенки, а дно между ними остаётся на исходной поверхности. Добавьте обработку дна концевой фрезой
toolpath-vcarve-floor-left = контуры { $contours }: шире, чем позволяет гравировка глубиной не более { $cap } ({ $wide } на поверхности): V-образная фреза обрабатывает стенки, а дно между ними остаётся на исходной поверхности. Добавьте обработку дна концевой фрезой
toolpath-vcarve-floor-cleared = контуры { $contours }: концевая фреза { $diameter } выбирает { $cleared } % дна; фреза не больше { $largest } выбрала бы всё
toolpath-vcarve-floor-cleared-none = контуры { $contours }: концевая фреза { $diameter } выбирает { $cleared } % дна; часть участков слишком узка для любой концевой фрезы
toolpath-vcarve-narrow = контуры { $contours }: участок средней линии длиной { $narrow } уже вершины ({ $tip }) и не гравируется
toolpath-path-too-short = контур №{ $contour } слишком короткий для обработки вдоль линии ({ $length }); пропущен
toolpath-groove-no-angle = инструмент { $tool } — V-образная фреза без заданного угла: ширина паза неизвестна
toolpath-groove-past-cone = V-образный паз глубже конуса фрезы: ширина вверху { $width }, выше конуса — прямые стенки
toolpath-groove-width = ширина V-образного паза вверху — { $width }
toolpath-holes-not-circles = Пропущено { $count ->
    [one] { $count } объект, не являющийся окружностью
    [few] { $count } объекта, не являющихся окружностями
   *[other] { $count } объектов, не являющихся окружностями
}
toolpath-holes-not-drill-size = { $count ->
    [one] Отверстие { $diameter }
    [few] { $count } отверстия диаметром { $diameter }
   *[other] { $count } отверстий диаметром { $diameter }
}: не соответствует сверлу { $tool } (допуск { $within }); не просверлено
toolpath-holes-too-small-to-bore = { $count ->
    [one] Отверстие { $diameter }
    [few] { $count } отверстия диаметром { $diameter }
   *[other] { $count } отверстий диаметром { $diameter }
}: слишком мало для фрезерования инструментом { $tool }; не обработано
toolpath-holes-core = { $count ->
    [one] В { $count } отверстии остаётся
   *[other] В { $count } отверстиях остаётся
} свободный стержень (диаметр отверстия больше двух диаметров инструмента); закрепите его или используйте карман
toolpath-error-cancelled = Расчёт отменён
toolpath-error-context = { $context }: { $error }
toolpath-error-label-contour = { $label }, контур №{ $contour }
toolpath-error-part-copy = деталь { $part }, копия { $copy }
toolpath-error-copy = копия { $copy }
toolpath-error-no-operation = В задании нет операции { $at }
toolpath-error-shape-missing = объект { $shapes } не входит ни в один контур (удалён либо импорт убрал дубликат или сегмент нулевой длины)
toolpath-error-construction = { $shapes } — вспомогательная геометрия: она не обрабатывается. Сделайте её обычной или выберите другие объекты
toolpath-error-partly-selected = контур №{ $contour } выделен лишь частично (слои { $layers }); исправьте чертёж или селектор
toolpath-error-contour-invalid = контур №{ $contour } некорректен: { $reason }
toolpath-error-selects-nothing = селектор { $selector } ничего не выбирает
toolpath-error-selects-open = селектор { $selector } выбирает только разомкнутые контуры ({ $contours }); для этой операции нужны замкнутые
toolpath-error-selector-layer = слой '{ $layer }'
toolpath-error-selector-shapes = объекты { $shapes }
toolpath-error-selector-and = { $first } и { $second }
toolpath-error-selector-dxf = DXF { $id }
toolpath-error-selector-image = изображение { $id }
toolpath-error-selector-text = текст { $id }
toolpath-error-selector-but = { $source }, кроме { $shapes }
toolpath-error-selector-of = { $what } из { $source }
toolpath-error-same-shape = контуры №{ $a } и №{ $b } совпадают; выберите только один
toolpath-error-pass-depths = глубины должны удовлетворять 0 <= start ({ $start }) < end ({ $end }), а step_down ({ $step_down }) должна быть положительной
toolpath-error-rest-tool = доработка после инструмента { $tool }, которого нет в проекте
toolpath-error-blind-bore = отверстие { $diameter } глухое и больше двух диаметров инструмента { $tool }; используйте карман
toolpath-error-no-hole-fits = ни одну выделенную окружность нельзя обработать инструментом { $tool } ({ $diameter } { $type })
toolpath-error-no-depth = не задана глубина обработки
toolpath-error-no-angle = у инструмента { $tool } не задан угол
toolpath-error-vbit-angle = угол V-образной фрезы должен быть больше 0 и меньше 180 градусов, получено { $got }
toolpath-error-stepover = боковой шаг должен быть положительным, получено { $got }
toolpath-error-ring-no-length = виток контура имеет нулевую длину
toolpath-error-lap-no-course = у обхода стенки нет траектории
toolpath-error-ring-ramp = Виток слишком короткий для заглубления на { $drop } под углом { $angle }° (длина { $length }); увеличьте угол врезания или используйте вертикальное врезание инструментом с режущим центром
toolpath-error-ramp-reach = Траектория слишком короткая для заглубления на { $drop } под углом { $angle }° (доступно только { $reach }); увеличьте угол врезания или используйте вертикальное врезание инструментом с режущим центром
toolpath-error-path-ramp = Траектория слишком короткая для заглубления на { $drop } под углом { $angle }° (длина { $length }); увеличьте угол врезания или используйте вертикальное врезание инструментом с режущим центром
toolpath-error-helix-turns = винтовой линии радиусом { $radius } под углом { $angle }° нужно { $turns } оборотов для заглубления на { $drop }; увеличьте радиус или угол
toolpath-error-tool-does-not-fit = Инструмент не помещается внутри контура (его центр должен идти на { $offset } внутрь от линии)
toolpath-error-offset-verification = Проверка смещения не пройдена (траектория не на расстоянии радиуса инструмента от контура); возможно, контур вырожден
toolpath-error-roughing-reach = черновая обработка не достигает части стенки (чистовой обход удалён от черновой траектории на { $farthest }, не меньше радиуса инструмента): уменьшите чистовой припуск или инструмент
toolpath-error-tabs-fit = Перемычки ({ $count } шт., ширина { $width }) не помещаются на траектории { $length } ({ $fits ->
    [0] не помещается ни одна
   *[other] помещается не больше { $fits }
})
toolpath-tab-on-start = перемычка в { $at } перекрывает начало витка или расположена ближе половины диаметра инструмента к нему, где заглубляется каждый проход; переместите перемычку или начало витка
toolpath-tabs-too-close = перемычки в { $at } и { $other } слишком близко: между центрами нужна их ширина плюс два диаметра инструмента
toolpath-tab-no-ring = { $label }: перемычка в { $at } больше не лежит ни на одном контуре (чертёж изменился?); переместите или удалите её
toolpath-error-tab-carry = не удалось перенести перемычку (от { $from } до { $to } вдоль витка) на черновой виток
toolpath-error-tab-verification = Проверка траектории перемычки не пройдена: { $what }
toolpath-error-tab-below = Z{ $z } в { $at } вдоль витка ниже перемычки
toolpath-error-tab-back = инструмент движется назад по витку в { $at }
toolpath-error-tab-end-below = Z{ $z } в конце перемычки { $end } вдоль витка ниже её вершины Z{ $top }
toolpath-error-corner-leg = { $label }: участок разгрузки угла { $at } не включён в виток как ход туда и обратно
toolpath-corner-cuts = { $label }: разгрузка { $style ->
    [dogbone] dogbone
   *[tbone] T-bone
} в углу { $at } также врезается в контур №{ $contour }: проверьте деталь в этом месте или выберите другую разгрузку
toolpath-corner-cuts-own = { $label }: разгрузка { $style ->
    [dogbone] dogbone
   *[tbone] T-bone
} в углу { $at } также врезается в другую стенку собственного контура №{ $contour }: проверьте деталь или выберите другую разгрузку (T-bone вдоль другой стенки либо скругление)
toolpath-corner-choice-unmatched = { $label }: выбранный угол в { $at } больше не является углом геометрии (чертёж изменился?); удалите настройку или выберите заново
toolpath-error-arc-off-circle = вывод дуги: ({ $x }, { $y }) не лежит на назначенной ей окружности
toolpath-error-islands-enclose = контуры { $contours }: перекрывающиеся острова замыкают область между собой; карманы пока не поддерживают это
toolpath-error-pocket-no-fit = контуры { $contours }: инструмент (диаметр { $diameter }) не помещается в кармане
toolpath-error-pocket-no-fit-clearance = контуры { $contours }: инструмент (диаметр { $diameter }, перерез { $clearance }) не помещается в кармане
toolpath-error-not-nested = контуры { $contours }: смещённые контуры кармана не вложены
toolpath-error-too-many-rings = контуры { $contours }: колец кармана больше { $most }
toolpath-error-pocket-verification = контуры { $contours }: проверка траектории кармана не пройдена (слишком близко к стенке или острову); возможно, чертёж вырожден
toolpath-error-nothing-to-clear = Нечего выбирать: выделенные острова покрывают выделенные границы (контуры { $contours })
toolpath-error-island-outside = смещение кармана создало остров вне всех областей
toolpath-error-shrink-hole = смещение кармана создало отверстие при уменьшении одного контура
toolpath-error-open-loop = смещение кармана вернуло разомкнутый контур вместо замкнутого
toolpath-error-rest-outside = область доработки остатка лежит вне кармана
toolpath-error-left-hole-outside = остаток материала: отверстие вне всех фрагментов
toolpath-error-left-holes-enclose = остаток материала: отверстия замыкают область
toolpath-error-left-wrong-area = остаток материала: булева операция вернула неверную площадь
toolpath-error-left-invalid = остаток материала: некорректная геометрия из библиотеки смещений
toolpath-error-medial-name = { $name ->
    [vcarve] V-гравировка
   *[centre-line] средняя линия
}
toolpath-error-medial-failed = контуры { $contours }: не удалось построить среднюю линию для «{ $name }» ({ $error }); граница касается или пересекает себя?
toolpath-error-medial-cell = средняя линия для «{ $name }»: ячейка без хорды
toolpath-error-medial-far = граница для «{ $name }» в { $at } слишком далеко от начала координат чертежа (средняя линия работает в пределах ±{ $reach })
toolpath-error-medial-step = Средняя линия для «{ $name }»
toolpath-error-vcarve-verification = контуры { $contours }: проверка V-гравировки не пройдена: рез пересёк бы границу рядом с { $at }
toolpath-error-join-gave = объединение двух перекрывающихся границ дало границ: { $outlines }
toolpath-error-join-changed = объединение перекрывающихся границ изменило заполненную область рядом с { $at }
toolpath-error-joined-cross = объединённые границы пересекаются
toolpath-error-outline-unreadable = библиотека геометрии не может прочитать границу
toolpath-error-copies-dx = Копии перекрываются: при нескольких столбцах шаг dx должен быть не меньше { $width } (ширина реза { $width }), получено { $got }
toolpath-error-copies-dy = Копии перекрываются: при нескольких строках шаг dy должен быть не меньше { $height } (высота реза { $height }), получено { $got }
toolpath-error-copies-overlap = Копии перекрываются: копии { $other } и { $copy } врезаются друг в друга; раздвиньте их (шаг dx { $dx }, dy { $dy })
toolpath-error-parts-overlap = Деталь «{ $part }», копия { $copy }, перекрывает деталь «{ $other_part }», копию { $other_copy } (X{ $x0 } … { $x1 }, Y{ $y0 } … { $y1 }): раздвиньте копии или измените сетку
toolpath-error-library-failed = Сбой внутри библиотеки геометрии: { $what }
toolpath-error-step = { $step ->
    [pocket-offset] Смещение кармана
    [joining-letters] Объединение букв
    [checking-joined-letters] Проверка объединённых букв
    [material-left-union] Объединение остатка материала
    [material-left-offset] Смещение остатка материала
    [material-left-boolean] Булева операция с остатком материала
    [comparing-shapes] Сравнение геометрии
    [merging-islands] Объединение островов
    [subtracting-an-island] Вычитание острова
   *[offset] Смещение
}
