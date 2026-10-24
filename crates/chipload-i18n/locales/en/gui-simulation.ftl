# The material-removal simulation shown in the canvas (crates/chipload-gui/src/app/simulation.rs).
# "Stock" is the material being machined.

# Starts lower case: it follows "Simulation failed: ".
simulation-stopped-unexpectedly = the simulation stopped unexpectedly
simulation-could-not-start = Simulation could not start: { $error }
simulation-failed = Simulation failed: { $error }
# The slider plays the program back; for very long programs only the toolpath moves.
simulation-too-long-to-play = The program is too long to play back the stock; the slider shows the toolpath only
