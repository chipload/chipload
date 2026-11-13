# 3D models in the window: File > Add 3D model…, their rows in the Browser and their
# properties (crates/chipload-gui/src/app/models.rs, sources.rs, forms/sources.rs).

models-stl-filter = Модели STL
models-cannot-read = Не удаётся прочитать { $path }: { $error }
models-reading = Чтение { $file }…
models-cannot-add = Не удаётся добавить { $path }:
models-cannot-place = Не удалось так разместить модель:
models-removed = Модель { $id } удалена
dialogs-path-model-title = Добавить 3D-модель
dialogs-path-model = Файл STL (двоичный или текстовый):
menu-add-model = Добавить 3D-модель…
menu-add-model-hover = Файл STL, вырезаемый в заготовке сверху как рельеф
menu-add-model-path = Добавить 3D-модель по пути…
sources-model-title = { $name } (модель { $id })
sources-add-model = Добавить 3D-модель…
sources-add-model-hover = Файл STL, вырезаемый в заготовке сверху как рельеф
sources-model-hover = 3D-модель: её контур на заготовке. Двойной щелчок — положение, размер и глубина
sources-model-new-relief = Новый рельеф
sources-model-new-relief-hover = Вырезать эту модель: операция «Рельеф» внутри её контура, в активной детали
sources-model-properties = Свойства
sources-remove-model-hover = Удалить модель и её контур; её рельефы остаются, каждый с проблемой, пока не получит другую модель
forms-model-file = Файл
forms-model-size = На заготовке
forms-model-size-hover = Ширина × высота × глубина размещённой модели
forms-model-position-hover = Нижний левый угол контура модели до поворота
forms-model-reset-position = Вернуть к началу координат чертежа
forms-model-rotation-hover = Повёрнута против часовой стрелки вокруг центра контура
forms-model-reset-rotation = Без поворота
forms-model-width-hover = Ширина модели на заготовке; высота следует за ней
forms-model-depth = Глубина
forms-model-depth-hover = Насколько глубоко идёт рельеф, от верха модели до её нижней точки. Рельефы обычно площе модели
forms-model-proportional = Сохранять пропорции
forms-model-proportional-hover = Глубина следует за шириной в пропорциях модели
forms-model-up = Вверх
forms-model-up-hover = Какая ось модели направлена вверх от заготовки. Многие файлы смоделированы с осью Y вверх
forms-model-layer-hover = Слой, на котором рисуется контур модели
forms-model-new-relief = Новый рельеф
forms-model-new-relief-hover = Вырезать эту модель: операция «Рельеф» внутри её контура, в активной детали
forms-remove-model-hover = Удалить модель и её контур; её рельефы остаются, каждый с проблемой, пока не получит другую модель

# Reliefs and their roughing pass (docs/specs/stl-relief.md §9, §14).
models-could-not-add-roughing = Не удалось добавить черновой проход: { $error }
models-no-roughing-tool = В проекте и вашей библиотеке нет концевой или обычной сферической фрезы
