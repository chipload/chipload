# File > Import image… and the window that traces a picture into shapes
# (crates/chipload-gui/src/app/images.rs).

# The name of picture files in the file dialog's filter.
images-pictures-filter = Pictures
images-cannot-read = Cannot read { $path }: { $error }
# Followed by the error on its own line.
images-cannot-import = Cannot import { $path }:
# Followed by the error on its own line.
images-trace-at-size-failed = Could not trace the picture at that size:
images-tracing-failed = tracing the picture failed
# After a picture is traced into the project. "V-carve (ribbon)" is the ribbon's V-carve button.
images-traced-next = The picture is selected: V-carve (ribbon) carves it. Set its size and place in Properties.
images-traced = Traced { $file }
images-removed = Removed image { $id }

# Window titles. $file: the picture's file name.
images-import-title = Import image: { $file }
# "Trace" is a verb here: trace this picture again.
images-trace-title = Trace { $file }
# The window's button, a verb.
images-import = Import

# Under the preview. $zoom: such as "400%".
images-zoom-note = Zoom { $zoom } · drag to pan · double-click to fit
images-scroll-to-zoom = Scroll to zoom in

# The settings.
images-threshold = Threshold
images-threshold-hover = Grey values below it are black (0 black, 255 white). Auto picks the value that best separates the dark and light parts.
# Checkbox: pick the threshold automatically.
images-threshold-auto = auto
images-invert = Invert
images-invert-check = carve the light parts
images-blur = Blur
images-blur-hover = Smooths ragged edges and noise before the threshold.
images-smallest-piece = Smallest piece
images-smallest-piece-hover = Black or white specks smaller than this are dropped.
images-smoothing = Smoothing
images-smoothing-hover = How far a traced edge may be straightened away from the picture's edge.
# What tracing makes, a noun.
images-trace = Trace
images-trace-hover = Outlines round the black parts, for a V-carve; or the centre lines of black strokes (handwriting, line art), for a follow path.
images-trace-outlines = Outlines (V-carve)
images-trace-centre-lines = Centre lines (follow path)
images-arcs = Arcs
images-arcs-hover = Fit arcs as well as lines: smaller programs and smoother motion for profiles, pockets and follow paths.
images-arcs-check = fit arcs

# What the preview traced.
images-tracing = Tracing…
images-found-shapes = { $count ->
        [one] { $count } shape
       *[other] { $count } shapes
    }
# $threshold: the grey value the automatic threshold picked, 1 to 254.
images-found-shapes-auto = { $count ->
        [one] { $count } shape
       *[other] { $count } shapes
    } (threshold { $threshold } picked)
images-found-centre-lines = { $count ->
        [one] { $count } centre line
       *[other] { $count } centre lines
    }
images-found-centre-lines-auto = { $count ->
        [one] { $count } centre line
       *[other] { $count } centre lines
    } (threshold { $threshold } picked)
# The narrowest and widest strokes; $wide carries the unit.
images-stroke-widths = Strokes { $narrow } to { $wide } wide
images-dots-left-out = { $count } dots left out
# Traced pieces where no arcs were fitted.
images-kept-as-lines = { $count } kept as lines
images-specks-dropped = { $count } specks dropped
images-black-at-border = Black touches the edge: invert if it's the background.
