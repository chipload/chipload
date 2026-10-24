# Findings of the independent G-code checker (crates/chipload-check): what it found wrong
# or worth a warning in a G-code program. Words of G-code (G0, G21, M6, F, P, T, X, Z…)
# are never translated. A length argument brings its own unit ("-3.000 mm"); a number
# written after a G-code letter without a unit is the program's own number.

## Reading a line

check-unbalanced-comment = Obalanserad eller nästlad kommentar
# $text is the part of the line that could not be read, in quotes.
check-cannot-parse = Kan inte tolka { $text }
check-after-end = Kod efter programslut (M30)
check-absolute-arc-centres = G90.1 (absoluta bågcentrum) stöds inte
# $word is a G or M code, such as G43 or M7.5.
check-not-supported = { $word } stöds inte
# $letter is a letter of G-code, such as X or F.
check-word-twice = { $letter } förekommer två gånger på en rad
check-unsupported-word = Ord som inte stöds: { $letter }
check-several-motions = Mer än ett rörelsekommando på en rad

## Units and modes

# $code is G20 or G21; $units the units it sets, "mm" or "inch".
check-units-changed = { $code } byter enheter till { $units ->
        [inch] tum
       *[mm] mm
    } mitt i programmet
# $code is G20 or G21, $units the units it sets and $machine the machine's G-code units,
# each "mm" or "inch".
check-units-differ = { $code } ({ $units ->
        [inch] tum
       *[mm] mm
    }) men maskinens G-kodsenheter är { $machine ->
        [inch] tum
       *[mm] mm
    }
# $feed is the number after F.
check-feed-not-positive = Matningen F{ $feed } måste vara positiv
check-relative = G91 (relativ) stöds inte
# $tool is the number after T.
check-tool-not-whole = T{ $tool } måste vara ett heltaligt verktygsnummer
check-start-without-speed = Spindeln startad efter ett verktygsbyte utan varvtal (S)
check-dwell-without-p = G4-uppehåll utan P
check-blend-and-dwell = G64 och G4 på samma rad: P är tvetydigt
# G64 P lets the controller round corners. $p is the number after P, $most a length.
check-blend-too-far = G64 P{ $p } låter styrningen avvika från banan med mer än { $most }
# $most is the largest number P may have.
check-blend-without-p = G64 utan P låter styrningen runda hörn utan gräns (använd P högst { $most })
check-no-motion-mode = Axelord utan rörelseläge

## Tool changes

check-change-without-tool = Verktygsbyte (M6) utan verktygsnummer (T)
# $tool is the tool number after T.
check-change-unknown-tool = Verktygsbyte (M6) till T{ $tool }, som inte är ett verktyg i projektet
check-change-spindle-on = Verktygsbyte (M6) med spindeln igång (stoppa den med M5 först)
check-change-below-safe = Verktygsbyte (M6) under säker höjd (lyft först)

## Moves

check-motion-before-units = Rörelse före G21 (mm) eller G20 (tum)
check-motion-before-absolute = Rörelse före G90 (absolut)
check-rapid-below-safe = Snabbförflyttning i XY medan Z är under säker höjd (lyft först)
# $z is a height with its unit, written after Z: "Z-3.000 mm".
check-rapid-into-material = Snabbförflyttning till Z{ $z }, under ämnets ovansida (snabbförflyttning in i materialet)
check-cut-without-feed = Skärande rörelse utan inställd matning
check-cut-spindle-off = Skärande rörelse med spindeln avstängd
check-cut-unknown-position = Skärande rörelse från en okänd position
# $z is a height with its unit, written after Z; $bottom the stock bottom's height and
# $allowed how far below it moves may go.
check-below-stock-too-far = Z{ $z } ligger under ämnets botten ({ $bottom }) med mer än tillåtna { $allowed }
check-arc-without-centre = Båge utan I/J-centrumförskjutningar
check-arc-unknown-position = Båge från en okänd position
check-arc-full-circle = Båge som slutar där den börjar (en hel cirkel i en rörelse)
# $start and $end are the arc's radius at its two ends.
check-arc-radius-mismatch = Bågens radie stämmer inte: start { $start } mot slut { $end }

## The whole program

# $deepest is the lowest height, written after Z; $bottom the stock bottom's height and
# $allowed how far below it moves may go.
check-below-stock = { $count ->
    [one] { $count } rörelse går
   *[other] { $count } rörelser går
} under ämnets botten (djupast Z{ $deepest }, botten { $bottom }), inom tillåtna { $allowed }
check-outside-stock = { $count ->
    [one] { $count } skärande rörelse slutar
   *[other] { $count } skärande rörelser slutar
} utanför ämnet (utökat med verktygsradien)
check-no-travel = Maskinens slaglängd är inte angiven; slaglängdsgränserna kontrollerades inte
# $axis is X, Y or Z; $span how far the program moves along it, $travel how far the
# machine can.
check-travel-exceeded = { $axis }-omfånget { $span } överskrider maskinens slaglängd { $travel }
check-no-end = Inget programslut (M30)
