# Messages of the planner (crates/chipload-toolpath): rules a project must meet before it
# can be cut. Most are shown after the operation's label, as "Operation 3 (Outline): …";
# see job-operation-problem. Words in the texts that name fields or values of the project
# file (safe_z, z_zero, max_feed, entry: ramp, center_cutting, …) are written in the file
# that way and are never translated. A length or feed argument brings its own unit
# ("12.5 mm", "1200 mm/min").

## The project as a whole

toolpath-no-operations = El trabajo no tiene operaciones
toolpath-all-disabled = Todas las operaciones están desactivadas: no hay nada que cortar
# $tool is a tool's number.
toolpath-tool-undefined = La herramienta { $tool } no está definida en el trabajo
# Under the tool's spindle speed field.
toolpath-no-rpm-short = sin velocidad del husillo
toolpath-no-rpm = La herramienta { $tool } no tiene velocidad del husillo (rpm)
# Under the machine's safe height; $top is the height of the stock's top.
toolpath-safe-z-short = debe estar por encima de la cara superior del material (Z { $top })
# $zero is "top" or "bottom", as written in the project file.
toolpath-safe-z = safe_z { $safe } debe estar por encima de la cara superior del material (Z { $top }); z_zero es '{ $zero }'

## Depths and sizes

# $below is how far below the stock's bottom it goes, $allowed the setting "max depth
# below stock".
toolpath-too-deep = la prof. final { $end } queda { $below } por debajo del material ({ $thickness } de espesor), más de lo permitido, { $allowed } (prof. máx. bajo el material)
toolpath-clearance-radius = la holgura { $clearance } debe ser menor que el radio de la herramienta { $radius }
toolpath-finish-radius = el sobrematerial de acabado { $stock } debe ser menor que el radio de la herramienta { $radius }
toolpath-tab-height = la altura de los puentes { $height } debe ser menor que la profundidad de corte { $cut } (prof. final menos prof. inicial)
# $field is "feed" or "plunge_feed", as written in the project file.
toolpath-max-feed = { $field } { $feed } supera el max_feed de la máquina, { $max }
toolpath-helix-for-path = la entrada en hélice es para cajeras; un seguir trayectoria puede entrar en rampa (entry: ramp)
toolpath-helix-radius = el radio de hélice { $radius } debe ser menor que el radio de la herramienta { $tool_radius }, o deja un núcleo en pie en el centro
# $most is a number of tool diameters, $longest that as a length.
toolpath-lead-radius = el radio de aproximación { $radius } es más de { $most } × el diámetro de la herramienta ({ $longest })

## Tools

# $tool is a tool's number, $type a kind of tool (job-tool-type).
toolpath-surface-endmill = la herramienta { $tool } es una { $type }; un planeado necesita una fresa
toolpath-vcarve-vbit = el tallado en V necesita una fresa en V, la herramienta { $tool } es una { $type }
toolpath-vbit-no-angle = la herramienta { $tool } es una fresa en V sin ángulo; fija su ángulo
toolpath-pocket-endmill = las cajeras necesitan una fresa, la herramienta { $tool } es una { $type }
toolpath-path-drill = la herramienta { $tool } es una broca, que no puede seguir una trayectoria; usa una fresa o una fresa en V
toolpath-holes-vbit = la herramienta { $tool } es una fresa en V, que no puede cortar taladros; usa una broca o una fresa
# $method is the holes' method as written in the project file, "drill" or "bore"; $type
# a kind of tool with its article (job-tool-type-article).
toolpath-holes-method = el método es { $method ->
    [drill] drill, que necesita una broca
   *[bore] bore, que necesita una fresa
}; la herramienta { $tool } es { $type }
# Angles in degrees.
toolpath-flank-not-centre-cutting = la herramienta { $tool } no corta al centro y los flancos de los puentes ({ $flank }°) tienen más pendiente que el ángulo de rampa ({ $ramp }°); bajar por un flanco es una rampa, así que no les des más de { $ramp }°
toolpath-plunge-not-centre-cutting = la herramienta { $tool } no corta al centro y esta operación penetra en vertical; entra en rampa (entry: ramp) o usa una herramienta que corte al centro

## Rest machining

# $tool is the number of the larger tool the pocket rests after.
toolpath-rest-no-tool = restos tras la herramienta { $tool }, que el proyecto no tiene
toolpath-rest-not-endmill = restos tras la herramienta { $tool }, una { $type }; la herramienta de una cajera es una fresa
# Two tool diameters.
toolpath-rest-not-larger = los restos tras la herramienta { $tool } ({ $large }) necesitan una herramienta más grande que la de esta cajera ({ $small })
toolpath-rest-dogbones = los dogbones y el mecanizado de restos aún no se pueden combinar; pon los dogbones en la cajera de la herramienta grande
toolpath-rest-passes-dogbones = los dogbones y las pasadas de restos aún no se pueden combinar; las esquinas dogbone no necesitan restos
toolpath-rest-pass-no-tool = la herramienta { $tool } no está definida en el trabajo
toolpath-rest-pass-endmill = la herramienta { $tool } es una { $type }; una pasada de restos corta con una fresa
toolpath-rest-pass-larger = la herramienta { $tool } ({ $diameter }) debe ser más pequeña que la herramienta { $before } ({ $before_diameter }) anterior
toolpath-rest-pass-no-rpm = la herramienta { $tool } no tiene velocidad del husillo (rpm)

## Planning notes: warnings that don't stop the G-code

# A note about an operation after its label ("Operation 3 (Outline): …"); $note is one
# of the notes below.
toolpath-labelled = { $label }: { $note }
# A note about one contour of an operation: "Operation 3 (Outline), contour #4: …".
toolpath-labelled-contour = { $label }, contorno #{ $contour }: { $note }
# Lists built one item at a time: "1, 2, 3" and "1, 2 and 3". $list is the items so far.
toolpath-comma = { $list }, { $item }
toolpath-and = { $list } y { $last }
# An operation's number in a list of operations.
toolpath-number = { $number }
# An operation's number and its kind in a list: "3 (pocket)".
toolpath-operation-kind = { $number } ({ $kind ->
    [profile] contorneado
    [pocket] cajera
    [vcarve] tallado en V
    [path] seguir trayectoria
    [holes] taladros
   *[surface] planeado
})
# $layers is the contour's layer names, $from and $to its two ends ("(10.000, 5.000) mm").
toolpath-skipped-open = se omitió el contorno abierto #{ $contour } en { $layers }, de { $from } a { $to }
toolpath-into-spoilboard = corta { $below } por debajo del material (en el tablero de sacrificio)
# $operations lists operation numbers with their kind: "1 (pocket), 3 (profile) and 4 (holes)".
toolpath-plunge-unknown = La herramienta { $tool } no está marcada como center_cutting y { $count ->
    [one] la operación { $operations } penetra
   *[other] las operaciones { $operations } penetran
} en vertical: asegúrate de que corta al centro, o entra en rampa (entry: ramp)
# $operations is a list of operation numbers ("1, 2 and 4"); $label the surface's label.
toolpath-surface-after = { $label } planea el material después de que { $count ->
    [one] la operación { $operations } lo haya cortado
   *[other] las operaciones { $operations } lo hayan cortado
}: el planeado suele ir primero
# $tool is the number of the larger tool the pocket rests after.
toolpath-rest-no-earlier = restos tras la herramienta { $tool }, pero ninguna cajera anterior corta estas formas con ella a esa profundidad; el resto de la cajera quedará sin cortar
toolpath-rest-pass-plunge = penetra en vertical con la herramienta { $tool }, que no está marcada como de corte al centro; usa una hélice o una rampa si no lo es
# $groups is how many separate runs of operations use the tool; $operations their numbers
# ("1, 2, 5").
toolpath-tool-returns = La herramienta { $tool } se usa en { $groups } grupos (operaciones { $operations }); ponerlas una tras otra ahorra un cambio de herramienta
toolpath-vbit-no-tip = la herramienta { $tool } no tiene diámetro de punta; el tallado en V la toma como puntiaguda
# Angles in degrees.
toolpath-flank-unknown-centre-cutting = los flancos de los puentes ({ $flank }°) tienen más pendiente que el ángulo de rampa ({ $ramp }°); la herramienta { $tool } no está marcada como center_cutting, asegúrate de que corta al centro

## Planning notes: profiles

# $loops is how many separate loops the contour is cut as.
toolpath-narrow-loops = la herramienta no cabe por una parte estrecha; se corta como { $loops } anillos separados y queda material entre ellos
# $point is a start point the user set ("(10.000, 5.000) mm").
toolpath-start-far-contour = el punto de inicio { $point } está a { $distance } del contorno #{ $contour }; se usa el punto más cercano
toolpath-start-twice-contour = el punto de inicio { $point } también está en el contorno #{ $contour }, que ya tiene uno; se ignora
toolpath-tabs-inside = los puentes en un contorneado interior sujetarán el recorte suelto del centro
# $contours is a list of contour numbers ("#3, #4").
toolpath-tabs-on-curves = algunos puentes en los contornos { $contours } quedan en una esquina o curva (sin tramo recto cerca)
# $pairs lists the tabs that meet, by number ("1 and 2, 3 and 4"); $flank is in degrees.
toolpath-tabs-joined = los puentes { $pairs } se juntan por debajo de su cara superior (flancos a { $flank }°): sujetan como un solo puente más largo
toolpath-finish-lap-cuts = la vuelta de acabado corta hasta { $most } en esquinas o partes estrechas (sobrematerial de acabado { $stock })
# $style is the operation's corner style.
toolpath-corner-unreached = esquina en { $point }: la herramienta no llega a ella; { $style ->
    [dogbone] sin dogbone
   *[tbone] sin T-bone
}

## Planning notes: pockets
## $contours is a list of contour numbers ("#3, #4"): the pocket's boundary and its islands.

toolpath-corners-not-rounded = contornos { $contours }: esquinas de los anillos sin redondear; redondearlas dejaría material
toolpath-pocket-narrow = contornos { $contours }: la herramienta no cabe por una parte estrecha; se vacía como { $areas } zonas y queda material entre ellas
# $width is a number without its unit; $height brings the unit; $near is a point.
toolpath-uncut = contornos { $contours }: material sin cortar en { $count ->
    [one] { $count } sitio
   *[other] { $count } sitios
}, el mayor de unos { $width } x { $height } cerca de { $near }; reduce el paso lateral para quitarlo
toolpath-lead-reduced = contornos { $contours }: radio de aproximación reducido a { $used } (no hay espacio para { $radius })
toolpath-no-lead = contornos { $contours }: no hay espacio para una aproximación (ni siquiera { $least }); la pared se corta sin ella
toolpath-helix-reduced = contornos { $contours }: radio de hélice reducido a { $used } (no hay espacio para { $radius })
toolpath-no-helix-room = contornos { $contours }: no hay espacio para una hélice (ni siquiera { $least }); entra en rampa en su lugar
toolpath-no-helix-narrow = contornos { $contours }: no hay espacio para una hélice (la cajera es demasiado estrecha); entra en rampa en su lugar
toolpath-no-helix-way = contornos { $contours }: no hay espacio para una hélice (su camino hasta la primera trayectoria pasa por una pared o una isla); entra en rampa en su lugar
toolpath-start-far-wall = contornos { $contours }: el punto de inicio { $point } está a { $distance } de la pared; se usa el punto más cercano
toolpath-start-twice-wall = contornos { $contours }: el punto de inicio { $point } está en una pared que ya tiene uno; se ignora
toolpath-rest-nothing-left = la herramienta mayor llega a toda la cajera; no quedan restos que mecanizar
# $width is a number without its unit; $height brings the unit.
toolpath-rest-too-small = mecanizado de restos: el material en { $count ->
    [one] { $count } sitio
   *[other] { $count } sitios
} es demasiado pequeño también para esta herramienta (el mayor de unos { $width } × { $height })

## Planning notes: V-carving, follow paths and holes

# $across is the V-bit's diameter, $cap the deepest it carves, $wide the widest stroke it
# carves at that depth, $need the depth the widest stroke needs.
toolpath-vcarve-too-wide-bit = contornos { $contours }: demasiado anchos para la fresa en V ({ $across } de diámetro), que talla como máximo { $cap } de profundidad ({ $wide } de ancho en la superficie); no se alcanzan los bordes de las partes más anchas (necesitan { $need }). Permite más profundidad, usa un ángulo mayor o vacía el fondo
toolpath-vcarve-too-wide = contornos { $contours }: demasiado anchos para un tallado de como máximo { $cap } de profundidad ({ $wide } de ancho en la superficie); no se alcanzan los bordes de las partes más anchas (necesitan { $need }). Permite más profundidad, usa un ángulo mayor o vacía el fondo
# $narrow is a length of centre line, $tip the V-bit's tip diameter.
toolpath-vcarve-narrow = contornos { $contours }: { $narrow } de línea central son más estrechos que la punta ({ $tip }) y no se tallan
toolpath-path-too-short = el contorno #{ $contour } es demasiado corto para seguirlo ({ $length }); se omite
toolpath-groove-no-angle = la herramienta { $tool } es una fresa en V sin ángulo: no se conoce el ancho de la ranura
toolpath-groove-past-cone = la ranura en V es más profunda que el cono de la fresa en V: { $width } de ancho arriba, con paredes rectas por encima del cono
toolpath-groove-width = la ranura en V tiene { $width } de ancho arriba
toolpath-holes-not-circles = { $count ->
    [one] se omitió { $count } forma que no es un círculo
   *[other] se omitieron { $count } formas que no son círculos
}
# $diameter is the holes' diameter, $tool the tool's, $within how far they may differ.
toolpath-holes-not-drill-size = { $count ->
    [one] taladro de { $diameter }
   *[other] { $count } taladros de { $diameter }
}: no coincide con la medida de la broca de { $tool } (margen de { $within }); no se taladra
toolpath-holes-too-small-to-bore = { $count ->
    [one] taladro de { $diameter }: demasiado pequeño para taladrarlo
   *[other] { $count } taladros de { $diameter }: demasiado pequeños para taladrarlos
} en hélice con la herramienta de { $tool }; sin cortar
toolpath-holes-core = { $count ->
    [one] { $count } taladro deja
   *[other] { $count } taladros dejan
} un núcleo suelto (taladro de más del doble de la herramienta); sujétalo o usa una cajera

## Errors: why no toolpath could be made

toolpath-error-cancelled = Cálculo cancelado
# An error after where it happened: $context is an operation's label, or one of the
# context texts below.
toolpath-error-context = { $context }: { $error }
toolpath-error-label-contour = { $label }, contorno #{ $contour }
# $part is a part's name, $copy the copy's number.
toolpath-error-part-copy = pieza { $part }, copia { $copy }
toolpath-error-copy = copia { $copy }
# $at is an operation's number in the cut list ("3", "3.1" for a rest pass).
toolpath-error-no-operation = No hay ninguna operación { $at } en el trabajo

## Errors: selecting shapes

# $shapes are shape numbers ("S4, S7").
toolpath-error-shape-missing = la forma { $shapes } no forma parte de ningún contorno (se borró, o era un segmento duplicado o de longitud cero que la importación eliminó)
# $layers are layer names, separated by commas.
toolpath-error-partly-selected = el contorno #{ $contour } solo está seleccionado en parte (capas { $layers }); corrige el dibujo o el selector
# $reason is why the drawing's contour can't be machined, as the DXF import found it (in English).
toolpath-error-contour-invalid = el contorno #{ $contour } no es válido: { $reason }
# $selector is what the operation selects, built from the toolpath-error-selector texts.
toolpath-error-selects-nothing = el selector { $selector } no coincide con nada
# $contours are contour numbers ("#3, #4").
toolpath-error-selects-open = el selector { $selector } solo coincide con contornos abiertos ({ $contours }); esta operación necesita contornos cerrados
toolpath-error-selector-layer = capa '{ $layer }'
toolpath-error-selector-shapes = formas { $shapes }
toolpath-error-selector-and = { $first } y { $second }
# $id is the number of a DXF, a picture or a text in the project.
toolpath-error-selector-dxf = DXF { $id }
toolpath-error-selector-image = imagen { $id }
toolpath-error-selector-text = texto { $id }
# $shapes are the shapes left out ("S3, S5").
toolpath-error-selector-but = { $source } excepto { $shapes }
toolpath-error-selector-of = { $what } de { $source }
# $a and $b are contour numbers.
toolpath-error-same-shape = los contornos #{ $a } y #{ $b } son la misma forma; selecciona solo uno de ellos

## Errors: operations and tools

toolpath-error-pass-depths = las profundidades deben cumplir 0 <= inicio ({ $start }) < final ({ $end }) y step_down ({ $step_down }) debe ser positivo
# $tool is a tool's number.
toolpath-error-rest-tool = restos tras la herramienta { $tool }, que el proyecto no tiene
# $diameter is the hole's, $tool the tool's diameter.
toolpath-error-blind-bore = el taladro de { $diameter } es ciego y mide más del doble de la herramienta de { $tool }; usa una cajera
# $tool is the tool's number, $type its kind as written in the project file (endmill, drill, vbit).
toolpath-error-no-hole-fits = ningún círculo seleccionado se puede cortar con la herramienta { $tool } ({ $diameter } { $type })
toolpath-error-no-depth = no hay profundidad hasta la que cortar
toolpath-error-no-angle = la herramienta { $tool } no tiene ángulo
# $got is an angle in degrees, without its unit.
toolpath-error-vbit-angle = el ángulo de la fresa en V debe ser más de 0 y menos de 180 grados, es { $got }
# $got is a length, without its unit.
toolpath-error-stepover = el paso lateral debe ser positivo, es { $got }

## Errors: profiles, ramps and tabs

toolpath-error-ring-no-length = un anillo de contorneado no tiene longitud
toolpath-error-lap-no-course = una vuelta de pared no tiene recorrido
# $angle is in degrees, without its unit; $length is how far the ring goes around.
toolpath-error-ring-ramp = El anillo es demasiado corto para bajar { $drop } en rampa a { $angle }° ({ $length } de vuelta); usa un ángulo de rampa más inclinado o una penetración vertical con una herramienta que corte al centro
# $reach is how much of the path the ramp may use.
toolpath-error-ramp-reach = La trayectoria es demasiado corta para bajar { $drop } en rampa a { $angle }° (solo hay { $reach } para la rampa); usa un ángulo de rampa más inclinado o una penetración vertical con una herramienta que corte al centro
toolpath-error-path-ramp = La trayectoria es demasiado corta para bajar { $drop } en rampa a { $angle }° ({ $length } de largo); usa un ángulo de rampa más inclinado o una penetración vertical con una herramienta que corte al centro
# $angle is in degrees, without its unit.
toolpath-error-helix-turns = una hélice de radio { $radius } a { $angle }° necesitaría { $turns } vueltas para bajar { $drop }; usa un radio mayor o un ángulo más inclinado
# $offset is how far inside the line the tool's centre would run.
toolpath-error-tool-does-not-fit = La herramienta no cabe dentro del contorno (su centro iría { $offset } por dentro de la línea)
toolpath-error-offset-verification = El equidistante no pasó la verificación (la trayectoria no está a un radio de herramienta del contorno); puede que el contorno sea degenerado
# $farthest is how far the finishing lap runs from the roughing path.
toolpath-error-roughing-reach = el desbaste no llega a parte de la pared (la vuelta de acabado pasa a { $farthest } de la trayectoria de desbaste, un radio de herramienta o más): usa menos sobrematerial de acabado o una herramienta más pequeña
# $fits is how many tabs would fit.
toolpath-error-tabs-fit = { $count ->
    [one] { $count } puente de { $width } no cabe
   *[other] { $count } puentes de { $width } no caben
} en una trayectoria de { $length } ({ $fits ->
    [0] no cabe ni uno
   *[other] caben como máximo { $fits }
})
toolpath-tab-on-start = el puente en { $at } cubre el inicio del anillo o queda a menos de medio diámetro de herramienta de él, donde baja cada pasada; mueve el puente o el inicio del anillo
toolpath-tabs-too-close = los puentes en { $at } y { $other } están demasiado cerca: necesitan su anchura más dos diámetros de herramienta entre sus centros
toolpath-tab-no-ring = { $label }: el puente colocado en { $at } ya no está en ninguno de sus contornos (¿cambió el dibujo?); muévelo o quítalo
# $from is a distance along the ring without its unit, $to one with it.
toolpath-error-tab-carry = no se pudo trasladar un puente (de { $from } a { $to } a lo largo del anillo) al anillo de desbaste
# $what is one of the three texts after it.
toolpath-error-tab-verification = La trayectoria de los puentes no pasó la verificación: { $what }
# $z is a height, $at a distance along the ring.
toolpath-error-tab-below = Z{ $z } a { $at } a lo largo del anillo está por debajo del puente
toolpath-error-tab-back = la herramienta retrocede a lo largo del anillo en { $at }
toolpath-error-tab-end-below = Z{ $z } en el extremo del puente, a { $end } a lo largo del anillo, está por debajo de la cara superior del puente, Z{ $top }
# $label is the operation's label; $at the corner's point.
toolpath-error-corner-leg = { $label }: el tramo de la esquina { $at } no entró en el anillo como ida y vuelta
toolpath-error-corner-cuts = { $label }: el { $style ->
    [dogbone] dogbone
   *[tbone] T-bone
} de la esquina { $at } cortaría el contorno #{ $contour }; elige otra cosa para esta esquina, baja el ángulo máximo de las esquinas o cambia el dibujo
toolpath-error-corner-cuts-own = { $label }: el { $style ->
    [dogbone] dogbone
   *[tbone] T-bone
} de la esquina { $at } cortaría otra pared de su propio contorno #{ $contour }; elige otra cosa para esta esquina (un T-bone a lo largo de la otra pared, o redonda) o cambia el dibujo
toolpath-corner-choice-unmatched = { $label }: la esquina elegida en { $at } ya no es una esquina de sus formas (¿cambió el dibujo?); quita la elección o elige de nuevo
# $x and $y are a point's coordinates, without their unit.
toolpath-error-arc-off-circle = salida de arcos: ({ $x }, { $y }) no está en el círculo con el que se marcó

## Errors: pockets

# $contours are contour numbers ("#3, #4"): the pocket's boundary and its islands.
toolpath-error-islands-enclose = contornos { $contours }: unas islas superpuestas encierran un área entre ellas, algo que las cajeras aún no admiten
toolpath-error-pocket-no-fit = contornos { $contours }: la herramienta (diámetro { $diameter }) no cabe en la cajera
toolpath-error-pocket-no-fit-clearance = contornos { $contours }: la herramienta (diámetro { $diameter }, holgura { $clearance }) no cabe en la cajera
toolpath-error-not-nested = contornos { $contours }: los equidistantes de la cajera no están anidados
toolpath-error-too-many-rings = contornos { $contours }: más de { $most } anillos de cajera
toolpath-error-pocket-verification = contornos { $contours }: la trayectoria de la cajera no pasó la verificación (demasiado cerca de una pared o una isla); puede que el dibujo sea degenerado
toolpath-error-nothing-to-clear = Nada que vaciar: las islas seleccionadas cubren los contornos exteriores seleccionados (contornos { $contours })
toolpath-error-island-outside = el equidistante de la cajera produjo una isla fuera de todas las áreas
toolpath-error-shrink-hole = el equidistante de la cajera creó un hueco al encoger un único lazo
toolpath-error-open-loop = el equidistante de la cajera devolvió un lazo abierto donde se esperaba uno cerrado
toolpath-error-rest-outside = el área de mecanizado de restos queda fuera de la cajera
toolpath-error-left-hole-outside = material restante: un hueco fuera de todos los fragmentos
toolpath-error-left-holes-enclose = material restante: unos huecos encierran un área
toolpath-error-left-wrong-area = material restante: una operación booleana dio un área errónea
toolpath-error-left-invalid = material restante: geometría no válida de la biblioteca de equidistantes

## Errors: V-carving, centre lines and letters

# What a centre line is for, inside the texts below.
toolpath-error-medial-name = { $name ->
    [vcarve] del tallado en V
   *[centre-line] del trazo
}
# $name is toolpath-error-medial-name; $error the Voronoi library's own words.
toolpath-error-medial-failed = contornos { $contours }: no se pudo construir la línea central { $name } ({ $error }); ¿el contorno se toca o se cruza consigo mismo?
toolpath-error-medial-cell = línea central { $name }: una celda sin su cuerda
# $reach is how far from the origin the centre line works.
toolpath-error-medial-far = un contorno { $name } en { $at } está demasiado lejos del origen del dibujo (la línea central funciona dentro de ±{ $reach })
toolpath-error-medial-step = La línea central { $name }
toolpath-error-vcarve-verification = contornos { $contours }: falló la verificación del tallado en V: el corte cruzaría el contorno cerca de { $at }
toolpath-error-join-gave = unir dos contornos superpuestos dio { $outlines } contornos
toolpath-error-join-changed = unir contornos superpuestos cambió lo que queda relleno cerca de { $at }
toolpath-error-joined-cross = los contornos unidos se cruzan entre sí
toolpath-error-outline-unreadable = un contorno que la biblioteca de geometría no puede leer

## Errors: copies and parts

# $width is the cut's width; $got the pitch, without its unit.
toolpath-error-copies-dx = Las copias se solapan: con columnas, el paso dx debe ser al menos { $width } (el corte mide { $width } de ancho), es { $got }
toolpath-error-copies-dy = Las copias se solapan: con filas, el paso dy debe ser al menos { $height } (el corte mide { $height } de alto), es { $got }
# $other and $copy are copies' numbers; $dx and $dy the pitch, without its unit.
toolpath-error-copies-overlap = Las copias se solapan: las copias { $other } y { $copy } se cortan entre sí; sepáralas (paso dx { $dx }, dy { $dy })
# $part and $other_part are parts' names; $x0 and $y0 are without their unit.
toolpath-error-parts-overlap = La copia { $copy } de la pieza "{ $part }" se solapa con la copia { $other_copy } de la pieza "{ $other_part }" (X{ $x0 } a { $x1 }, Y{ $y0 } a { $y1 }): separa las copias o cambia la cuadrícula

## Errors: inside the geometry library

# $what is toolpath-error-step or toolpath-error-medial-step.
toolpath-error-library-failed = { $what } falló dentro de la biblioteca de geometría
toolpath-error-step = { $step ->
    [pocket-offset] Equidistante de cajera
    [joining-letters] Unión de letras
    [checking-joined-letters] Comprobación de letras unidas
    [material-left-union] Unión del material restante
    [material-left-offset] Equidistante del material restante
    [material-left-boolean] Booleana del material restante
    [comparing-shapes] Comparación de formas
    [merging-islands] Fusión de islas
    [subtracting-an-island] Resta de una isla
   *[offset] Equidistante
}
