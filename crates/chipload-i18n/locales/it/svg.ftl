# Importazione SVG (docs/specs/svg-import.md).
svg-error-too-large = l'SVG supera { $most }
svg-error-unreadable = non è un SVG leggibile: { $reason }
svg-error-crashed = il parser SVG si è bloccato su questo file
svg-error-too-deep = ci sono elementi annidati oltre { $most } livelli
svg-error-needs-scale = l'SVG non indica una dimensione reale (solo pixel): scegli i suoi pixel per pollice, 96 (Inkscape, browser) o 72 (vecchi Illustrator)
svg-error-scale = la scala o i pixel per pollice devono essere maggiori di 0, trovato { $got }
svg-error-too-many = l'SVG ha { $count } forme, più di { $most }
svg-error-far-out = { $element } ha un punto che non è un numero o si trova a più di { $most } dall'origine
svg-px-chosen = l'SVG non indica una dimensione reale; letto a { $px } pixel per pollice, come scelto
svg-text-left-out = { $count ->
    [one] 1 testo tralasciato
   *[other] { $count } testi tralasciati
}: converti il testo in tracciati nel suo editor (Inkscape: Tracciato > Da oggetto a tracciato; Illustrator: Crea profili) oppure usa il testo di Chipload
svg-images-left-out = { $count ->
    [one] 1 immagine tralasciata
   *[other] { $count } immagini tralasciate
}; un'immagine da vettorializzare si aggiunge a parte
svg-clips-ignored = { $count ->
    [one] 1 tracciato di ritaglio o maschera ignorato
   *[other] { $count } tracciati di ritaglio o maschere ignorati
}: ciò che ritagliano viene importato per intero
svg-hidden-paths = { $count ->
    [one] è stato importato 1 tracciato che non si vede nel file (senza riempimento né contorno, o nascosto)
   *[other] sono stati importati { $count } tracciati che non si vedono nel file (senza riempimento né contorno, o nascosti)
}
svg-empty-dropped = { $count ->
    [one] 1 sottotracciato senza lunghezza tralasciato
   *[other] { $count } sottotracciati senza lunghezza tralasciati
}
