# Generating the toolpath and G-code, and the live preview
# (crates/chipload-gui/src/app/generation.rs).

generation-no-project = Import a DXF or open a project first.
generation-could-not-start = Generation could not start: { $error }
generation-cancelled = Generation cancelled
# What went wrong when the planning stopped without an answer.
generation-worker-stopped = the worker stopped without a result

generation-check-failed = G-code check FAILED. Nothing can be exported.
# Followed by the reason on the next line.
generation-failed = Generation failed:
generation-failed-unexpectedly = Generation failed unexpectedly: { $error }
generation-please-report = Your project is unchanged. Please report this with the project file.

# $post: the post-processor's name, e.g. GRBL.
generation-done = Generated { $lines } lines for { $post }. Check passed.
generation-done-files = Generated { $lines } lines in { $files } files, one per tool group, for { $post }. Check passed.
# A warning from the G-code checker.
generation-check-warning = Check: { $message }
# Where X, Y and Z zero are on the stock.
generation-work-zero = Work zero: { $zero }
# "All parts" is a switch on the bar above the canvas.
generation-one-part-shown = Shown: part "{ $part }" only (All parts on the canvas bar shows every part). Export writes every part that is cut.

generation-export-when-done = Exporting once the toolpath is generated
