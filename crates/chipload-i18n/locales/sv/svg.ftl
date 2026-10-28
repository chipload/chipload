# SVG-import (docs/specs/svg-import.md).
svg-error-too-large = SVG-filen är större än { $most }
svg-error-unreadable = inte en läsbar SVG: { $reason }
svg-error-crashed = SVG-tolken kraschade på den här filen
svg-error-too-deep = element är nästlade djupare än { $most }
svg-error-needs-scale = SVG-filen anger ingen verklig storlek (bara pixlar): välj dess pixlar per tum, 96 (Inkscape, webbläsare) eller 72 (äldre Illustrator)
svg-error-scale = skalan eller pixlarna per tum måste vara större än 0, fick { $got }
svg-error-too-many = SVG-filen har { $count } former, fler än { $most }
svg-error-far-out = { $element } har en punkt som inte är ett tal eller ligger mer än { $most } från origo
svg-px-chosen = SVG-filen anger ingen verklig storlek; läst med { $px } pixlar per tum, som valt
svg-text-left-out = { $count ->
    [one] 1 text utelämnad
   *[other] { $count } texter utelämnade
}: gör om texten till banor i sitt program (Inkscape: Bana > Objekt till bana; Illustrator: Skapa konturer) eller använd Chiploads egen text
svg-images-left-out = { $count ->
    [one] 1 bild utelämnad
   *[other] { $count } bilder utelämnade
}; en bild att spåra läggs till för sig
svg-clips-ignored = { $count ->
    [one] 1 urklippsbana eller mask ignorerad
   *[other] { $count } urklippsbanor eller masker ignorerade
}: det de klipper importeras helt
svg-hidden-paths = { $count ->
    [one] 1 bana som inte syns i filen (utan fyllning och kontur, eller dold) importerades
   *[other] { $count } banor som inte syns i filen (utan fyllning och kontur, eller dolda) importerades
}
svg-empty-dropped = { $count ->
    [one] 1 delbana utan längd utelämnad
   *[other] { $count } delbanor utan längd utelämnade
}
