# Generating the toolpath and G-code, and the live preview
# (crates/chipload-gui/src/app/generation.rs).

generation-no-project = Importera en DXF eller öppna ett projekt först.
generation-could-not-start = Beräkningen kunde inte starta: { $error }
generation-cancelled = Beräkningen avbröts
# What went wrong when the planning stopped without an answer.
generation-worker-stopped = arbetstråden stannade utan resultat

generation-check-failed = G-kodskontrollen MISSLYCKADES. Inget kan exporteras.
# Followed by the reason on the next line.
generation-failed = Beräkningen misslyckades:
generation-failed-unexpectedly = Beräkningen misslyckades oväntat: { $error }
generation-please-report = Ditt projekt är oförändrat. Rapportera gärna detta med projektfilen.

# $post: the post-processor's name, e.g. GRBL.
generation-done = Beräknade { $lines } rader för { $post }. Kontrollen godkänd.
generation-done-files = Beräknade { $lines } rader i { $files } filer, en per verktygsgrupp, för { $post }. Kontrollen godkänd.
# A warning from the G-code checker.
generation-check-warning = Kontroll: { $message }
# Where X, Y and Z zero are on the stock.
generation-work-zero = Arbetsnollpunkt: { $zero }
# "All parts" is a switch on the bar above the canvas.
generation-one-part-shown = Visas: bara detaljen "{ $part }" (Alla detaljer på ritytans list visar alla detaljer). Export skriver alla detaljer som fräses.

generation-export-when-done = Exporterar när verktygsbanan har beräknats
