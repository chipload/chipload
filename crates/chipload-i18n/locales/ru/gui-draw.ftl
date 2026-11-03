draw-status-circle-start = Окружность: укажите центр (сетка { $grid }, Esc — отмена)
draw-status-circle = Окружность радиусом { $radius }: укажите точку на окружности (сетка { $grid }, Esc — отмена)
draw-status-rectangle-start = Прямоугольник: укажите угол (сетка { $grid }, Esc — отмена)
draw-status-rectangle = Прямоугольник { $length } x { $width }: укажите противоположный угол (сетка { $grid }, Esc — отмена)
draw-status-polyline-start = Полилиния, { $count ->
    [one] { $count } точка
    [few] { $count } точки
   *[other] { $count } точек
}: укажите первую точку (сетка { $grid }, Esc — отмена)
draw-status-polyline = Полилиния, { $count ->
    [one] { $count } точка
    [few] { $count } точки
   *[other] { $count } точек
}: укажите следующую точку; Enter или двойной щелчок — завершить без замыкания, первая точка — замкнуть, Backspace — удалить последнюю точку (сетка { $grid }, Esc — отмена)
draw-no-size-circle = окружность нулевого размера
draw-no-size-rectangle = прямоугольник нулевого размера
draw-polyline-too-short = для полилинии нужны хотя бы 2 точки
draw-snap-end = конец №{ $contour }
draw-snap-centre = центр №{ $contour }
draw-snap-middle = середина №{ $contour }
draw-snap-intersection = пересечение №{ $contour } и №{ $other }
draw-snap-intersection-self = пересечение на №{ $contour }
draw-snap-quadrant = квадрант №{ $contour }
draw-snap-perpendicular = перпендикуляр к №{ $contour }
draw-snap-tangent = касательная к №{ $contour }
draw-snap-nearest = на №{ $contour }
draw-entry-unreadable = не удалось распознать: введите x,y  @dx,dy  длину или длина<угол
draw-entry-needs-point = первая точка: введите x,y
draw-entry-no-direction = отведите курсор от последней точки, чтобы задать направление
draw-status-arc-start = Дуга: укажите начало (сетка { $grid }, Esc — отмена)
draw-status-arc-end = Дуга, хорда { $length }: укажите конец (сетка { $grid }, Esc — отмена)
draw-status-arc = Дуга радиусом { $radius }: укажите точку на дуге или введите радиус (сетка { $grid }, Esc — отмена)
draw-status-slot-start = Паз: укажите центр одного конца (сетка { $grid }, Esc — отмена)
draw-status-slot-end = Паз, между концами { $length }: укажите центр другого конца (сетка { $grid }, Esc — отмена)
draw-status-slot = Паз шириной { $width }: укажите или введите ширину (сетка { $grid }, Esc — отмена)
draw-arc-on-a-line = три точки лежат на одной прямой: используйте полилинию
draw-no-size-slot = паз нулевой длины или ширины
draw-entry-radius-too-small = радиус должен быть не меньше половины расстояния между концами
draw-entry-no-side = переместите курсор в сторону выпуклости дуги
draw-status-polygon-start = Многоугольник: укажите центр (сетка { $grid }, Esc — отмена)
draw-status-polygon = Многоугольник радиусом { $radius }: укажите вершину (сетка { $grid }, Esc — отмена)
modify-offset-pick = Смещение: выделите объекты (щелчок; Ctrl — добавить), Esc — завершить
modify-offset-status = Смещение { $distance } { $side }: щёлкните или введите расстояние (Alt — без сетки, Esc — завершить)
modify-offset-done = { $count ->
    [one] Добавлено смещение
    [few] Добавлено { $count } смещения
   *[other] Добавлено { $count } смещений
} на { $distance }.
modify-side-outside = наружу
modify-side-inside = внутрь
modify-side-left = влево
modify-side-right = вправо
modify-corners-round = Скруглённые углы
modify-corners-sharp = Острые углы
modify-corners-hover = Скруглённые: точное смещение, с дугами вокруг углов. Острые: стороны продлеваются до пересечения, как у рамки.
modify-offset-count = Количество
modify-corner-pick = Выделите один начерченный объект, затем нажимайте на его углы; Esc — завершить
modify-fillet-status = Скругление: нажмите на угол; Shift+щелчок — скруглить все углы (Esc — завершить)
modify-chamfer-status = Фаска: нажмите на угол; Shift+щелчок — срезать все углы (Esc — завершить)
modify-fillet-radius = Радиус
modify-chamfer-distance = Расстояние
modify-trim-status = Обрезка: нажмите на удаляемый участок начерченного объекта; предпросмотр до щелчка (Esc — завершить)
modify-extend-status = Продление: нажмите рядом с концом разомкнутого объекта, чтобы продлить его до следующего объекта (Esc — завершить)
modify-break-status = Разрыв: нажмите на начерченный объект в месте разрыва (Esc — завершить)
modify-extend-closed = у замкнутого объекта нет конца для продления
modify-combine-pick = Сначала выделите хотя бы два замкнутых начерченных объекта
modify-subtract-status = Вычитание: нажмите на объект, из которого нужно вычесть выделенные (Esc — завершить)
modify-construction-on = Следующие объекты — вспомогательная геометрия (K — выключить).
modify-construction-off = Следующие объекты — обычная геометрия.
modify-array-pick = Сначала выделите начерченные объекты для копирования
modify-array-status = Массив: задайте копии в разделе «Выделение», затем нажмите «Создать» (Esc — завершить)
modify-polar-status = Круговой массив: укажите центр, задайте копии в разделе «Выделение», затем нажмите «Создать» (Esc — завершить)
array-columns = Столбцы
array-rows = Строки
array-dx = Шаг столбцов
array-dy = Шаг строк
array-count = Количество
array-center = Центр
array-angle = Угол
array-angle-hover = 360° — равномерно по окружности; меньше — первая и последняя копии на концах дуги; отрицательный — по часовой стрелке
array-rotate = Поворачивать копии
array-create = Создать
arrange-align = Выровнять
arrange-left = По левым краям
arrange-centre = По центрам по горизонтали
arrange-right = По правым краям
arrange-top = По верхним краям
arrange-middle = По центрам по вертикали
arrange-bottom = По нижним краям
arrange-stock = По заготовке
arrange-stock-hover = Выровнять по краям или центру заготовки, а не по общей рамке объектов
arrange-distribute = Распределить
arrange-across = Равные зазоры по горизонтали; крайние объекты неподвижны
arrange-up = Равные зазоры по вертикали; нижний и верхний объекты неподвижны
modify-frame-status = Рамка вокруг выделения: настройте в разделе «Выделение», затем нажмите «Добавить» (Esc — завершить)
modify-frame-size-status = Рамка по размеру: настройте в разделе «Выделение» или выделите объекты для обрамления (Esc — завершить)
frame-title = Рамка
frame-around = Вокруг выделения, по его центру.
frame-to-size = По размеру, по центру заготовки.
frame-margin = Поле
frame-margin-hover = От рамки выделения до самой внутренней линии
frame-width = Ширина
frame-height = Высота
frame-corner = Угол
frame-corner-hover = Размер угла: радиус скругления, выемка или фаска, подъём арки, выпуклость таблички
frame-corner-auto = авто ({ $size })
frame-corner-auto-hover = Задать вручную; выключено — десятая часть короткой стороны
frame-borders = Окантовки
frame-gap = Зазор
frame-gap-hover = От внешнего контура до первой линии окантовки и между линиями
frame-layer = Слой
frame-border-layer = Слой окантовки
frame-size = { $width } × { $height }
frame-add = Добавить
frame-kind-rectangle = Прямоугольник
frame-kind-rounded = Со скруглением
frame-kind-notched = С выемками: четверти окружностей в углах
frame-kind-chamfered = С фасками
frame-kind-oval = Овал
frame-kind-arched = С арочным верхом
frame-kind-plaque = Табличка: выпуклые короткие стороны
