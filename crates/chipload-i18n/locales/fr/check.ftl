# Findings of the independent G-code checker (crates/chipload-check): what it found wrong
# or worth a warning in a G-code program. Words of G-code (G0, G21, M6, F, P, T, X, Z…)
# are never translated. A length argument brings its own unit ("-3.000 mm"); a number
# written after a G-code letter without a unit is the program's own number.

## Reading a line

check-unbalanced-comment = Unbalanced or nested comment
# $text is the part of the line that could not be read, in quotes.
check-cannot-parse = Cannot parse { $text }
check-after-end = Code after program end (M30)
check-absolute-arc-centres = G90.1 (absolute arc centres) is not supported
# $word is a G or M code, such as G43 or M7.5.
check-not-supported = { $word } is not supported
# $letter is a letter of G-code, such as X or F.
check-word-twice = { $letter } appears twice on one line
check-unsupported-word = Unsupported word { $letter }
check-several-motions = More than one motion command on one line

## Units and modes

# $code is G20 or G21; $units the units it sets, "mm" or "inch".
check-units-changed = { $code } changes the units to { $units ->
        [inch] inch
       *[mm] mm
    } within the program
# $code is G20 or G21, $units the units it sets and $machine the machine's G-code units,
# each "mm" or "inch".
check-units-differ = { $code } ({ $units ->
        [inch] inch
       *[mm] mm
    }) but the machine's G-code units are { $machine ->
        [inch] inch
       *[mm] mm
    }
# $feed is the number after F.
check-feed-not-positive = Feed F{ $feed } must be positive
check-relative = G91 (relative) is not supported
# $tool is the number after T.
check-tool-not-whole = T{ $tool } must be a whole tool number
check-start-without-speed = Spindle started after a tool change without a speed (S)
check-dwell-without-p = G4 dwell without P
check-blend-and-dwell = G64 and G4 on one line: P is ambiguous
# G64 P lets the controller round corners. $p is the number after P, $most a length.
check-blend-too-far = G64 P{ $p } lets the controller leave the path by more than { $most }
# $most is the largest number P may have.
check-blend-without-p = G64 without P lets the controller round corners without limit (use P at most { $most })
check-no-motion-mode = Axis words without a motion mode

## Tool changes

check-change-without-tool = Tool change (M6) without a tool number (T)
# $tool is the tool number after T.
check-change-unknown-tool = Tool change (M6) to T{ $tool }, not a tool of the project
check-change-spindle-on = Tool change (M6) with the spindle on (stop it with M5 first)
check-change-below-safe = Tool change (M6) below safe height (retract first)

## Moves

check-motion-before-units = Motion before G21 (mm) or G20 (inch)
check-motion-before-absolute = Motion before G90 (absolute)
check-rapid-below-safe = Rapid XY move while Z is below safe height (retract first)
# $z is a height with its unit, written after Z: "Z-3.000 mm".
check-rapid-into-material = Rapid to Z{ $z }, below the stock top (rapid into material)
check-cut-without-feed = Cutting move with no feed set
check-cut-spindle-off = Cutting move with the spindle off
check-cut-unknown-position = Cutting move from an unknown position
# $z is a height with its unit, written after Z; $bottom the stock bottom's height and
# $allowed how far below it moves may go.
check-below-stock-too-far = Z{ $z } is below the stock bottom ({ $bottom }) by more than the allowed { $allowed }
check-arc-without-centre = Arc without I/J centre offsets
check-arc-unknown-position = Arc from an unknown position
check-arc-full-circle = Arc ending where it starts (a full circle in one move)
# $start and $end are the arc's radius at its two ends.
check-arc-radius-mismatch = Arc radius mismatch: start { $start } vs end { $end }

## The whole program

# $deepest is the lowest height, written after Z; $bottom the stock bottom's height and
# $allowed how far below it moves may go.
check-below-stock = { $count } moves go below the stock bottom (deepest Z{ $deepest }, bottom { $bottom }), within the allowed { $allowed }
check-outside-stock = { $count } cutting moves end outside the stock (expanded by the tool radius)
check-no-travel = Machine travel not set; travel limits were not checked
# $axis is X, Y or Z; $span how far the program moves along it, $travel how far the
# machine can.
check-travel-exceeded = { $axis } span { $span } exceeds machine travel { $travel }
check-no-end = No program end (M30)
