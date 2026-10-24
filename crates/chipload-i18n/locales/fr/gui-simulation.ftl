# The material-removal simulation shown in the canvas (crates/chipload-gui/src/app/simulation.rs).
# "Stock" is the material being machined.

# Starts lower case: it follows "Simulation failed: ".
simulation-stopped-unexpectedly = la simulation s'est arrêtée de façon inattendue
simulation-could-not-start = La simulation n'a pas pu démarrer : { $error }
simulation-failed = Échec de la simulation : { $error }
# The slider plays the program back; for very long programs only the toolpath moves.
simulation-too-long-to-play = Le programme est trop long pour rejouer le brut ; le curseur ne montre que le parcours d'outil
