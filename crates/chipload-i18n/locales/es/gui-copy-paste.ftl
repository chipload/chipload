# Copy, paste and mirror drawn shapes (crates/chipload-gui/src/app/copy_paste.rs).
# Drawn shapes are drawn in Chipload; imported ones come from a DXF, a picture or a text.

copy-paste-imported-no-copy = Las formas importadas no se pueden copiar; solo las dibujadas
copy-paste-select-to-copy = Selecciona formas dibujadas para copiar
copy-paste-copied = { $count ->
    [one] { $count } forma copiada
   *[other] { $count } formas copiadas
}

copy-paste-no-shapes = El portapapeles no contiene formas
copy-paste-copy-first = Copia formas dibujadas primero (Ctrl+C)
# $shapes: the new shapes' names, e.g. S4, S5.
copy-paste-pasted = Pegado { $shapes }
# Followed by the reason on the next line.
copy-paste-cannot-paste = No se puede pegar:

copy-paste-imported-no-mirror = Las formas importadas no se pueden reflejar; solo las dibujadas
copy-paste-select-to-mirror = Selecciona formas dibujadas para reflejar
copy-paste-mirrored = Reflejado
copy-paste-mirrored-drawn = Reflejado (las formas importadas de la selección quedan como estaban)
# Followed by the reason on the next line.
copy-paste-cannot-mirror = No se puede reflejar:
