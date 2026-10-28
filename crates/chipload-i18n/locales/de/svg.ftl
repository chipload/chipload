# SVG-Import (docs/specs/svg-import.md).
svg-error-too-large = das SVG ist größer als { $most }
svg-error-unreadable = kein lesbares SVG: { $reason }
svg-error-crashed = der SVG-Parser ist an dieser Datei abgestürzt
svg-error-too-deep = Elemente sind tiefer als { $most } verschachtelt
svg-error-needs-scale = das SVG hat keine echte Größe (nur Pixel): wählen Sie seine Pixel pro Zoll, 96 (Inkscape, Browser) oder 72 (älteres Illustrator)
svg-error-scale = der Maßstab oder die Pixel pro Zoll müssen größer als 0 sein, ist { $got }
svg-error-too-many = das SVG hat { $count } Formen, mehr als { $most }
svg-error-far-out = { $element } hat einen Punkt, der keine Zahl ist oder mehr als { $most } vom Ursprung entfernt liegt
svg-px-chosen = das SVG hat keine echte Größe; gelesen mit { $px } Pixeln pro Zoll, wie gewählt
svg-text-left-out = { $count ->
    [one] 1 Text ausgelassen
   *[other] { $count } Texte ausgelassen
}: wandeln Sie Text in seinem Editor in Pfade um (Inkscape: Pfad > Objekt in Pfad umwandeln; Illustrator: In Pfade umwandeln) oder nutzen Sie Chiploads eigenen Text
svg-images-left-out = { $count ->
    [one] 1 Bild ausgelassen
   *[other] { $count } Bilder ausgelassen
}; ein Bild zum Nachzeichnen wird eigens hinzugefügt
svg-clips-ignored = { $count ->
    [one] 1 Beschneidungspfad oder Maske ignoriert
   *[other] { $count } Beschneidungspfade oder Masken ignoriert
}: was sie beschneiden, wird ganz importiert
svg-hidden-paths = { $count ->
    [one] 1 Pfad, der in der Datei nicht sichtbar ist (ohne Füllung und Kontur oder verborgen), wurde importiert
   *[other] { $count } Pfade, die in der Datei nicht sichtbar sind (ohne Füllung und Kontur oder verborgen), wurden importiert
}
svg-empty-dropped = { $count ->
    [one] 1 Teilpfad ohne Länge ausgelassen
   *[other] { $count } Teilpfade ohne Länge ausgelassen
}
