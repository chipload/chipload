# The playback bar under the canvas (crates/chipload-gui/src/app/playback.rs).

playback-play-hover = Play the program at the chosen speed

# The readout under the slider, its pieces separated by " · ".
# $time and $total: durations, e.g. "12 min 30 s of 41 min".
playback-time-of = { $time } of { $total }
# $operation: the operation's number and name, e.g. "Operation 3 (Outline)".
playback-operation-copy = { $operation }, copy { $copy }
# $position: where the tool is, e.g. "X 10.000 Y 5.000 Z -2.000 mm".
playback-rapid = { $position }, rapid
# $feed: the feed rate, a number in the display units.
playback-cutting = { $position }, F { $feed }, cutting
