# Copy, paste and mirror drawn shapes (crates/chipload-gui/src/app/copy_paste.rs).
# Drawn shapes are drawn in Chipload; imported ones come from a DXF, a picture or a text.

copy-paste-imported-no-copy = Importerade former kan inte kopieras; bara ritade
copy-paste-select-to-copy = Markera ritade former att kopiera
copy-paste-copied = { $count ->
    [one] Kopierade { $count } form
   *[other] Kopierade { $count } former
}

copy-paste-no-shapes = Urklipp innehåller inga former
copy-paste-copy-first = Kopiera ritade former först (Ctrl+C)
# $shapes: the new shapes' names, e.g. S4, S5.
copy-paste-pasted = Klistrade in { $shapes }
# Followed by the reason on the next line.
copy-paste-cannot-paste = Kan inte klistra in:

copy-paste-imported-no-mirror = Importerade former kan inte speglas; bara ritade
copy-paste-select-to-mirror = Markera ritade former att spegla
copy-paste-mirrored = Speglade
copy-paste-mirrored-drawn = Speglade (importerade former i markeringen förblir som de är)
# Followed by the reason on the next line.
copy-paste-cannot-mirror = Kan inte spegla:
