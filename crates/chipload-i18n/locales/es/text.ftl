# Messages of the lettering (crates/chipload-text): why a text can't be turned into
# letters. Font names are never translated.

# $fonts is the built-in fonts' names, separated by commas.
text-error-unknown-font = no hay ninguna fuente incluida "{ $font }"; las fuentes son: { $fonts }
text-error-empty = el texto está vacío: escribe algo que cortar
text-error-too-long = el texto es demasiado largo: { $count } caracteres, como máximo { $most }
# $zero is the length 0 with its unit; $got the height asked for, without its unit.
text-error-height = la altura debe ser más de { $zero }, es { $got }
# Spacings are factors, without a unit.
text-error-line-spacing = el interlineado debe ser más de 0, es { $got }
text-error-spacing = el espaciado de letras debe ser un número, es { $got }
text-error-bad-font = no se puede leer la fuente "{ $font }"
text-error-no-letters = nada que cortar: la fuente no tiene letras para este texto
