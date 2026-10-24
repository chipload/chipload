# Files dragged onto the window: the hint shown while they are over it
# (crates/chipload-gui/src/app/dropping.rs).

dropping-unsupported = Suelta un proyecto .chipload, o DXF e imágenes
dropping-open-project = Abrir este proyecto
dropping-one-project = Suelta un solo proyecto .chipload cada vez

# Dropped on an open project.
dropping-add-dxfs = { $dxfs ->
    [one] Añadir { $dxfs } DXF al proyecto
   *[other] Añadir { $dxfs } DXF al proyecto
}
dropping-add-pictures = { $pictures ->
    [one] Añadir { $pictures } imagen al proyecto
   *[other] Añadir { $pictures } imágenes al proyecto
}
dropping-add-both = Añadir { $dxfs } DXF y { $pictures ->
        [one] { $pictures } imagen
       *[other] { $pictures } imágenes
    } al proyecto

# Dropped when no project is open.
dropping-new-dxfs = Proyecto nuevo con { $dxfs } DXF
dropping-new-pictures = { $pictures ->
    [one] Proyecto nuevo con { $pictures } imagen
   *[other] Proyecto nuevo con { $pictures } imágenes
}
dropping-new-both = Proyecto nuevo con { $dxfs } DXF y { $pictures ->
        [one] { $pictures } imagen
       *[other] { $pictures } imágenes
    }
