# Files dragged onto the window: the hint shown while they are over it
# (crates/chipload-gui/src/app/dropping.rs).

dropping-unsupported = Déposez un projet .chipload, ou des DXF et des images
dropping-open-project = Ouvrir ce projet
dropping-one-project = Déposez un seul projet .chipload à la fois

# Dropped on an open project.
dropping-add-dxfs = { $dxfs ->
    [one] Ajouter { $dxfs } DXF au projet
   *[other] Ajouter { $dxfs } DXF au projet
}
dropping-add-pictures = { $pictures ->
    [one] Ajouter { $pictures } image au projet
   *[other] Ajouter { $pictures } images au projet
}
dropping-add-both = Ajouter { $dxfs } DXF et { $pictures ->
        [one] { $pictures } image
       *[other] { $pictures } images
    } au projet

# Dropped when no project is open.
dropping-new-dxfs = Nouveau projet avec { $dxfs } DXF
dropping-new-pictures = { $pictures ->
    [one] Nouveau projet avec { $pictures } image
   *[other] Nouveau projet avec { $pictures } images
}
dropping-new-both = Nouveau projet avec { $dxfs } DXF et { $pictures ->
        [one] { $pictures } image
       *[other] { $pictures } images
    }
