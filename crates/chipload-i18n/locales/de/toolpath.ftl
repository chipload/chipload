# Messages of the planner (crates/chipload-toolpath): rules a project must meet before it
# can be cut. Most are shown after the operation's label, as "Operation 3 (Outline): …";
# see job-operation-problem. Words in the texts that name fields or values of the project
# file (safe_z, z_zero, max_feed, entry: ramp, center_cutting, …) are written in the file
# that way and are never translated. A length or feed argument brings its own unit
# ("12.5 mm", "1200 mm/min").

## The project as a whole

toolpath-no-operations = Der Job hat keine Bearbeitungen
toolpath-all-disabled = Alle Bearbeitungen sind deaktiviert: nichts zu fräsen
# $tool is a tool's number.
toolpath-tool-undefined = Werkzeug { $tool } ist im Job nicht definiert
# Under the tool's spindle speed field.
toolpath-no-rpm-short = keine Drehzahl
toolpath-no-rpm = Werkzeug { $tool } hat keine Drehzahl (U/min)
# Under the machine's safe height; $top is the height of the stock's top.
toolpath-safe-z-short = muss über der Rohteiloberkante liegen (Z { $top })
# $zero is "top" or "bottom", as written in the project file.
toolpath-safe-z = safe_z { $safe } muss über der Rohteiloberkante liegen (Z { $top }); z_zero ist '{ $zero }'

## Depths and sizes

# $below is how far below the stock's bottom it goes, $allowed the setting "max depth
# below stock".
toolpath-too-deep = Endtiefe { $end } liegt { $below } unter dem Rohteil ({ $thickness } dick), mehr als die zulässigen { $allowed } (Max. Tiefe unter Rohteil)
toolpath-clearance-radius = Übermaß { $clearance } muss kleiner als der Werkzeugradius { $radius } sein
toolpath-finish-radius = Aufmaß { $stock } für den Schlichtgang muss kleiner als der Werkzeugradius { $radius } sein
toolpath-tab-height = Höhe der Haltestege { $height } muss kleiner als die Schnitttiefe { $cut } sein (Endtiefe minus Starttiefe)
# $field is "feed" or "plunge_feed", as written in the project file.
toolpath-max-feed = { $field } { $feed } überschreitet max_feed { $max } der Maschine
toolpath-helix-for-path = Helix-Eintauchen ist für Taschen; Pfad folgen kann per Rampe eintauchen (entry: ramp)
toolpath-helix-radius = Helixradius { $radius } muss kleiner als der Werkzeugradius { $tool_radius } sein, sonst bleibt in der Mitte ein Kern stehen
# $most is a number of tool diameters, $longest that as a length.
toolpath-lead-radius = Anfahrradius { $radius } ist mehr als { $most } × Werkzeugdurchmesser ({ $longest })

## Tools

# $tool is a tool's number, $type a kind of tool (job-tool-type).
toolpath-surface-endmill = Werkzeug { $tool } ist ein { $type }; Planfräsen braucht einen Schaftfräser
toolpath-vcarve-vbit = V-Gravur braucht einen V-Fräser, Werkzeug { $tool } ist ein { $type }
toolpath-vbit-no-angle = Werkzeug { $tool } ist ein V-Fräser ohne Winkel; geben Sie seinen Winkel an
toolpath-pocket-endmill = Taschen brauchen einen Schaftfräser, Werkzeug { $tool } ist ein { $type }
toolpath-path-drill = Werkzeug { $tool } ist ein Bohrer und kann keinem Pfad folgen; verwenden Sie einen Schaftfräser oder V-Fräser
toolpath-holes-vbit = Werkzeug { $tool } ist ein V-Fräser und kann keine Bohrungen fräsen; verwenden Sie einen Bohrer oder Schaftfräser
# $method is the holes' method as written in the project file, "drill" or "bore"; $type
# a kind of tool with its article (job-tool-type-article).
toolpath-holes-method = die Methode ist { $method ->
    [drill] drill, die einen Bohrer braucht
   *[bore] bore, die einen Schaftfräser braucht
}; Werkzeug { $tool } ist { $type }
# Angles in degrees.
toolpath-flank-not-centre-cutting = Werkzeug { $tool } schneidet nicht über Mitte, und die Flanken der Haltestege ({ $flank }°) sind steiler als der Rampenwinkel ({ $ramp }°); eine Flanke hinunter ist eine Rampe, also machen Sie sie nicht steiler als { $ramp }°
toolpath-plunge-not-centre-cutting = Werkzeug { $tool } schneidet nicht über Mitte, und diese Bearbeitung taucht senkrecht ein; tauchen Sie stattdessen per Rampe ein (entry: ramp) oder verwenden Sie ein über Mitte schneidendes Werkzeug

## Rest machining

# $tool is the number of the larger tool the pocket rests after.
toolpath-rest-no-tool = Rest nach Werkzeug { $tool }, das es im Projekt nicht gibt
toolpath-rest-not-endmill = Rest nach Werkzeug { $tool }, ein { $type }; das Werkzeug einer Tasche ist ein Schaftfräser
# Two tool diameters.
toolpath-rest-not-larger = Rest nach Werkzeug { $tool } ({ $large }) braucht ein größeres Werkzeug als das dieser Tasche ({ $small })
toolpath-rest-dogbones = Dogbones und Restbearbeitung lassen sich noch nicht kombinieren; versehen Sie die Tasche des großen Werkzeugs mit Dogbones
toolpath-rest-passes-dogbones = Dogbones und Restdurchgänge lassen sich noch nicht kombinieren; Dogbone-Ecken brauchen keinen Rest
toolpath-rest-pass-no-tool = Werkzeug { $tool } ist im Job nicht definiert
toolpath-rest-pass-endmill = Werkzeug { $tool } ist ein { $type }; ein Restdurchgang fräst mit einem Schaftfräser
toolpath-rest-pass-larger = Werkzeug { $tool } ({ $diameter }) muss kleiner sein als Werkzeug { $before } ({ $before_diameter }) davor
toolpath-rest-pass-no-rpm = Werkzeug { $tool } hat keine Drehzahl (U/min)
