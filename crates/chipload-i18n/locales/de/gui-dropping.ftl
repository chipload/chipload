# Files dragged onto the window: the hint shown while they are over it
# (crates/chipload-gui/src/app/dropping.rs).

dropping-unsupported = Ein .chipload-Projekt oder DXFs und Bilder ablegen
dropping-open-project = Dieses Projekt öffnen
dropping-one-project = Jeweils nur ein .chipload-Projekt ablegen

# Dropped on an open project.
dropping-add-dxfs = { $dxfs ->
    [one] { $dxfs } DXF zum Projekt hinzufügen
   *[other] { $dxfs } DXFs zum Projekt hinzufügen
}
dropping-add-pictures = { $pictures ->
    [one] { $pictures } Bild zum Projekt hinzufügen
   *[other] { $pictures } Bilder zum Projekt hinzufügen
}
dropping-add-both = { $dxfs ->
        [one] { $dxfs } DXF
       *[other] { $dxfs } DXFs
    } und { $pictures ->
        [one] { $pictures } Bild
       *[other] { $pictures } Bilder
    } zum Projekt hinzufügen

# Dropped when no project is open.
dropping-new-dxfs = { $dxfs ->
    [one] Neues Projekt mit { $dxfs } DXF
   *[other] Neues Projekt mit { $dxfs } DXFs
}
dropping-new-pictures = { $pictures ->
    [one] Neues Projekt mit { $pictures } Bild
   *[other] Neues Projekt mit { $pictures } Bildern
}
dropping-new-both = Neues Projekt mit { $dxfs ->
        [one] { $dxfs } DXF
       *[other] { $dxfs } DXFs
    } und { $pictures ->
        [one] { $pictures } Bild
       *[other] { $pictures } Bildern
    }
