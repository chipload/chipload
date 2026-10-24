# The material-removal simulation shown in the canvas (crates/chipload-gui/src/app/simulation.rs).
# "Stock" is the material being machined.

# Starts lower case: it follows "Simulation failed: ".
simulation-stopped-unexpectedly = die Simulation wurde unerwartet beendet
simulation-could-not-start = Simulation konnte nicht starten: { $error }
simulation-failed = Simulation fehlgeschlagen: { $error }
# The slider plays the program back; for very long programs only the toolpath moves.
simulation-too-long-to-play = Das Programm ist zu lang, um das Rohteil abzuspielen; der Schieberegler zeigt nur den Werkzeugweg
