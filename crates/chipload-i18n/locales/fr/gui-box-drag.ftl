# Dragging the handles of a picture's or text's box (crates/chipload-gui/src/app/box_drag.rs).

# In the status bar when the box can't be dragged to that size; $error says why.
box-drag-not-that-size = Pas à cette taille : { $error }
box-drag-cannot-move-shapes = Impossible de déplacer les formes :
box-drag-stretched-fitted = { $count ->
    [one] { $shapes } est maintenant une polyligne
   *[other] { $shapes } sont maintenant des polylignes
} de lignes et d'arcs qui suivent la courbe étirée.
