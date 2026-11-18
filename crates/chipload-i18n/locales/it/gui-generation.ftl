# Generating the toolpath and G-code, and the live preview
# (crates/chipload-gui/src/app/generation.rs).

generation-no-project = Importa prima un DXF o SVG o apri un progetto.
generation-could-not-start = Impossibile avviare il calcolo: { $error }
generation-cancelled = Calcolo annullato
# What went wrong when the planning stopped without an answer.
generation-worker-stopped = il processo di calcolo si è fermato senza un risultato

generation-check-failed = Verifica del G-code NON SUPERATA. Non si può esportare nulla.
# Followed by the reason on the next line.
generation-failed = Calcolo non riuscito:
generation-failed-unexpectedly = Calcolo non riuscito in modo imprevisto: { $error }
generation-please-report = Il tuo progetto è invariato. Segnala il problema allegando il file di progetto.

# $post: the post-processor's name, e.g. GRBL.
generation-done = Calcolate { $lines } righe per { $post }. Verifica superata.
generation-done-files = Calcolate { $lines } righe in { $files } file, uno per gruppo di utensili, per { $post }. Verifica superata.
# A warning from the G-code checker.
generation-check = Verifica: { $message }
# Where X, Y and Z zero are on the stock.
generation-work-zero = Zero pezzo: { $zero }
# "All parts" is a switch on the bar above the canvas.
generation-one-part-shown = Mostrato: il pezzo "{ $part }" da solo, una volta, dove è disegnato (seleziona il piazzamento o il progetto per vedere tutti i pezzi). Esporta scrive tutto il piazzamento.

generation-export-when-done = Esportazione appena il percorso utensile è calcolato
