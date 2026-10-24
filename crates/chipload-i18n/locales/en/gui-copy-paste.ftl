# Copy, paste and mirror drawn shapes (crates/chipload-gui/src/app/copy_paste.rs).
# Drawn shapes are drawn in Chipload; imported ones come from a DXF, a picture or a text.

copy-paste-imported-no-copy = Imported shapes can't be copied; only drawn ones
copy-paste-select-to-copy = Select drawn shapes to copy
copy-paste-copied = { $count ->
    [one] Copied { $count } shape
   *[other] Copied { $count } shapes
}

copy-paste-no-shapes = The clipboard holds no shapes
copy-paste-copy-first = Copy drawn shapes first (Ctrl+C)
# $shapes: the new shapes' names, e.g. S4, S5.
copy-paste-pasted = Pasted { $shapes }
# Followed by the reason on the next line.
copy-paste-cannot-paste = Cannot paste:

copy-paste-imported-no-mirror = Imported shapes can't be mirrored; only drawn ones
copy-paste-select-to-mirror = Select drawn shapes to mirror
copy-paste-mirrored = Mirrored
copy-paste-mirrored-drawn = Mirrored (imported shapes in the selection stay as they are)
# Followed by the reason on the next line.
copy-paste-cannot-mirror = Cannot mirror:
