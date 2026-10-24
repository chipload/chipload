# File > Import image… and the window that traces a picture into shapes
# (crates/chipload-gui/src/app/images.rs).

# The name of picture files in the file dialog's filter.
images-pictures-filter = Immagini
images-cannot-read = Impossibile leggere { $path }: { $error }
# Followed by the error on its own line.
images-cannot-import = Impossibile importare { $path }:
# Followed by the error on its own line.
images-trace-at-size-failed = Impossibile vettorizzare l'immagine a quella dimensione:
images-tracing-failed = vettorizzazione dell'immagine non riuscita
# After a picture is traced into the project. "V-carve (ribbon)" is the ribbon's V-carve button.
images-traced-next = L'immagine è selezionata: Incisione a V (barra multifunzione) la incide. Imposta dimensione e posizione in Proprietà.
images-traced = Vettorizzata { $file }
images-removed = Rimossa immagine { $id }

# Window titles. $file: the picture's file name.
images-import-title = Importa immagine: { $file }
# "Trace" is a verb here: trace this picture again.
images-trace-title = Vettorizza { $file }
# The window's button, a verb.
images-import = Importa

# Under the preview. $zoom: such as "400%".
images-zoom-note = Zoom { $zoom } · trascina per spostare · doppio clic per adattare
images-scroll-to-zoom = Scorri per ingrandire

# The settings.
images-threshold = Soglia
images-threshold-hover = I valori di grigio sotto la soglia sono neri (0 nero, 255 bianco). Auto sceglie il valore che separa meglio le parti scure da quelle chiare.
# Checkbox: pick the threshold automatically.
images-threshold-auto = auto
images-invert = Inverti
images-invert-check = incidi le parti chiare
images-blur = Sfocatura
images-blur-hover = Leviga bordi frastagliati e rumore prima della soglia.
images-smallest-piece = Pezzetto minimo
images-smallest-piece-hover = Le macchioline nere o bianche più piccole di così vengono scartate.
images-smoothing = Levigatura
images-smoothing-hover = Di quanto un bordo vettorizzato può essere raddrizzato rispetto al bordo dell'immagine.
# What tracing makes, a noun.
images-trace = Tracciato
images-trace-hover = Contorni attorno alle parti nere, per un'incisione a V; oppure le linee mediane dei tratti neri (scrittura a mano, disegni al tratto), per un segui percorso.
images-trace-outlines = Contorni (incisione a V)
images-trace-centre-lines = Linee mediane (segui percorso)
images-arcs = Archi
images-arcs-hover = Approssima con archi oltre che con linee: programmi più piccoli e moto più fluido per contornature, tasche e segui percorso.
images-arcs-check = usa archi

# What the preview traced.
images-tracing = Vettorizzazione…
images-found-shapes = { $count ->
        [one] { $count } forma
       *[other] { $count } forme
    }
# $threshold: the grey value the automatic threshold picked, 1 to 254.
images-found-shapes-auto = { $count ->
        [one] { $count } forma
       *[other] { $count } forme
    } (scelta soglia { $threshold })
images-found-centre-lines = { $count ->
        [one] { $count } linea mediana
       *[other] { $count } linee mediane
    }
images-found-centre-lines-auto = { $count ->
        [one] { $count } linea mediana
       *[other] { $count } linee mediane
    } (scelta soglia { $threshold })
# The narrowest and widest strokes; $wide carries the unit.
images-stroke-widths = Tratti larghi da { $narrow } a { $wide }
images-dots-left-out = { $count } punti esclusi
# Traced pieces where no arcs were fitted.
images-kept-as-lines = { $count } lasciati come linee
images-specks-dropped = { $count } macchioline scartate
images-black-at-border = Il nero tocca il bordo: inverti se è lo sfondo.
