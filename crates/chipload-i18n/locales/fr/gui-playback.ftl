# The playback bar under the canvas (crates/chipload-gui/src/app/playback.rs).

playback-play-hover = Lire le programme à la vitesse choisie

# The readout under the slider, its pieces separated by " · ".
# $time and $total: durations, e.g. "12 min 30 s of 41 min".
playback-time-of = { $time } sur { $total }
# $operation: the operation's number and name, e.g. "Operation 3 (Outline)".
playback-operation-copy = { $operation }, copie { $copy }
# $position: where the tool is, e.g. "X 10.000 Y 5.000 Z -2.000 mm".
playback-rapid = { $position }, rapide
# $feed: the feed rate, a number in the display units.
playback-cutting = { $position }, F { $feed }, en coupe
