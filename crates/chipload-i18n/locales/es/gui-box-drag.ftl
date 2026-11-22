# Dragging the handles of a picture's or text's box (crates/chipload-gui/src/app/box_drag.rs).

# In the status bar when the box can't be dragged to that size; $error says why.
box-drag-not-that-size = No a ese tamaño: { $error }
box-drag-cannot-move-shapes = No se pudieron mover las formas:
box-drag-stretched-fitted = { $count ->
    [one] { $shapes } es ahora una polilínea
   *[other] { $shapes } son ahora polilíneas
} de líneas y arcos que siguen la curva estirada.
