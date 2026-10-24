# Findings of the independent G-code checker (crates/chipload-check): what it found wrong
# or worth a warning in a G-code program. Words of G-code (G0, G21, M6, F, P, T, X, Z…)
# are never translated. A length argument brings its own unit ("-3.000 mm"); a number
# written after a G-code letter without a unit is the program's own number.

## Reading a line

check-unbalanced-comment = Commento non bilanciato o annidato
# $text is the part of the line that could not be read, in quotes.
check-cannot-parse = Impossibile leggere { $text }
check-after-end = Codice dopo la fine del programma (M30)
check-absolute-arc-centres = G90.1 (centri d'arco assoluti) non è supportato
# $word is a G or M code, such as G43 or M7.5.
check-not-supported = { $word } non è supportato
# $letter is a letter of G-code, such as X or F.
check-word-twice = { $letter } compare due volte nella stessa riga
check-unsupported-word = Parola non supportata { $letter }
check-several-motions = Più di un comando di movimento nella stessa riga

## Units and modes

# $code is G20 or G21; $units the units it sets, "mm" or "inch".
check-units-changed = { $code } cambia le unità in { $units ->
        [inch] pollici
       *[mm] mm
    } all'interno del programma
# $code is G20 or G21, $units the units it sets and $machine the machine's G-code units,
# each "mm" or "inch".
check-units-differ = { $code } ({ $units ->
        [inch] pollici
       *[mm] mm
    }) ma le unità G-code della macchina sono { $machine ->
        [inch] pollici
       *[mm] mm
    }
# $feed is the number after F.
check-feed-not-positive = L'avanzamento F{ $feed } deve essere positivo
check-relative = G91 (relativo) non è supportato
# $tool is the number after T.
check-tool-not-whole = T{ $tool } deve essere un numero utensile intero
check-start-without-speed = Mandrino avviato dopo un cambio utensile senza velocità (S)
check-dwell-without-p = Sosta G4 senza P
check-blend-and-dwell = G64 e G4 nella stessa riga: P è ambiguo
# G64 P lets the controller round corners. $p is the number after P, $most a length.
check-blend-too-far = G64 P{ $p } permette al controllo di uscire dal percorso di più di { $most }
# $most is the largest number P may have.
check-blend-without-p = G64 senza P permette al controllo di arrotondare gli spigoli senza limite (usa P al massimo { $most })
check-no-motion-mode = Parole d'asse senza modo di movimento

## Tool changes

check-change-without-tool = Cambio utensile (M6) senza numero utensile (T)
# $tool is the tool number after T.
check-change-unknown-tool = Cambio utensile (M6) a T{ $tool }, che non è un utensile del progetto
check-change-spindle-on = Cambio utensile (M6) con il mandrino acceso (fermalo prima con M5)
check-change-below-safe = Cambio utensile (M6) sotto la quota di sicurezza (prima risali)

## Moves

check-motion-before-units = Movimento prima di G21 (mm) o G20 (pollici)
check-motion-before-absolute = Movimento prima di G90 (assoluto)
check-rapid-below-safe = Rapido in XY con Z sotto la quota di sicurezza (prima risali)
# $z is a height with its unit, written after Z: "Z-3.000 mm".
check-rapid-into-material = Rapido a Z{ $z }, sotto la superficie del grezzo (rapido nel materiale)
check-cut-without-feed = Movimento di taglio senza avanzamento impostato
check-cut-spindle-off = Movimento di taglio con il mandrino spento
check-cut-unknown-position = Movimento di taglio da una posizione sconosciuta
# $z is a height with its unit, written after Z; $bottom the stock bottom's height and
# $allowed how far below it moves may go.
check-below-stock-too-far = Z{ $z } è sotto il fondo del grezzo ({ $bottom }) di più del margine consentito di { $allowed }
check-arc-without-centre = Arco senza offset del centro I/J
check-arc-unknown-position = Arco da una posizione sconosciuta
check-arc-full-circle = Arco che finisce dove inizia (un cerchio completo in un solo movimento)
# $start and $end are the arc's radius at its two ends.
check-arc-radius-mismatch = Raggio dell'arco non coerente: inizio { $start }, fine { $end }

## The whole program

# $deepest is the lowest height, written after Z; $bottom the stock bottom's height and
# $allowed how far below it moves may go.
check-below-stock = { $count ->
    [one] { $count } movimento scende
   *[other] { $count } movimenti scendono
} sotto il fondo del grezzo (punto più basso Z{ $deepest }, fondo { $bottom }), entro il margine consentito di { $allowed }
check-outside-stock = { $count ->
    [one] { $count } movimento di taglio finisce
   *[other] { $count } movimenti di taglio finiscono
} fuori dal grezzo (allargato del raggio utensile)
check-no-travel = Corsa della macchina non impostata; i limiti di corsa non sono stati controllati
# $axis is X, Y or Z; $span how far the program moves along it, $travel how far the
# machine can.
check-travel-exceeded = L'estensione in { $axis } { $span } supera la corsa della macchina { $travel }
check-no-end = Manca la fine del programma (M30)
