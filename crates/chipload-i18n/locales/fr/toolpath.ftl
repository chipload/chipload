# Messages of the planner (crates/chipload-toolpath): rules a project must meet before it
# can be cut. Most are shown after the operation's label, as "Operation 3 (Outline): …";
# see job-operation-problem. Words in the texts that name fields or values of the project
# file (safe_z, z_zero, max_feed, entry: ramp, center_cutting, …) are written in the file
# that way and are never translated. A length or feed argument brings its own unit
# ("12.5 mm", "1200 mm/min").

## The project as a whole

toolpath-no-operations = Le projet n'a aucune opération
toolpath-all-disabled = Toutes les opérations sont désactivées : rien à usiner
# $tool is a tool's number.
toolpath-tool-undefined = L'outil { $tool } n'est pas défini dans le projet
# Under the tool's spindle speed field.
toolpath-no-rpm-short = aucune vitesse de broche
toolpath-no-rpm = L'outil { $tool } n'a pas de vitesse de broche (tr/min)
# Under the machine's safe height; $top is the height of the stock's top.
toolpath-safe-z-short = doit être au-dessus du dessus du brut (Z { $top })
# $zero is "top" or "bottom", as written in the project file.
toolpath-safe-z = safe_z { $safe } doit être au-dessus du dessus du brut (Z { $top }) ; z_zero vaut '{ $zero }'

## Depths and sizes

# $below is how far below the stock's bottom it goes, $allowed the setting "max depth
# below stock".
toolpath-too-deep = la profondeur finale { $end } descend { $below } sous le brut ({ $thickness } d'épaisseur), plus que les { $allowed } permis (Prof. max sous le brut)
toolpath-clearance-radius = la surcote { $clearance } doit être inférieure au rayon de l'outil { $radius }
toolpath-finish-radius = l'épaisseur de finition { $stock } doit être inférieure au rayon de l'outil { $radius }
toolpath-tab-height = la hauteur des attaches { $height } doit être inférieure à la profondeur de coupe { $cut } (profondeur finale moins profondeur de départ)
# $field is "feed" or "plunge_feed", as written in the project file.
toolpath-max-feed = { $field } { $feed } dépasse max_feed de la machine { $max }
toolpath-helix-for-path = l'entrée en hélice est pour les poches ; un suivi de tracé peut entrer en rampe (entry: ramp)
toolpath-helix-radius = le rayon d'hélice { $radius } doit être inférieur au rayon de l'outil { $tool_radius }, sinon un noyau reste debout au milieu
# $most is a number of tool diameters, $longest that as a length.
toolpath-lead-radius = le rayon d'approche { $radius } dépasse { $most } × le diamètre de l'outil ({ $longest })

## Tools

# $tool is a tool's number, $type a kind of tool (job-tool-type).
toolpath-surface-endmill = l'outil { $tool } est de type { $type } ; un surfaçage a besoin d'une fraise
toolpath-vcarve-vbit = la gravure en V a besoin d'une fraise en V, l'outil { $tool } est de type { $type }
toolpath-vbit-no-angle = l'outil { $tool } est une fraise en V sans angle ; réglez son angle
toolpath-pocket-endmill = les poches ont besoin d'une fraise, l'outil { $tool } est de type { $type }
toolpath-path-drill = l'outil { $tool } est un foret, qui ne peut pas suivre un tracé ; utilisez une fraise ou une fraise en V
toolpath-holes-vbit = l'outil { $tool } est une fraise en V, qui ne peut pas usiner de perçages ; utilisez un foret ou une fraise
# $method is the holes' method as written in the project file, "drill" or "bore"; $type
# a kind of tool with its article (job-tool-type-article).
toolpath-holes-method = la méthode est { $method ->
    [drill] drill, qui demande un foret
   *[bore] bore, qui demande une fraise
} ; l'outil { $tool } est { $type }
# Angles in degrees.
toolpath-flank-not-centre-cutting = l'outil { $tool } ne coupe pas au centre et les flancs des attaches ({ $flank }°) sont plus raides que l'angle de rampe ({ $ramp }°) ; descendre un flanc est une rampe, rendez-les donc au plus aussi raides que { $ramp }°
toolpath-plunge-not-centre-cutting = l'outil { $tool } ne coupe pas au centre et cette opération plonge à la verticale ; entrez plutôt en rampe (entry: ramp) ou utilisez un outil qui coupe au centre

## Rest machining

# $tool is the number of the larger tool the pocket rests after.
toolpath-rest-no-tool = reprise après l'outil { $tool }, que le projet n'a pas
toolpath-rest-not-endmill = reprise après l'outil { $tool }, de type { $type } ; l'outil d'une poche est une fraise
# Two tool diameters.
toolpath-rest-not-larger = la reprise après l'outil { $tool } ({ $large }) demande un outil plus gros que celui de cette poche ({ $small })
toolpath-rest-dogbones = les dogbones et la reprise des restes ne peuvent pas encore être combinés ; mettez les dogbones sur la poche du gros outil
toolpath-rest-passes-dogbones = les dogbones et les passes de reprise ne peuvent pas encore être combinés ; les coins en dogbone n'ont pas besoin de reprise
toolpath-rest-pass-no-tool = l'outil { $tool } n'est pas défini dans le projet
toolpath-rest-pass-endmill = l'outil { $tool } est de type { $type } ; une passe de reprise usine avec une fraise
toolpath-rest-pass-larger = l'outil { $tool } ({ $diameter }) doit être plus petit que l'outil { $before } ({ $before_diameter }) qui le précède
toolpath-rest-pass-no-rpm = l'outil { $tool } n'a pas de vitesse de broche (tr/min)
