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
