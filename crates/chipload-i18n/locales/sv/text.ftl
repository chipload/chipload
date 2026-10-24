# Messages of the lettering (crates/chipload-text): why a text can't be turned into
# letters. Font names are never translated.

# $fonts is the built-in fonts' names, separated by commas.
text-error-unknown-font = det finns inget inbyggt typsnitt "{ $font }"; typsnitten är: { $fonts }
text-error-empty = texten är tom: skriv något att fräsa
text-error-too-long = texten är för lång: { $count } tecken, högst { $most }
# $zero is the length 0 with its unit; $got the height asked for, without its unit.
text-error-height = höjden måste vara mer än { $zero }, fick { $got }
# Spacings are factors, without a unit.
text-error-line-spacing = radavståndet måste vara mer än 0, fick { $got }
text-error-spacing = teckenavståndet måste vara ett tal, fick { $got }
text-error-bad-font = typsnittet "{ $font }" kan inte läsas
text-error-no-letters = inget att fräsa: typsnittet har inga bokstäver för den här texten
