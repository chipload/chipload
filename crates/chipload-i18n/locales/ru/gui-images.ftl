images-pictures-filter = Изображения
images-cannot-read = Не удалось прочитать { $path }: { $error }
images-cannot-import = Не удалось импортировать { $path }:
images-trace-at-size-failed = Не удалось векторизовать изображение с таким размером:
images-tracing-failed = ошибка векторизации изображения
images-traced-next = Изображение выделено: нажмите «V-гравировка» на ленте для обработки. Размер и положение задаются в свойствах.
images-traced = Векторизовано { $file }
images-removed = Изображение { $id } удалено
images-import-title = Импорт изображения: { $file }
images-trace-title = Векторизация { $file }
images-import = Импортировать
images-zoom-note = Масштаб { $zoom } · перетаскивание — сдвиг вида · двойной щелчок — вписать
images-scroll-to-zoom = Колесо — масштаб
images-threshold = Порог
images-threshold-hover = Оттенки серого ниже порога становятся чёрными (0 — чёрный, 255 — белый). Авто выбирает порог, лучше всего разделяющий тёмные и светлые области.
images-threshold-auto = авто
images-invert = Инверсия
images-invert-check = обрабатывать светлые области
images-blur = Размытие
images-blur-hover = Сглаживание неровных краёв и шума перед пороговой обработкой.
images-smallest-piece = Мин. размер фрагмента
images-smallest-piece-hover = Чёрные и белые вкрапления меньшего размера удаляются.
images-smoothing = Сглаживание
images-smoothing-hover = Допустимое отклонение векторной границы от края на изображении.
images-trace = Векторизация
images-trace-hover = Границы чёрных областей для V-гравировки или средние линии чёрных штрихов (рукописный текст, линейный рисунок) для обработки вдоль линии.
images-trace-outlines = Границы (V-гравировка)
images-trace-centre-lines = Средние линии (вдоль линии)
images-arcs = Дуги
images-arcs-hover = Аппроксимация дугами и отрезками: короче программы и плавнее движение при обработке контуров, карманов и линий.
images-arcs-check = аппроксимировать дугами
images-tracing = Векторизация…
images-found-shapes = { $count ->
    [one] { $count } объект
    [few] { $count } объекта
   *[other] { $count } объектов
}
images-found-shapes-auto = { $count ->
    [one] { $count } объект
    [few] { $count } объекта
   *[other] { $count } объектов
} (выбран порог { $threshold })
images-found-centre-lines = { $count ->
    [one] { $count } средняя линия
    [few] { $count } средние линии
   *[other] { $count } средних линий
}
images-found-centre-lines-auto = { $count ->
    [one] { $count } средняя линия
    [few] { $count } средние линии
   *[other] { $count } средних линий
} (выбран порог { $threshold })
images-stroke-widths = Ширина штрихов от { $narrow } до { $wide }
images-dots-left-out = Исключено точек: { $count }
images-kept-as-lines = Оставлено отрезками: { $count }
images-specks-dropped = Удалено вкраплений: { $count }
images-black-at-border = Чёрная область касается края: включите инверсию, если это фон.
