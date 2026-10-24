# Messages of the planner (crates/chipload-toolpath): rules a project must meet before it
# can be cut. Most are shown after the operation's label, as "Operation 3 (Outline): …";
# see job-operation-problem. Words in the texts that name fields or values of the project
# file (safe_z, z_zero, max_feed, entry: ramp, center_cutting, …) are written in the file
# that way and are never translated. A length or feed argument brings its own unit
# ("12.5 mm", "1200 mm/min").

## The project as a whole

toolpath-no-operations = Il lavoro non ha lavorazioni
toolpath-all-disabled = Tutte le lavorazioni sono disattivate: niente da tagliare
# $tool is a tool's number.
toolpath-tool-undefined = L'utensile { $tool } non è definito nel lavoro
# Under the tool's spindle speed field.
toolpath-no-rpm-short = nessuna velocità mandrino
toolpath-no-rpm = L'utensile { $tool } non ha una velocità mandrino (rpm)
# Under the machine's safe height; $top is the height of the stock's top.
toolpath-safe-z-short = deve essere sopra la superficie del grezzo (Z { $top })
# $zero is "top" or "bottom", as written in the project file.
toolpath-safe-z = safe_z { $safe } deve essere sopra la superficie del grezzo (Z { $top }); z_zero è '{ $zero }'

## Depths and sizes

# $below is how far below the stock's bottom it goes, $allowed the setting "max depth
# below stock".
toolpath-too-deep = la profondità finale { $end } è { $below } sotto il grezzo (spessore { $thickness }), più dei { $allowed } consentiti (prof. max sotto grezzo)
toolpath-clearance-radius = la maggiorazione { $clearance } deve essere minore del raggio dell'utensile { $radius }
toolpath-finish-radius = il sovrametallo di finitura { $stock } deve essere minore del raggio dell'utensile { $radius }
toolpath-tab-height = l'altezza dei ponticelli { $height } deve essere minore della profondità di taglio { $cut } (profondità finale meno profondità iniziale)
# $field is "feed" or "plunge_feed", as written in the project file.
toolpath-max-feed = { $field } { $feed } supera max_feed della macchina { $max }
toolpath-helix-for-path = l'ingresso a elica è per le tasche; un segui percorso può entrare in rampa (entry: ramp)
toolpath-helix-radius = il raggio dell'elica { $radius } deve essere minore del raggio dell'utensile { $tool_radius }, altrimenti resta in piedi un nucleo al centro
# $most is a number of tool diameters, $longest that as a length.
toolpath-lead-radius = il raggio di raccordo { $radius } è più di { $most } × il diametro dell'utensile ({ $longest })

## Tools

# $tool is a tool's number, $type a kind of tool (job-tool-type).
toolpath-surface-endmill = l'utensile { $tool } è una { $type }; una spianatura richiede una fresa
toolpath-vcarve-vbit = l'incisione a V richiede una fresa a V, l'utensile { $tool } è una { $type }
toolpath-vbit-no-angle = l'utensile { $tool } è una fresa a V senza angolo; impostane l'angolo
toolpath-pocket-endmill = le tasche richiedono una fresa, l'utensile { $tool } è una { $type }
toolpath-path-drill = l'utensile { $tool } è una punta, che non può seguire un percorso; usa una fresa o una fresa a V
toolpath-holes-vbit = l'utensile { $tool } è una fresa a V, che non può tagliare fori; usa una punta o una fresa
# $method is the holes' method as written in the project file, "drill" or "bore"; $type
# a kind of tool with its article (job-tool-type-article).
toolpath-holes-method = il metodo è { $method ->
    [drill] drill, che richiede una punta
   *[bore] bore, che richiede una fresa
}; l'utensile { $tool } è { $type }
# Angles in degrees.
toolpath-flank-not-centre-cutting = l'utensile { $tool } non taglia al centro e i fianchi dei ponticelli ({ $flank }°) sono più ripidi dell'angolo di rampa ({ $ramp }°); scendere lungo un fianco è una rampa, quindi rendili non più ripidi di { $ramp }°
toolpath-plunge-not-centre-cutting = l'utensile { $tool } non taglia al centro e questa lavorazione affonda in verticale; entra invece in rampa (entry: ramp) o usa un utensile che taglia al centro

## Rest machining

# $tool is the number of the larger tool the pocket rests after.
toolpath-rest-no-tool = ripresa dopo l'utensile { $tool }, che il progetto non ha
toolpath-rest-not-endmill = ripresa dopo l'utensile { $tool }, una { $type }; l'utensile di una tasca è una fresa
# Two tool diameters.
toolpath-rest-not-larger = la ripresa dopo l'utensile { $tool } ({ $large }) richiede un utensile più grande di quello di questa tasca ({ $small })
toolpath-rest-dogbones = dogbone e ripresa dei residui non si possono ancora combinare; metti i dogbone nella tasca dell'utensile grande
toolpath-rest-passes-dogbones = dogbone e passate di ripresa non si possono ancora combinare; gli angoli dogbone non richiedono ripresa
toolpath-rest-pass-no-tool = l'utensile { $tool } non è definito nel lavoro
toolpath-rest-pass-endmill = l'utensile { $tool } è una { $type }; una passata di ripresa taglia con una fresa
toolpath-rest-pass-larger = l'utensile { $tool } ({ $diameter }) deve essere più piccolo dell'utensile { $before } ({ $before_diameter }) che lo precede
toolpath-rest-pass-no-rpm = l'utensile { $tool } non ha una velocità mandrino (rpm)
