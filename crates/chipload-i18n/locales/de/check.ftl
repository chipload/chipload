# Findings of the independent G-code checker (crates/chipload-check): what it found wrong
# or worth a warning in a G-code program. Words of G-code (G0, G21, M6, F, P, T, X, Z…)
# are never translated. A length argument brings its own unit ("-3.000 mm"); a number
# written after a G-code letter without a unit is the program's own number.

## Reading a line

check-unbalanced-comment = Unausgeglichener oder verschachtelter Kommentar
# $text is the part of the line that could not be read, in quotes.
check-cannot-parse = { $text } kann nicht gelesen werden
check-after-end = Code nach dem Programmende (M30)
check-absolute-arc-centres = G90.1 (absolute Kreismittelpunkte) wird nicht unterstützt
# $word is a G or M code, such as G43 or M7.5.
check-not-supported = { $word } wird nicht unterstützt
# $letter is a letter of G-code, such as X or F.
check-word-twice = { $letter } kommt zweimal in einer Zeile vor
check-unsupported-word = Nicht unterstütztes Wort { $letter }
check-several-motions = Mehr als ein Bewegungsbefehl in einer Zeile

## Units and modes

# $code is G20 or G21; $units the units it sets, "mm" or "inch".
check-units-changed = { $code } wechselt innerhalb des Programms die Einheiten auf { $units ->
        [inch] Zoll
       *[mm] mm
    }
# $code is G20 or G21, $units the units it sets and $machine the machine's G-code units,
# each "mm" or "inch".
check-units-differ = { $code } ({ $units ->
        [inch] Zoll
       *[mm] mm
    }), aber die G-Code-Einheiten der Maschine sind { $machine ->
        [inch] Zoll
       *[mm] mm
    }
# $feed is the number after F.
check-feed-not-positive = Vorschub F{ $feed } muss positiv sein
check-relative = G91 (relativ) wird nicht unterstützt
# $tool is the number after T.
check-tool-not-whole = T{ $tool } muss eine ganze Werkzeugnummer sein
check-start-without-speed = Spindel nach einem Werkzeugwechsel ohne Drehzahl (S) gestartet
check-dwell-without-p = G4-Verweilzeit ohne P
check-blend-and-dwell = G64 und G4 in einer Zeile: P ist mehrdeutig
# G64 P lets the controller round corners. $p is the number after P, $most a length.
check-blend-too-far = G64 P{ $p } lässt die Steuerung um mehr als { $most } von der Bahn abweichen
# $most is the largest number P may have.
check-blend-without-p = G64 ohne P lässt die Steuerung Ecken unbegrenzt verrunden (P höchstens { $most } verwenden)
check-no-motion-mode = Achswörter ohne Bewegungsart

## Tool changes

check-change-without-tool = Werkzeugwechsel (M6) ohne Werkzeugnummer (T)
# $tool is the tool number after T.
check-change-unknown-tool = Werkzeugwechsel (M6) auf T{ $tool }, kein Werkzeug des Projekts
check-change-spindle-on = Werkzeugwechsel (M6) bei laufender Spindel (zuerst mit M5 stoppen)
check-change-below-safe = Werkzeugwechsel (M6) unterhalb der Sicherheitshöhe (zuerst zurückziehen)

## Moves

check-motion-before-units = Bewegung vor G21 (mm) oder G20 (inch)
check-motion-before-absolute = Bewegung vor G90 (absolut)
check-rapid-below-safe = XY-Eilgang, während Z unterhalb der Sicherheitshöhe ist (zuerst zurückziehen)
# $z is a height with its unit, written after Z: "Z-3.000 mm".
check-rapid-into-material = Eilgang auf Z{ $z }, unterhalb der Rohteiloberkante (Eilgang ins Material)
check-cut-without-feed = Schnittbewegung ohne gesetzten Vorschub
check-cut-spindle-off = Schnittbewegung bei ausgeschalteter Spindel
check-cut-unknown-position = Schnittbewegung von unbekannter Position
# $z is a height with its unit, written after Z; $bottom the stock bottom's height and
# $allowed how far below it moves may go.
check-below-stock-too-far = Z{ $z } liegt mehr als die erlaubten { $allowed } unter der Rohteilunterkante ({ $bottom })
check-arc-without-centre = Kreisbogen ohne Mittelpunktsabstände I/J
check-arc-unknown-position = Kreisbogen von unbekannter Position
check-arc-full-circle = Kreisbogen endet an seinem Anfang (ein Vollkreis in einer Bewegung)
# $start and $end are the arc's radius at its two ends.
check-arc-radius-mismatch = Kreisbogenradien stimmen nicht überein: Anfang { $start }, Ende { $end }

## The whole program

# $deepest is the lowest height, written after Z; $bottom the stock bottom's height and
# $allowed how far below it moves may go.
check-below-stock = { $count ->
    [one] { $count } Bewegung geht
   *[other] { $count } Bewegungen gehen
} unter die Rohteilunterkante (tiefstes Z{ $deepest }, Unterkante { $bottom }), innerhalb der erlaubten { $allowed }
check-outside-stock = { $count ->
    [one] { $count } Schnittbewegung endet
   *[other] { $count } Schnittbewegungen enden
} außerhalb des Rohteils (um den Werkzeugradius erweitert)
check-no-travel = Verfahrweg der Maschine nicht gesetzt; Verfahrweggrenzen wurden nicht geprüft
# $axis is X, Y or Z; $span how far the program moves along it, $travel how far the
# machine can.
check-travel-exceeded = { $axis }-Bereich { $span } überschreitet den Verfahrweg der Maschine { $travel }
check-no-end = Kein Programmende (M30)
