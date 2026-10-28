# Files dragged onto the window: the hint shown while they are over it
# (crates/chipload-gui/src/app/dropping.rs).

dropping-unsupported = Släpp ett .chipload-projekt, eller ritningar (DXF, SVG) och bilder
dropping-open-project = Öppna detta projekt
dropping-one-project = Släpp ett .chipload-projekt i taget

# Dropped on an open project.
dropping-add-dxfs = { $dxfs ->
    [one] Lägg till { $dxfs } ritning i projektet
   *[other] Lägg till { $dxfs } ritningar i projektet
}
dropping-add-pictures = { $pictures ->
    [one] Lägg till { $pictures } bild i projektet
   *[other] Lägg till { $pictures } bilder i projektet
}
dropping-add-both = Lägg till { $dxfs ->
        [one] { $dxfs } ritning
       *[other] { $dxfs } ritningar
    } och { $pictures ->
        [one] { $pictures } bild
       *[other] { $pictures } bilder
    } i projektet

# Dropped when no project is open.
dropping-new-dxfs = { $dxfs ->
    [one] Nytt projekt med { $dxfs } ritning
   *[other] Nytt projekt med { $dxfs } ritningar
}
dropping-new-pictures = { $pictures ->
    [one] Nytt projekt med { $pictures } bild
   *[other] Nytt projekt med { $pictures } bilder
}
dropping-new-both = Nytt projekt med { $dxfs ->
        [one] { $dxfs } ritning
       *[other] { $dxfs } ritningar
    } och { $pictures ->
        [one] { $pictures } bild
       *[other] { $pictures } bilder
    }
