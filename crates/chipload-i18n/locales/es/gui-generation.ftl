# Generating the toolpath and G-code, and the live preview
# (crates/chipload-gui/src/app/generation.rs).

generation-no-project = Importa un DXF o SVG o abre un proyecto primero.
generation-could-not-start = No se pudo iniciar el cálculo: { $error }
generation-cancelled = Cálculo cancelado
# What went wrong when the planning stopped without an answer.
generation-worker-stopped = el proceso de cálculo se detuvo sin resultado

generation-check-failed = La comprobación del código G FALLÓ. No se puede exportar nada.
# Followed by the reason on the next line.
generation-failed = El cálculo falló:
generation-failed-unexpectedly = El cálculo falló de forma inesperada: { $error }
generation-please-report = Tu proyecto no ha cambiado. Informa de esto adjuntando el archivo del proyecto.

# $post: the post-processor's name, e.g. GRBL.
generation-done = Se calcularon { $lines } líneas para { $post }. Comprobación superada.
generation-done-files = Se calcularon { $lines } líneas en { $files } archivos, uno por grupo de herramienta, para { $post }. Comprobación superada.
# A warning from the G-code checker.
generation-check = Comprobación: { $message }
# Where X, Y and Z zero are on the stock.
generation-work-zero = Cero pieza: { $zero }
# "All parts" is a switch on the bar above the canvas.
generation-one-part-shown = Se muestra la pieza "{ $part }" sola, una vez, donde está dibujada (seleccione la sujeción o el proyecto para ver todas las piezas). Exportar escribe toda la sujeción.

generation-export-when-done = Se exportará cuando la trayectoria esté calculada
