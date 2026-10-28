# Importación de SVG (docs/specs/svg-import.md).
svg-error-too-large = el SVG ocupa más de { $most }
svg-error-unreadable = no es un SVG legible: { $reason }
svg-error-crashed = el analizador de SVG falló con este archivo
svg-error-too-deep = hay elementos anidados a más de { $most } niveles
svg-error-needs-scale = el SVG no da un tamaño real (solo píxeles): elige sus píxeles por pulgada, 96 (Inkscape, navegadores) o 72 (Illustrator antiguo)
svg-error-scale = la escala o los píxeles por pulgada deben ser mayores que 0, es { $got }
svg-error-too-many = el SVG tiene { $count } formas, más de { $most }
svg-error-far-out = { $element } tiene un punto que no es un número o está a más de { $most } del origen
svg-px-chosen = el SVG no da un tamaño real; leído a { $px } píxeles por pulgada, como se eligió
svg-text-left-out = { $count ->
    [one] 1 texto omitido
   *[other] { $count } textos omitidos
}: convierte el texto en trazados en su editor (Inkscape: Trayecto > Objeto a trayecto; Illustrator: Crear contornos) o usa el texto propio de Chipload
svg-images-left-out = { $count ->
    [one] 1 imagen omitida
   *[other] { $count } imágenes omitidas
}; una imagen para vectorizar se añade aparte
svg-clips-ignored = { $count ->
    [one] 1 trazado de recorte o máscara ignorado
   *[other] { $count } trazados de recorte o máscaras ignorados
}: lo que recortan se importa entero
svg-hidden-paths = { $count ->
    [one] se importó 1 trazado que no se ve en el archivo (sin relleno ni contorno, u oculto)
   *[other] se importaron { $count } trazados que no se ven en el archivo (sin relleno ni contorno, u ocultos)
}
svg-empty-dropped = { $count ->
    [one] 1 subtrazado sin longitud omitido
   *[other] { $count } subtrazados sin longitud omitidos
}
