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

toolpath-error-cancelled = Generation cancelled
# An error after where it happened: $context is an operation's label, or one of the
# context texts below.
toolpath-error-context = { $context }: { $error }
toolpath-error-label-contour = { $label }, contour #{ $contour }
# $part is a part's name, $copy the copy's number.
toolpath-error-part-copy = part { $part }, copy { $copy }
toolpath-error-copy = copy { $copy }
# $at is an operation's number in the cut list ("3", "3.1" for a rest pass).
toolpath-error-no-operation = No operation { $at } in the job

## Errors: selecting shapes

# $shapes are shape numbers ("S4, S7").
toolpath-error-shape-missing = shape { $shapes } is not part of any contour (deleted, or a duplicate or zero-length segment the import removed)
# $layers are layer names, separated by commas.
toolpath-error-partly-selected = contour #{ $contour } is only partly selected (layers { $layers }); fix the drawing or the selector
# $reason is why the drawing's contour can't be machined, as the DXF import found it (in English).
toolpath-error-contour-invalid = contour #{ $contour } is invalid: { $reason }
# $selector is what the operation selects, built from the toolpath-error-selector texts.
toolpath-error-selects-nothing = selector { $selector } matches nothing
# $contours are contour numbers ("#3, #4").
toolpath-error-selects-open = selector { $selector } matches only open contours ({ $contours }); this operation needs closed ones
toolpath-error-selector-layer = layer '{ $layer }'
toolpath-error-selector-shapes = shapes { $shapes }
toolpath-error-selector-and = { $first } and { $second }
# $id is the number of a DXF, a picture or a text in the project.
toolpath-error-selector-dxf = DXF { $id }
toolpath-error-selector-image = image { $id }
toolpath-error-selector-text = text { $id }
# $shapes are the shapes left out ("S3, S5").
toolpath-error-selector-but = { $source } but { $shapes }
toolpath-error-selector-of = { $what } of { $source }
# $a and $b are contour numbers.
toolpath-error-same-shape = contours #{ $a } and #{ $b } are the same shape; select only one of them

## Errors: operations and tools

toolpath-error-pass-depths = depths must satisfy 0 <= start ({ $start }) < end ({ $end }) and step_down ({ $step_down }) must be positive
# $tool is a tool's number.
toolpath-error-rest-tool = rest after tool { $tool }, which the project doesn't have
# $diameter is the hole's, $tool the tool's diameter.
toolpath-error-blind-bore = hole { $diameter } is blind and more than twice the { $tool } tool; use a pocket
# $tool is the tool's number, $type its kind as written in the project file (endmill, drill, vbit).
toolpath-error-no-hole-fits = no selected circle can be cut with tool { $tool } ({ $diameter } { $type })
toolpath-error-no-depth = no depth to cut to
toolpath-error-no-angle = tool { $tool } has no angle
# $got is an angle in degrees, without its unit.
toolpath-error-vbit-angle = the V-bit angle must be more than 0 and less than 180 degrees, got { $got }
# $got is a length, without its unit.
toolpath-error-stepover = stepover must be positive, got { $got }

## Errors: profiles, ramps and tabs

toolpath-error-ring-no-length = a profile ring has no length
toolpath-error-lap-no-course = a wall lap has no course
# $angle is in degrees, without its unit; $length is how far the ring goes around.
toolpath-error-ring-ramp = The ring is too short to ramp down { $drop } at { $angle }° ({ $length } around); use a steeper ramp angle or a straight plunge with a centre-cutting tool
# $reach is how much of the path the ramp may use.
toolpath-error-ramp-reach = The path is too short to ramp down { $drop } at { $angle }° (only { $reach } to ramp along); use a steeper ramp angle or a straight plunge with a centre-cutting tool
toolpath-error-path-ramp = The path is too short to ramp down { $drop } at { $angle }° ({ $length } long); use a steeper ramp angle or a straight plunge with a centre-cutting tool
# $angle is in degrees, without its unit.
toolpath-error-helix-turns = a helix of radius { $radius } at { $angle }° would need { $turns } turns to go down { $drop }; use a larger radius or a steeper angle
# $offset is how far inside the line the tool's centre would run.
toolpath-error-tool-does-not-fit = Tool does not fit inside the contour (its centre would run { $offset } inside the line)
toolpath-error-offset-verification = Offset failed verification (path not at tool radius from the contour); the contour may be degenerate
# $farthest is how far the finishing lap runs from the roughing path.
toolpath-error-roughing-reach = the roughing can't reach part of the wall (the finishing lap runs { $farthest } from the roughing path, a tool radius or more): use less finishing stock or a smaller tool
# $fits is how many tabs would fit.
toolpath-error-tabs-fit = { $count } tabs of { $width } don't fit on a { $length } path ({ $fits ->
    [0] not even one fits
   *[other] at most { $fits } fit
})
# $from is a distance along the ring without its unit, $to one with it.
toolpath-error-tab-carry = could not carry a tab ({ $from } to { $to } along the ring) onto the roughing ring
# $what is one of the three texts after it.
toolpath-error-tab-verification = Tab toolpath failed verification: { $what }
# $z is a height, $at a distance along the ring.
toolpath-error-tab-below = Z{ $z } at { $at } along the ring is below the tab
toolpath-error-tab-back = the tool goes back along the ring at { $at }
toolpath-error-tab-end-below = Z{ $z } at the tab end { $end } along the ring is below the tab top Z{ $top }
# $label is the operation's label; $at the corner's point.
toolpath-error-corner-leg = { $label }: the leg at corner { $at } did not go into the ring as out and back
toolpath-error-corner-cuts = { $label }: the { $style ->
    [dogbone] dogbone
   *[tbone] T-bone
} at corner { $at } would cut into contour #{ $contour }; lower the corners' max angle or change the drawing
# $x and $y are a point's coordinates, without their unit.
toolpath-error-arc-off-circle = arc output: ({ $x }, { $y }) is not on the circle it was tagged with

## Errors: pockets

# $contours are contour numbers ("#3, #4"): the pocket's boundary and its islands.
toolpath-error-islands-enclose = contours { $contours }: overlapping islands enclose an area between them, which pockets don't support yet
toolpath-error-pocket-no-fit = contours { $contours }: tool (diameter { $diameter }) does not fit in the pocket
toolpath-error-pocket-no-fit-clearance = contours { $contours }: tool (diameter { $diameter }, clearance { $clearance }) does not fit in the pocket
toolpath-error-not-nested = contours { $contours }: pocket offsets are not nested
toolpath-error-too-many-rings = contours { $contours }: more than { $most } pocket rings
toolpath-error-pocket-verification = contours { $contours }: pocket toolpath failed verification (too close to a wall or island); the drawing may be degenerate
toolpath-error-nothing-to-clear = Nothing to clear: the selected islands cover the selected boundaries (contours { $contours })
toolpath-error-island-outside = pocket offset produced an island outside every area
toolpath-error-shrink-hole = pocket offset made a hole while shrinking a single loop
toolpath-error-open-loop = pocket offset returned an open loop where a closed one was expected
toolpath-error-rest-outside = the rest machining area lies outside the pocket
toolpath-error-left-hole-outside = material left: a hole outside every piece
toolpath-error-left-holes-enclose = material left: holes enclose an area
toolpath-error-left-wrong-area = material left: a boolean gave the wrong area
toolpath-error-left-invalid = material left: invalid geometry from the offset library

## Errors: V-carving, centre lines and letters

# What a centre line is for, inside the texts below.
toolpath-error-medial-name = { $name ->
    [vcarve] V-carve
   *[centre-line] centre line
}
# $name is toolpath-error-medial-name; $error the Voronoi library's own words.
toolpath-error-medial-failed = contours { $contours }: the { $name } centre line could not be built ({ $error }); does the outline touch or cross itself?
toolpath-error-medial-cell = { $name } centre line: a cell without its chord
# $reach is how far from the origin the centre line works.
toolpath-error-medial-far = a { $name } outline at { $at } is too far from the drawing origin (the centre line works within ±{ $reach })
toolpath-error-medial-step = The { $name } centre line
toolpath-error-vcarve-verification = contours { $contours }: V-carve verification failed: the cut would cross the outline near { $at }
toolpath-error-join-gave = joining two overlapping outlines gave { $outlines } outlines
toolpath-error-join-changed = joining overlapping outlines changed what is filled near { $at }
toolpath-error-joined-cross = joined outlines cross each other
toolpath-error-outline-unreadable = an outline the geometry library can't read

## Errors: copies and parts

# $width is the cut's width; $got the pitch, without its unit.
toolpath-error-copies-dx = Copies overlap: with columns the pitch dx must be at least { $width } (the cut is { $width } wide), got { $got }
toolpath-error-copies-dy = Copies overlap: with rows the pitch dy must be at least { $height } (the cut is { $height } high), got { $got }
# $other and $copy are copies' numbers; $dx and $dy the pitch, without its unit.
toolpath-error-copies-overlap = Copies overlap: copies { $other } and { $copy } cut into each other; move them apart (pitch dx { $dx }, dy { $dy })
# $part and $other_part are parts' names; $x0 and $y0 are without their unit.
toolpath-error-parts-overlap = Part "{ $part }" copy { $copy } overlaps part "{ $other_part }" copy { $other_copy } (X{ $x0 } to { $x1 }, Y{ $y0 } to { $y1 }): move the copies apart or change the grid

## Errors: inside the geometry library

# $what is toolpath-error-step or toolpath-error-medial-step.
toolpath-error-library-failed = { $what } failed inside the geometry library
toolpath-error-step = { $step ->
    [pocket-offset] Pocket offset
    [joining-letters] Joining letters
    [checking-joined-letters] Checking joined letters
    [material-left-union] Material left union
    [material-left-offset] Material left offset
    [material-left-boolean] Material left boolean
    [comparing-shapes] Comparing shapes
    [merging-islands] Merging islands
    [subtracting-an-island] Subtracting an island
   *[offset] Offset
}
