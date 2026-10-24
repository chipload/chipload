# Messages of the lettering (crates/chipload-text): why a text can't be turned into
# letters. Font names are never translated.

# $fonts is the built-in fonts' names, separated by commas.
text-error-unknown-font = il n'y a pas de police intégrée "{ $font }" ; les polices sont : { $fonts }
text-error-empty = le texte est vide : écrivez quelque chose à usiner
text-error-too-long = le texte est trop long : { $count ->
        [one] { $count } caractère
       *[other] { $count } caractères
    }, au plus { $most }
# $zero is the length 0 with its unit; $got the height asked for, without its unit.
text-error-height = la hauteur doit être supérieure à { $zero }, reçu { $got }
# Spacings are factors, without a unit.
text-error-line-spacing = l'interligne doit être supérieur à 0, reçu { $got }
text-error-spacing = l'interlettrage doit être un nombre, reçu { $got }
text-error-bad-font = impossible de lire la police "{ $font }"
text-error-no-letters = rien à usiner : la police n'a aucune lettre pour ce texte
