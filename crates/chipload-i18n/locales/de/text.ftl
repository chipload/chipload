# Messages of the lettering (crates/chipload-text): why a text can't be turned into
# letters. Font names are never translated.

# $fonts is the built-in fonts' names, separated by commas.
text-error-unknown-font = es gibt keine eingebaute Schrift „{ $font }“; die Schriften sind: { $fonts }
text-error-empty = der Text ist leer: schreiben Sie etwas zum Fräsen
text-error-too-long = der Text ist zu lang: { $count } Zeichen, höchstens { $most }
# $zero is the length 0 with its unit; $got the height asked for, without its unit.
text-error-height = die Höhe muss mehr als { $zero } sein, ist { $got }
# Spacings are factors, without a unit.
text-error-line-spacing = der Zeilenabstand muss mehr als 0 sein, ist { $got }
text-error-spacing = der Buchstabenabstand muss eine Zahl sein, ist { $got }
text-error-bad-font = die Schrift „{ $font }“ kann nicht gelesen werden
text-error-no-letters = nichts zu fräsen: die Schrift hat keine Buchstaben für diesen Text
