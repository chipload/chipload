# File > Import image… and the window that traces a picture into shapes
# (crates/chipload-gui/src/app/images.rs).

# The name of picture files in the file dialog's filter.
images-pictures-filter = Bilder
images-cannot-read = Kan inte läsa { $path }: { $error }
# Followed by the error on its own line.
images-cannot-import = Kan inte importera { $path }:
# Followed by the error on its own line.
images-trace-at-size-failed = Kunde inte spåra bilden i den storleken:
images-tracing-failed = spårningen av bilden misslyckades
# After a picture is traced into the project. "V-carve (ribbon)" is the ribbon's V-carve button.
images-traced-next = Bilden är markerad: V-gravyr (verktygsfältet) graverar den. Ange storlek och läge i Egenskaper.
images-traced = Spårade { $file }
images-removed = Tog bort bild { $id }

# Window titles. $file: the picture's file name.
images-import-title = Importera bild: { $file }
# "Trace" is a verb here: trace this picture again.
images-trace-title = Spåra { $file }
# The window's button, a verb.
images-import = Importera

# Under the preview. $zoom: such as "400%".
images-zoom-note = Zoom { $zoom } · dra för att panorera · dubbelklicka för att anpassa
images-scroll-to-zoom = Scrolla för att zooma in

# The settings.
images-threshold = Tröskel
images-threshold-hover = Gråvärden under den blir svarta (0 svart, 255 vit). Auto väljer det värde som bäst skiljer mörka och ljusa delar.
# Checkbox: pick the threshold automatically.
images-threshold-auto = auto
images-invert = Invertera
images-invert-check = gravera de ljusa delarna
images-blur = Oskärpa
images-blur-hover = Jämnar ut ojämna kanter och brus före tröskeln.
images-smallest-piece = Minsta bit
images-smallest-piece-hover = Svarta eller vita prickar mindre än detta tas bort.
images-smoothing = Utjämning
images-smoothing-hover = Hur långt en spårad kant får rätas ut från bildens kant.
# What tracing makes, a noun.
images-trace = Spårning
images-trace-hover = Konturer runt de svarta delarna, för V-gravyr; eller mittlinjer i svarta streck (handskrift, linjeteckningar), för Följ bana.
images-trace-outlines = Konturer (V-gravyr)
images-trace-centre-lines = Mittlinjer (följ bana)
images-arcs = Bågar
images-arcs-hover = Anpassa bågar såväl som linjer: mindre program och mjukare rörelser för konturfräsning, fickor och följ bana.
images-arcs-check = anpassa bågar

# What the preview traced.
images-tracing = Spårar…
images-found-shapes = { $count ->
        [one] { $count } form
       *[other] { $count } former
    }
# $threshold: the grey value the automatic threshold picked, 1 to 254.
images-found-shapes-auto = { $count ->
        [one] { $count } form
       *[other] { $count } former
    } (tröskel { $threshold } vald)
images-found-centre-lines = { $count ->
        [one] { $count } mittlinje
       *[other] { $count } mittlinjer
    }
images-found-centre-lines-auto = { $count ->
        [one] { $count } mittlinje
       *[other] { $count } mittlinjer
    } (tröskel { $threshold } vald)
# The narrowest and widest strokes; $wide carries the unit.
images-stroke-widths = Streck { $narrow } till { $wide } breda
images-dots-left-out = { $count } punkter utelämnade
# Traced pieces where no arcs were fitted.
images-kept-as-lines = { $count } behölls som linjer
images-specks-dropped = { $count } prickar borttagna
images-black-at-border = Svart når kanten: invertera om det är bakgrunden.
