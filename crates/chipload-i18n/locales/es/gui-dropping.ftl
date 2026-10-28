# Files dragged onto the window: the hint shown while they are over it
# (crates/chipload-gui/src/app/dropping.rs).

dropping-unsupported = Suelta un proyecto .chipload, o dibujos (DXF, SVG) e imágenes
dropping-open-project = Abrir este proyecto
dropping-one-project = Suelta un solo proyecto .chipload cada vez

# Dropped on an open project.
dropping-add-dxfs = { $dxfs ->
    [one] Añadir { $dxfs } dibujo al proyecto
   *[other] Añadir { $dxfs } dibujos al proyecto
}
dropping-add-pictures = { $pictures ->
    [one] Añadir { $pictures } imagen al proyecto
   *[other] Añadir { $pictures } imágenes al proyecto
}
dropping-add-both = Añadir { $dxfs ->
        [one] { $dxfs } dibujo
       *[other] { $dxfs } dibujos
    } y { $pictures ->
        [one] { $pictures } imagen
       *[other] { $pictures } imágenes
    } al proyecto

# Dropped when no project is open.
dropping-new-dxfs = Proyecto nuevo con { $dxfs ->
        [one] { $dxfs } dibujo
       *[other] { $dxfs } dibujos
    }
dropping-new-pictures = { $pictures ->
    [one] Proyecto nuevo con { $pictures } imagen
   *[other] Proyecto nuevo con { $pictures } imágenes
}
dropping-new-both = Proyecto nuevo con { $dxfs ->
        [one] { $dxfs } dibujo
       *[other] { $dxfs } dibujos
    } y { $pictures ->
        [one] { $pictures } imagen
       *[other] { $pictures } imágenes
    }
