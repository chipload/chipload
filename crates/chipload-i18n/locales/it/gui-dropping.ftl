# Files dragged onto the window: the hint shown while they are over it
# (crates/chipload-gui/src/app/dropping.rs).

dropping-unsupported = Rilascia un progetto .chipload, o disegni (DXF, SVG) e immagini
dropping-open-project = Apri questo progetto
dropping-one-project = Rilascia un solo progetto .chipload alla volta

# Dropped on an open project.
dropping-add-dxfs = { $dxfs ->
    [one] Aggiungi { $dxfs } disegno al progetto
   *[other] Aggiungi { $dxfs } disegni al progetto
}
dropping-add-pictures = { $pictures ->
    [one] Aggiungi { $pictures } immagine al progetto
   *[other] Aggiungi { $pictures } immagini al progetto
}
dropping-add-both = Aggiungi { $dxfs ->
        [one] { $dxfs } disegno
       *[other] { $dxfs } disegni
    } e { $pictures ->
        [one] { $pictures } immagine
       *[other] { $pictures } immagini
    } al progetto

# Dropped when no project is open.
dropping-new-dxfs = { $dxfs ->
    [one] Nuovo progetto con { $dxfs } disegno
   *[other] Nuovo progetto con { $dxfs } disegni
}
dropping-new-pictures = { $pictures ->
    [one] Nuovo progetto con { $pictures } immagine
   *[other] Nuovo progetto con { $pictures } immagini
}
dropping-new-both = Nuovo progetto con { $dxfs ->
        [one] { $dxfs } disegno
       *[other] { $dxfs } disegni
    } e { $pictures ->
        [one] { $pictures } immagine
       *[other] { $pictures } immagini
    }
