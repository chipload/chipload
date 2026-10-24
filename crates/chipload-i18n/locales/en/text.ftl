# Messages of the lettering (crates/chipload-text): why a text can't be turned into
# letters. Font names are never translated.

# $fonts is the built-in fonts' names, separated by commas.
text-error-unknown-font = there is no built-in font "{ $font }"; the fonts are: { $fonts }
text-error-empty = the text is empty: write something to cut
text-error-too-long = the text is too long: { $count } characters, at most { $most }
# $zero is the length 0 with its unit; $got the height asked for, without its unit.
text-error-height = the height must be more than { $zero }, got { $got }
# Spacings are factors, without a unit.
text-error-line-spacing = the line spacing must be more than 0, got { $got }
text-error-spacing = the letter spacing must be a number, got { $got }
text-error-bad-font = the font "{ $font }" can't be read
text-error-no-letters = nothing to cut: the font has no letters for this text
