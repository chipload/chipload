svg-error-too-large = размер SVG превышает { $most }
svg-error-unreadable = невозможно прочитать SVG: { $reason }
svg-error-crashed = сбой парсера SVG на этом файле
svg-error-too-deep = вложенность элементов больше { $most }
svg-error-needs-scale = в SVG нет физического размера (только пиксели): задайте пикселей на дюйм — 96 (Inkscape, браузеры) или 72 (старые версии Illustrator)
svg-error-scale = масштаб или число пикселей на дюйм должны быть больше 0, получено { $got }
svg-error-too-many = в SVG объектов: { $count }, больше { $most }
svg-error-far-out = у { $element } есть точка с нечисловой координатой или дальше { $most } от начала координат
svg-error-huge-number = у { $element } есть число больше { $most } в единицах файла
svg-px-chosen = в SVG нет физического размера; использовано выбранное разрешение { $px } пикселей на дюйм
svg-text-left-out = Пропущено текстов: { $count ->
    [one] { $count }
   *[other] { $count }
}: преобразуйте текст в кривые в редакторе (Inkscape: Path > Object to Path; Illustrator: Create Outlines) или используйте текст Chipload
svg-images-left-out = Пропущено изображений: { $count ->
    [one] { $count }
   *[other] { $count }
}; изображение для векторизации добавляется отдельно
svg-clips-ignored = Проигнорировано обтравочных контуров или масок: { $count ->
    [one] { $count }
   *[other] { $count }
}: обрезаемая ими геометрия импортирована целиком
svg-hidden-paths = Импортировано невидимых контуров (без заливки и обводки либо скрытых): { $count ->
    [one] { $count }
   *[other] { $count }
}
svg-empty-dropped = Пропущено подконтуров нулевой длины: { $count ->
    [one] { $count }
   *[other] { $count }
}
