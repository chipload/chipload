# The new project dialog (crates/chipload-gui/src/app/new_project.rs, docs/specs/new-project-dialog.md).

new-project-title = Новый проект
new-project-material = Материал
new-project-material-none = Нет
new-project-thickness = Толщина
# The board: fitted round the drawing with a margin, or a size.
new-project-board = Заготовка
new-project-fit = По чертежу, отступ
new-project-size = Размер
new-project-z-zero = Ноль Z
new-project-z-zero-hover = Где задаётся Z0 станка: на верху заготовки или на жертвенном столе под ней
new-project-z-top = Верх заготовки
new-project-z-spoilboard = Жертвенный стол
new-project-machine = Станок
new-project-machine-none = Собственные настройки Chipload (в библиотеке нет станка)
# Under the machine: its post-processor, G-code units (mm or inch) and how tools are changed.
new-project-machine-summary = { $post } · { $units } · { $change }
new-project-tools = Инструменты
new-project-search = Поиск
# Under the tool list: how many are ticked and their numbers ("T1, T2").
new-project-tools-count = { $count ->
    [0] Нет инструментов: добавьте инструмент перед расчётом
    [one] { $count } инструмент: { $list }
    [few] { $count } инструмента: { $list }
   *[other] { $count } инструментов: { $list }
}
new-project-no-library-tools = Библиотека инструментов пуста: проект начнётся с концевой фрезы 6 мм. Добавьте инструменты в разделе «Этот компьютер > Инструменты».
new-project-create = Создать
