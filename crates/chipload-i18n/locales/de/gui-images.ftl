# File > Import image… and the window that traces a picture into shapes
# (crates/chipload-gui/src/app/images.rs).

# The name of picture files in the file dialog's filter.
images-pictures-filter = Bilder
images-cannot-read = { $path } kann nicht gelesen werden: { $error }
# Followed by the error on its own line.
images-cannot-import = { $path } kann nicht importiert werden:
# Followed by the error on its own line.
images-trace-at-size-failed = Das Bild konnte in dieser Größe nicht nachgezeichnet werden:
images-tracing-failed = das Nachzeichnen des Bildes ist fehlgeschlagen
# After a picture is traced into the project. "V-carve (ribbon)" is the ribbon's V-carve button.
images-traced-next = Das Bild ist ausgewählt: V-Gravur (Menüband) graviert es. Größe und Lage unter Eigenschaften einstellen.
images-traced = { $file } nachgezeichnet
images-removed = Bild { $id } entfernt

# Window titles. $file: the picture's file name.
images-import-title = Bild importieren: { $file }
# "Trace" is a verb here: trace this picture again.
images-trace-title = { $file } nachzeichnen
# The window's button, a verb.
images-import = Importieren

# Under the preview. $zoom: such as "400%".
images-zoom-note = Zoom { $zoom } · ziehen zum Verschieben · Doppelklick zum Einpassen
images-scroll-to-zoom = Scrollen zum Vergrößern

# The settings.
images-threshold = Schwellwert
images-threshold-hover = Grauwerte darunter werden schwarz (0 schwarz, 255 weiß). Auto wählt den Wert, der dunkle und helle Bereiche am besten trennt.
# Checkbox: pick the threshold automatically.
images-threshold-auto = auto
images-invert = Invertieren
images-invert-check = die hellen Bereiche gravieren
images-blur = Weichzeichnen
images-blur-hover = Glättet ausgefranste Kanten und Rauschen vor dem Schwellwert.
images-smallest-piece = Kleinstes Stück
images-smallest-piece-hover = Schwarze oder weiße Flecken kleiner als dies werden verworfen.
images-smoothing = Glättung
images-smoothing-hover = Wie weit eine nachgezeichnete Kante von der Kante im Bild weg begradigt werden darf.
# What tracing makes, a noun.
images-trace = Ergebnis
images-trace-hover = Umrisse um die schwarzen Bereiche, für eine V-Gravur; oder die Mittellinien schwarzer Striche (Handschrift, Strichzeichnungen), für Pfad folgen.
images-trace-outlines = Umrisse (V-Gravur)
images-trace-centre-lines = Mittellinien (Pfad folgen)
images-arcs = Bögen
images-arcs-hover = Neben Linien auch Bögen anpassen: kleinere Programme und ruhigere Bewegung für Konturen, Taschen und Pfad folgen.
images-arcs-check = Bögen anpassen

# What the preview traced.
images-tracing = Wird nachgezeichnet…
images-found-shapes = { $count ->
        [one] { $count } Form
       *[other] { $count } Formen
    }
# $threshold: the grey value the automatic threshold picked, 1 to 254.
images-found-shapes-auto = { $count ->
        [one] { $count } Form
       *[other] { $count } Formen
    } (Schwellwert { $threshold } gewählt)
images-found-centre-lines = { $count ->
        [one] { $count } Mittellinie
       *[other] { $count } Mittellinien
    }
images-found-centre-lines-auto = { $count ->
        [one] { $count } Mittellinie
       *[other] { $count } Mittellinien
    } (Schwellwert { $threshold } gewählt)
# The narrowest and widest strokes; $wide carries the unit.
images-stroke-widths = Striche { $narrow } bis { $wide } breit
images-dots-left-out = { $count } Punkte ausgelassen
# Traced pieces where no arcs were fitted.
images-kept-as-lines = { $count } als Linien behalten
images-specks-dropped = { $count } Flecken verworfen
images-black-at-border = Schwarz berührt den Rand: invertieren, falls es der Hintergrund ist.
