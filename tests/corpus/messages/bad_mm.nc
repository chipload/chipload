(Deliberately wrong G-code for the message snapshot: every line breaks a rule.)
G21 G90
M3 S18000
G0 X10 Y10 Z5
G1 Z-3 F0
G0 X50 Y50
G1 X900 Y10 F1000
G1 Z-50 F300
G2 X20 Y10 I3 J0 F800
G3 X0 Y0 R0.1
G1 X10 Q7 F500
G0 Z-2
G0 X0 Y0
G20
G1 X1 Y1 F10
M5
M30
