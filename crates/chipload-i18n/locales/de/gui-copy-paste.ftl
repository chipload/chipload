# Copy, paste and mirror drawn shapes (crates/chipload-gui/src/app/copy_paste.rs).
# Drawn shapes are drawn in Chipload; imported ones come from a DXF, a picture or a text.

copy-paste-imported-no-copy = Importierte Formen können nicht kopiert werden, nur gezeichnete
copy-paste-select-to-copy = Gezeichnete Formen zum Kopieren auswählen
copy-paste-copied = { $count ->
    [one] { $count } Form kopiert
   *[other] { $count } Formen kopiert
}

copy-paste-no-shapes = Die Zwischenablage enthält keine Formen
copy-paste-copy-first = Zuerst gezeichnete Formen kopieren (Ctrl+C)
# $shapes: the new shapes' names, e.g. S4, S5.
copy-paste-pasted = { $shapes } eingefügt
# Followed by the reason on the next line.
copy-paste-cannot-paste = Einfügen nicht möglich:

copy-paste-imported-no-mirror = Importierte Formen können nicht gespiegelt werden, nur gezeichnete
copy-paste-select-to-mirror = Gezeichnete Formen zum Spiegeln auswählen
copy-paste-mirrored = Gespiegelt
copy-paste-mirrored-drawn = Gespiegelt (importierte Formen in der Auswahl bleiben unverändert)
# Followed by the reason on the next line.
copy-paste-cannot-mirror = Spiegeln nicht möglich:
