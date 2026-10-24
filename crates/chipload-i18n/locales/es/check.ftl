# Findings of the independent G-code checker (crates/chipload-check): what it found wrong
# or worth a warning in a G-code program. Words of G-code (G0, G21, M6, F, P, T, X, Z…)
# are never translated. A length argument brings its own unit ("-3.000 mm"); a number
# written after a G-code letter without a unit is the program's own number.

## Reading a line

check-unbalanced-comment = Comentario sin cerrar o anidado
# $text is the part of the line that could not be read, in quotes.
check-cannot-parse = No se puede interpretar { $text }
check-after-end = Código después del fin de programa (M30)
check-absolute-arc-centres = G90.1 (centros de arco absolutos) no es compatible
# $word is a G or M code, such as G43 or M7.5.
check-not-supported = { $word } no es compatible
# $letter is a letter of G-code, such as X or F.
check-word-twice = { $letter } aparece dos veces en una línea
check-unsupported-word = Palabra no compatible { $letter }
check-several-motions = Más de un comando de movimiento en una línea

## Units and modes

# $code is G20 or G21; $units the units it sets, "mm" or "inch".
check-units-changed = { $code } cambia las unidades a { $units ->
        [inch] pulgadas
       *[mm] mm
    } dentro del programa
# $code is G20 or G21, $units the units it sets and $machine the machine's G-code units,
# each "mm" or "inch".
check-units-differ = { $code } ({ $units ->
        [inch] pulgadas
       *[mm] mm
    }), pero las unidades del código G de la máquina son { $machine ->
        [inch] pulgadas
       *[mm] mm
    }
# $feed is the number after F.
check-feed-not-positive = El avance F{ $feed } debe ser positivo
check-relative = G91 (relativo) no es compatible
# $tool is the number after T.
check-tool-not-whole = T{ $tool } debe ser un número de herramienta entero
check-start-without-speed = Husillo arrancado tras un cambio de herramienta sin velocidad (S)
check-dwell-without-p = Pausa G4 sin P
check-blend-and-dwell = G64 y G4 en una línea: P es ambiguo
# G64 P lets the controller round corners. $p is the number after P, $most a length.
check-blend-too-far = G64 P{ $p } permite al controlador apartarse de la trayectoria más de { $most }
# $most is the largest number P may have.
check-blend-without-p = G64 sin P permite al controlador redondear esquinas sin límite (usa P como máximo { $most })
check-no-motion-mode = Palabras de eje sin modo de movimiento

## Tool changes

check-change-without-tool = Cambio de herramienta (M6) sin número de herramienta (T)
# $tool is the tool number after T.
check-change-unknown-tool = Cambio de herramienta (M6) a T{ $tool }, que no es una herramienta del proyecto
check-change-spindle-on = Cambio de herramienta (M6) con el husillo en marcha (detenlo antes con M5)
check-change-below-safe = Cambio de herramienta (M6) por debajo de la altura de seguridad (retrae antes)

## Moves

check-motion-before-units = Movimiento antes de G21 (mm) o G20 (pulgadas)
check-motion-before-absolute = Movimiento antes de G90 (absoluto)
check-rapid-below-safe = Movimiento rápido en XY con Z por debajo de la altura de seguridad (retrae antes)
# $z is a height with its unit, written after Z: "Z-3.000 mm".
check-rapid-into-material = Movimiento rápido a Z{ $z }, por debajo de la cara superior del material (rápido dentro del material)
check-cut-without-feed = Movimiento de corte sin avance definido
check-cut-spindle-off = Movimiento de corte con el husillo parado
check-cut-unknown-position = Movimiento de corte desde una posición desconocida
# $z is a height with its unit, written after Z; $bottom the stock bottom's height and
# $allowed how far below it moves may go.
check-below-stock-too-far = Z{ $z } está por debajo de la cara inferior del material ({ $bottom }) más de lo permitido ({ $allowed })
check-arc-without-centre = Arco sin desplazamientos de centro I/J
check-arc-unknown-position = Arco desde una posición desconocida
check-arc-full-circle = Arco que termina donde empieza (un círculo completo en un solo movimiento)
# $start and $end are the arc's radius at its two ends.
check-arc-radius-mismatch = Los radios del arco no coinciden: inicio { $start }, fin { $end }

## The whole program

# $deepest is the lowest height, written after Z; $bottom the stock bottom's height and
# $allowed how far below it moves may go.
check-below-stock = { $count ->
    [one] { $count } movimiento baja
   *[other] { $count } movimientos bajan
} de la cara inferior del material (lo más hondo Z{ $deepest }, cara inferior { $bottom }), dentro de lo permitido ({ $allowed })
check-outside-stock = { $count ->
    [one] { $count } movimiento de corte termina
   *[other] { $count } movimientos de corte terminan
} fuera del material (ampliado por el radio de la herramienta)
check-no-travel = Recorrido de la máquina sin definir; no se comprobaron los límites de recorrido
# $axis is X, Y or Z; $span how far the program moves along it, $travel how far the
# machine can.
check-travel-exceeded = La extensión en { $axis } ({ $span }) supera el recorrido de la máquina ({ $travel })
check-no-end = Falta el fin de programa (M30)
