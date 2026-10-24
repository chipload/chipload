# Texts of the app that name a length or its unit, which follow the display units
# (Settings > Units): $unit is "mm" or "in", a length argument brings its own unit
# ("10 mm", "0.3937 in").

# Where X, Y and Z zero are, after "Work zero:". $at is one of the work zero choices
# ("stock bottom left"), $point where that is in the drawing ("(0.000, 0.000) mm"), $z one
# of the two below.
units-text-work-zero = { $at } = dibujo { $point }, Z0 = { $z }
units-text-zero-stock-top = cara superior del material
units-text-zero-stock-bottom = cara inferior del material

# Edit > Paste. $shift: how far a paste lands from the copied shapes.
units-text-paste-hover = { $shift } a la derecha y arriba de las formas copiadas; Ctrl+V sobre el lienzo pega en el cursor
# $diameter: the new tool's diameter.
units-text-new-tool-hover = Una fresa de { $diameter } para editar; las operaciones la eligen en su fila Herramienta
units-text-oversize-hover = Cuánto más de lo dibujado se corta, en { $unit } (clearance en el archivo): un valor positivo agranda una cajera o achica una pieza; uno negativo deja material para un corte posterior. Menor que el radio de la herramienta. Otros programas de CAM llaman a lo contrario "sobrematerial".
units-text-picture-position-hover = La esquina inferior izquierda del recuadro de la imagen, { $unit }
units-text-text-position-corner-hover = La esquina inferior izquierda del recuadro del texto, { $unit }
units-text-text-position-centre-hover = El centro del recuadro del texto, { $unit }
units-text-dxf-position-hover = La esquina inferior izquierda del recuadro del dibujo, { $unit }. Las formas dibujadas en Chipload y los demás DXF no se mueven.
units-text-pitch-hover = De copia a copia, { $unit }. Los negativos crecen hacia la izquierda o hacia abajo.
