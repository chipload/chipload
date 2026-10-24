# Messages of the lettering (crates/chipload-text): why a text can't be turned into
# letters. Font names are never translated.

# $fonts is the built-in fonts' names, separated by commas.
text-error-unknown-font = non esiste un font integrato "{ $font }"; i font sono: { $fonts }
text-error-empty = il testo è vuoto: scrivi qualcosa da tagliare
text-error-too-long = il testo è troppo lungo: { $count } caratteri, al massimo { $most }
# $zero is the length 0 with its unit; $got the height asked for, without its unit.
text-error-height = l'altezza deve essere più di { $zero }, trovato { $got }
# Spacings are factors, without a unit.
text-error-line-spacing = l'interlinea deve essere più di 0, trovato { $got }
text-error-spacing = la spaziatura tra le lettere deve essere un numero, trovato { $got }
text-error-bad-font = impossibile leggere il font "{ $font }"
text-error-no-letters = niente da tagliare: il font non ha lettere per questo testo
