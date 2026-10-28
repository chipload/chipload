# The property forms: an operation, the job settings, tools, DXFs, pictures, texts, parts (crates/chipload-gui/src/app/forms.rs).

# Depths, shared by the operations. "Through": a button that sets the end depth through the stock.
forms-start-depth = Starttiefe
forms-start-depth-hover = Wo der Schnitt beginnt, gemessen von der Rohteiloberkante nach unten
forms-end-depth = Endtiefe
forms-end-depth-hover = Wo der Schnitt endet, gemessen von der Rohteiloberkante nach unten
forms-through = Durch
forms-through-hover = Durch das Rohteil fräsen: { $depth }, seine Dicke plus die zulässige Tiefe darunter (Jobeinstellungen > Rohteil)
forms-oversize = Übermaß
forms-oversize-disabled = Eine Kontur auf der Linie hat keine Seite, auf der mehr gefräst werden könnte
forms-step-down = Zustellung
# Indented under "Passes": the depth of each pass.
forms-step-down-sub = Zustellung

# Cutting direction (milling).
forms-direction = Richtung
forms-climb = Gleichlauf
forms-conventional = Gegenlauf

# Tabs' sloped sides.
forms-flank-angle = Flankenwinkel
forms-flank-angle-hover = Wie steil die Seiten des Haltestegs von seiner flachen Oberseite abfallen. 90° ist ein gerader Steg: das Werkzeug fährt senkrecht hoch und runter. Darunter folgt das Werkzeug der Schräge und taucht nach einem Steg nie senkrecht ein. Eine Flanke hinunter ist eine Rampe: mit einem nicht über Mitte schneidenden Werkzeug nicht steiler als der Rampenwinkel.
forms-flank-straight = gerade
forms-flank-feet = Fuß { $length } je Seite

# How the tool goes down into the material.
forms-entry = Eintauchen
forms-entry-hover = Wie das Werkzeug in das Material eintaucht. Eine Rampe fährt entlang der Bahn hinunter (eine Kontur spiralt um ihren Ring hinab, Pfad folgen fährt beim Hin- und Herfahren hinunter, eine Tasche fährt eine Rampe hinaus und zurück), eine Helix (Taschen) um einen kleinen Kreis, sodass das Werkzeug nicht über Mitte schneiden muss; beide mit dem Arbeitsvorschub.
forms-entry-plunge = senkrecht
forms-entry-ramp = Rampe
forms-entry-helix = Helix
forms-entry-ramp-summary = Rampe { $angle }
forms-entry-helix-summary = Helix r { $radius }, { $angle }
forms-entry-with-lead = { $entry }, Anfahrt { $radius }
forms-helix-radius = Helixradius
forms-helix-radius-hover = Des Kreises, dem die Werkzeugmitte folgt: kleiner als der Werkzeugradius, damit kein Kern stehen bleibt
forms-ramp-angle-sub = Rampenwinkel
forms-ramp-angle = Rampenwinkel
forms-ramp-angle-hover = Das Werkzeug spiralt entlang der Kontur hinunter, nie steiler als dieser Winkel und nie mehr als eine Zustellung pro Umlauf: kein senkrechtes Eintauchen ins Teil. Jeder Durchgang fährt in diesem Winkel hinunter und dann eben weiter; ein steilerer Winkel macht die Rampe kürzer.

# The operation kinds, in the form's title.
forms-kind-profile = Konturfräsen
forms-kind-pocket = Tasche
forms-kind-vcarve = V-Gravur
forms-kind-path = Pfad folgen
forms-kind-holes = Bohrungen
forms-kind-surface = Planfräsen
forms-operation-number = Bearbeitung { $number }

forms-name = Name
forms-geometry = Geometrie
forms-whole-stock = Das ganze Rohteil
forms-whole-stock-hover = Planfräsen bearbeitet das ganze Rohteil, einmal, unabhängig von den Kopien
forms-use-selection = Auswahl verwenden
forms-use-selection-count = Auswahl verwenden ({ $count })
forms-use-selection-hover = Durch die auf der Zeichenfläche ausgewählten Konturen ersetzen.
forms-use-selection-disabled = Zuerst Konturen auf der Zeichenfläche auswählen

# A profile's side of the line.
forms-side = Seite
forms-side-inside = innen
forms-side-outside = außen
forms-side-on = auf der Linie
forms-side-choose = — wählen —

# Section titles and their folded summaries. "Cut": how the operation cuts (noun).
forms-section-cut = Schnitt
forms-profile-cut-summary = { $start }–{ $end } { $unit }, Zustellung { $step } { $unit }, { $direction }, Spiralrampe { $ramp }°
forms-pocket-cut-summary = { $start }–{ $end } { $unit }, Zustellung { $step } { $unit }, { $direction }
forms-path-cut-summary = { $start }–{ $end } { $unit }, Zustellung { $step } { $unit }, auf der Linie
forms-surface-cut-summary = { $start }–{ $end } { $unit }, Zustellung { $step } { $unit }, seitl. Zustellung { $stepover } × D
forms-vcarve-cut-one-pass = ab { $start }, höchstens { $end } tief, ein Durchgang
forms-vcarve-cut-steps = ab { $start }, höchstens { $end } tief, Zustellung { $step }

# A pocket's clearing, in rings.
forms-section-clearing = Ausräumen
forms-clearing-summary = Ringe, seitl. Zustellung { $stepover } × D
forms-clearing-summary-rest = Ringe, seitl. Zustellung { $stepover } × D, Rest nach T{ $tool }
forms-stepover = Seitl. Zustellung
forms-stepover-hover = Wie weit jeder Ring weiterrückt, als Anteil des Werkzeugdurchmessers
forms-surface-stepover-hover = Wie weit jede Bahn seitlich weiterrückt, als Anteil des Werkzeugdurchmessers
forms-surface-end-depth-hover = Wie viel oben abgetragen wird, gemessen von der Rohteiloberkante nach unten
forms-section-entry-lead = Eintauchen und Anfahrt
forms-section-entry = Eintauchen

# V-carving.
forms-vcarve-start-depth-hover = Die gravierte Fläche, gemessen von der Rohteiloberkante nach unten (0: die Oberkante; mehr, um in einen Taschenboden zu gravieren)
forms-vcarve-end-depth-hover = Die größte Tiefe der Spitze, ab Rohteiloberkante (end_depth in der Datei). Breitere Striche werden bis dahin graviert und gemeldet
forms-passes = Durchgänge
forms-in-steps = in Stufen

# The operation's tool. "T3" is tool number 3.
forms-tool = Werkzeug
forms-tool-not-in-project = T{ $tool } (nicht im Projekt)
forms-from-library = Aus Ihrer Bibliothek:
forms-use-cutting-values = Schnittwerte des Werkzeugs verwenden
forms-use-cutting-values-hover = Aus der Bibliothek: Vorschub { $feed } / Eintauchen { $plunge } { $feed_unit }, Zustellung { $step_down }, seitl. Zustellung { $stepover }

forms-section-feeds = Vorschübe
forms-feed = Vorschub
forms-plunge-feed = Eintauchvorschub

# Rest machining: clear only what a larger tool left.
forms-rest-after = Rest nach
forms-rest-after-hover = Nur räumen, was ein größeres Werkzeug in diesen Formen stehen lässt: die Ecken, die es rund lässt, und die Schlitze, in die es nicht passt. Die Tasche zuerst mit jenem Werkzeug fräsen
forms-rest-none = keiner
forms-rest-endmill = { $diameter } Schaftfräser

# Square inside corners.
forms-corners = Ecken
forms-corners-hover = Innenecken, die ein rundes Werkzeug rund lässt: ein Dogbone geht entlang ihrer Mitte in die Ecke, ein T-Bone entlang der längeren Wand (die Kerbe liegt in der kürzeren), sodass ein eckiges Teil ganz hineinpasst
forms-corners-round = rund
forms-corners-dogbones = Dogbones
forms-corners-tbones = T-Bones
forms-corners-disabled = Eine Kontur auf der Linie hat keine Innenecken
forms-corners-up-to = bis
forms-corners-up-to-hover = Nur Ecken, die so spitz oder spitzer sind (leicht schief gezeichnete 90°-Ecken zählen noch)
forms-corners-chosen = { $count ->
    [one] { $count } Ecke einzeln gewählt
   *[other] { $count } Ecken einzeln gewählt
}
forms-corners-chosen-hover = Ecken, die auf der Zeichenfläche angeklickt und eigens gewählt wurden, egal was die Zeile darüber sagt
forms-corners-reset = Zurücksetzen
forms-corners-reset-hover = Jede Ecke bekommt wieder, was die Zeile darüber sagt

# Holes: drilled, or bored with an endmill round a helix.
forms-holes-drilled-pecks = gebohrt, Entspanen alle { $peck }
forms-holes-drilled-once = in einem Zug gebohrt
forms-holes-bore = Helixbohren, { $step } pro Umdrehung, { $direction }
forms-holes-cut-summary = { $start }–{ $end } { $unit }, { $method }
forms-holes-drill-end-depth-hover = Wo die Bohrerspitze endet, gemessen von der Rohteiloberkante nach unten: der volle Durchmesser endet etwa 0.3 × Bohrerdurchmesser höher
forms-pecks = Entspanen
forms-pecks-hover = Jeweils so tief bohren und dazwischen aus dem Loch fahren, um die Späne zu räumen
forms-in-pecks = mit Entspanen
forms-peck = Hubtiefe
forms-step-per-turn = Steigung
forms-step-per-turn-hover = Wie weit die Helix bei einer Umdrehung höchstens hinunterfährt
forms-bore-ramp-angle-hover = Die Helix ist nie steiler als dieser Winkel
forms-drill-feeds-summary = { $feed } { $unit } abwärts
forms-drill-feed = Bohrvorschub

# How a profile holds its part.
forms-holding-none = keine
forms-holding-tabs = Haltestege
forms-holding-skin = Restschicht
forms-holding-skin-tabs = Restschicht und Haltestege
forms-holding-summary-tabs = { $holding }: { $count } × { $width } { $unit }, { $height } { $unit } hoch
forms-holding-summary-skin = { $summary }, Schicht { $thickness }
forms-section-holding = Halten
forms-hold-the-part = Teil halten
forms-hold-the-part-hover =
    Wie ein ausgeschnittenes Teil an seinem Platz bleibt.
    Haltestege: Brücken, die unten im Schnitt stehen bleiben; das Teil bleibt verbunden, bis Sie es freischneiden.
    Restschicht: eine dünne Schicht bleibt beim Schruppen ringsum stehen, dann fräst ein letzter Umlauf nach allen Ringen der Bearbeitung sie durch. Das Teil ist während dieses Umlaufs lose: fügen Sie Haltestege hinzu, wenn sich die Teile dann nicht bewegen dürfen.
forms-tab-count = Anzahl Stege
forms-tab-width = Breite
forms-tab-height = Höhe
forms-tabs-placed = { $count ->
    [one] { $count } Haltesteg von Hand gesetzt
   *[other] { $count } Haltestege von Hand gesetzt
}
forms-tabs-placed-hover = Auf der Zeichenfläche gezogene, hinzugefügte oder entfernte Haltestege; Konturen ohne bekommen die Anzahl oben, gleichmäßig verteilt
forms-tabs-reset = Zurücksetzen
forms-tabs-reset-hover = Die Haltestege jeder Kontur werden wieder gleichmäßig verteilt
forms-skin-thickness = Schichtdicke
forms-skin-thickness-hover = Bleibt bis zum letzten Umlauf über der Endtiefe stehen: höchstens eine Zustellung, und dünner als die Haltestege

# A profile's finishing pass.
forms-finish-none = keiner
forms-finish-summary = { $stock } Aufmaß, dann ein Umlauf
forms-finishing-pass = Schlichtgang
forms-finishing-pass-hover = Mit so viel Aufmaß an der Wand schruppen, dann in einem leichten Umlauf auf voller Tiefe abtragen: eine sauberere Wand ohne Stufenspuren. Der Umlauf taucht im Schlitz des Schruppens ein, nie an der fertigen Wand.
forms-finishing-lap = Schlichtumlauf
forms-finishing-disabled = Eine Kontur auf der Linie hat keine Wand zum Schlichten
# The material left on the wall for the finishing lap.
forms-finish-stock = Aufmaß

# A pocket's lead-in and lead-out.
forms-lead = Anfahrt
forms-lead-hover = Im Bogen aus der geräumten Tasche an die Wand und wieder weg, damit das Werkzeug nie an der Wand stehen bleibt. Eine Anfahrt ohne Platz wird kleiner oder entfällt (der Bericht sagt es).
forms-lead-in-out = An- und Abfahren
forms-lead-radius = Radius

# The G-code's X0 Y0.
forms-work-zero = Nullpunkt X Y
forms-work-zero-hover = Der Punkt, der im G-Code X0 Y0 wird (Werkstücknullpunkt). Die Geometrie bewegt sich nicht.
forms-zero-drawing-origin = Zeichnungsursprung
forms-zero-stock-centre = Rohteilmitte
forms-zero-stock-bottom-left = Rohteil unten links
forms-zero-stock-bottom-right = Rohteil unten rechts
forms-zero-stock-top-left = Rohteil oben links
forms-zero-stock-top-right = Rohteil oben rechts
forms-zero-point = Punkt

# An imported DXF.
forms-file = Datei
forms-units = Einheiten
forms-dxf-inch = Zoll (in mm umgerechnet)
forms-shapes = Formen
forms-section-placement = Position, Drehung und Maßstab
forms-update-dxf = Aus einer DXF oder SVG aktualisieren…
forms-update-dxf-hover = Eine neue Version der Zeichnung: Bearbeitungen behalten ihre Layer
# A button (verb).
forms-remove = Entfernen
forms-remove-dxf-hover = Diese DXF und ihre Formen entfernen

# A picture traced into shapes.
forms-picture = Bild
forms-picture-pixels = { $width } × { $height } Pixel
forms-traced-as = Nachgezeichnet als
forms-traced-outlines = Umrisse
forms-traced-centre-lines = Mittellinien

forms-position = Position
forms-picture-reset-position = Ecke des Rahmens auf X0 Y0
forms-rotation = Drehung
forms-picture-rotation-hover = Grad gegen den Uhrzeigersinn, beliebiger Winkel, um die Bildmitte. Das Bild wird nachgezeichnet, wie es ist, dann werden seine Formen gedreht.
forms-turn-ccw-hover = 90° gegen den Uhrzeigersinn
forms-turn-cw-hover = 90° im Uhrzeigersinn
forms-picture-reset-rotation = Wie das Bild ist (0°)
forms-width = Breite
forms-picture-width-hover = Die Breite des Bildes vor dem Drehen; die Höhe folgt
forms-picture-height = Höhe { $height }
forms-layer = Layer
forms-picture-layer-hover = Der Layer seiner Formen, für Bearbeitungen, die ihn auswählen
forms-trace-settings = Nachzeichnen…
forms-trace-settings-hover = Schwellwert, Glättung, Umrisse oder Mittellinien, mit Vorschau
forms-remove-picture-hover = Das Bild und seine Formen entfernen

# A text written in a built-in font.
forms-text = Text
forms-text-hover = Was dasteht; Enter beginnt eine neue Zeile
forms-font = Schrift
forms-font-hover = Eine eingebaute Schrift: schlicht, Schreibschrift oder Zierschrift (Benutzerhandbuch: Text)
forms-height = Höhe
forms-text-height-hover = Die Höhe eines Großbuchstabens
forms-anchor = Anker
forms-anchor-hover = Was die Position ist: die untere linke Ecke des Textrahmens oder seine Mitte. Ein zentrierter Text bleibt zentriert, wenn sich Wörter, Schrift oder Größe ändern
forms-anchor-corner = untere linke Ecke
forms-anchor-centre = Mitte
forms-text-reset-position = Auf X0 Y0
forms-text-rotation-hover = Grad gegen den Uhrzeigersinn, beliebiger Winkel, um die Textmitte
forms-text-reset-rotation = Waagerecht (0°)
forms-align = Ausrichtung
forms-align-hover = Wie die Zeilen zueinander ausgerichtet sind
forms-align-left = links
forms-align-centre = Mitte
forms-align-right = rechts
forms-letter-spacing = Zeichenabstand
forms-letter-spacing-hover = Zusätzlicher Abstand zwischen Buchstaben; 0: der der Schrift
forms-line-spacing = Zeilenabstand
forms-line-spacing-hover = Von einer Grundlinie zur nächsten, in Versalhöhen
# After the number: "1.2 × height".
forms-line-spacing-suffix = × Höhe
forms-text-layer-hover = Der Layer seiner Buchstaben, für Bearbeitungen, die ihn auswählen
forms-size = Größe
forms-remove-text-hover = Den Text und seine Buchstaben entfernen

# The job settings. "Stock": the material the parts are cut from.
forms-job-settings = Jobeinstellungen
forms-section-stock = Rohteil
forms-stock-size = Größe X × Y
forms-thickness = Dicke
forms-material = Material
forms-material-hover = Ihre eigenen Vorschübe je Werkzeug, aus Dieser Computer > Materialien. Die Wahl gibt jeder Bearbeitung, auch später hinzugefügten, dieses Material und seine Werte. Das Rohteil bleibt, wie es ist
forms-material-none = Keins
forms-material-not-in-library = { $name } (nicht in Ihrer Bibliothek)
forms-material-from = Aus { $name }
forms-material-from-hover = Vorschübe, Zustellung und seitliche Zustellung sind die Werte des Materials für dieses Werkzeug
forms-material-changed = Geändert gegenüber { $name }
forms-material-use = Werte von { $name }
forms-material-use-hover = Mit den Werten des Materials für dieses Werkzeug schneiden
forms-material-save = In { $name } speichern
forms-material-save-hover = Diese Vorschübe, Zustellung und seitliche Zustellung als Werte des Materials für dieses Werkzeug behalten, für die nächsten Aufträge
forms-material-not-in = { $name } hat keine Werte für dieses Werkzeug
forms-material-no-values = { $name } (keine Werte für dieses Werkzeug)
forms-op-material-hover = Ihre eigenen Vorschübe je Werkzeug, aus Dieser Computer > Materialien: die Bearbeitung übernimmt die Werte dieses Materials für ihr Werkzeug und folgt ihnen, wenn Sie das Werkzeug wechseln
forms-max-overcut = Max. Tiefe unter Rohteil
forms-max-overcut-hover = Wie weit ein Schnitt unter das Rohteil gehen darf (in die Opferplatte). Schnitte innerhalb davon werden gewarnt; tiefere werden abgelehnt.
forms-stock-position = Rohteillage
forms-stock-corner-at-origin = Ecke im Zeichnungsursprung
forms-stock-centred-on-origin = mittig auf Zeichnungsursprung
forms-stock-centred-on-drawing = mittig auf der Zeichnung
forms-stock-margin = Rand
forms-stock-fits-hover = Die Umrisse der Zeichnung plus der Rand auf jeder Seite, mittig auf der Zeichnung; folgt der Zeichnung
forms-stock-fit = An Zeichnung anpassen
forms-stock-fit-hover = Das Rohteil mittig auf die Zeichnung legen und auf Zeichnung plus Rand bemessen, der Zeichnung folgend. Aus: eine feste Größe, die Sie eingeben, für eine Platte, die Sie verwenden wollen
forms-drawing-size = Zeichnung
forms-drawing-size-hover = Die Größe der Zeichnungsumrisse (was gefräst wird; ohne Hilfsgeometrie). Der Umriss des Schnitts mit dem Werkzeug steht nach Berechnen im Bericht
forms-z-zero = Z-Null
forms-z-zero-top = Rohteiloberkante
forms-z-zero-bottom = Rohteilunterkante
forms-machine-summary = { $post }, Sicherheitshöhe { $safe_z }, { $spindle }
forms-section-machine = Maschine
forms-from-machine = Von Maschine
forms-edit-here = Hier ändern
forms-edit-here-hover = Die Werte der Maschine nur für dieses Projekt ändern; sie bleibt verknüpft und zeigt, was abweicht
forms-safe-z-job = Sicherheitshöhe (dieser Job)
forms-safe-z-hover = Die Höhe für seitliche Eilgänge, über dem Rohteil und seinen Spannmitteln

# The project's tools.
forms-tools = Werkzeuge
forms-add-from-library = Aus Bibliothek hinzufügen…
forms-new-tool = Neues Werkzeug
forms-tool-not-used = nicht verwendet
# $operations: a list of operation numbers, "1, 3".
forms-tool-used-by = Bearbeitungen { $operations }
forms-tool-not-used-long = Von keiner Bearbeitung verwendet
forms-tool-cuts = Fräst Bearbeitungen { $operations }
forms-remove-tool = Werkzeug entfernen
forms-remove-tool-disabled = Verwendet von Bearbeitungen { $operations }; geben Sie ihnen zuerst ein anderes Werkzeug
# An icon button that puts a value back.
forms-reset = Zurücksetzen

# A DXF's position, rotation and scale.
forms-dxf-reset-position = Zurück an die Stelle aus der DXF (Drehung und Maßstab bleiben)
forms-bottom-left = Unten links
forms-bottom-left-hover = Die Ecke der Zeichnung { $margin } innerhalb der unteren linken Rohteilecke
# A button: centre the drawing on the stock.
forms-centre = Zentrieren
forms-centre-hover = Die Mitte der Zeichnung auf die Mitte des Rohteils
forms-dxf-rotation-hover = Grad gegen den Uhrzeigersinn gegenüber der gezeichneten DXF, beliebiger Winkel, um die Zeichnungsmitte
forms-dxf-reset-rotation = Zurück zum Winkel der DXF (0°)
forms-scale = Maßstab
forms-scale-hover = Gegenüber der gezeichneten DXF, um die Zeichnungsmitte. Werkzeuggrößen, Tiefen und Haltestege werden nicht skaliert.
forms-dxf-reset-scale = Zurück zur Größe der DXF (× 1)

# A part: a group of operations cut one after the other.
forms-part-number = Teil { $number }
# Label of the on/off checkbox (verb): whether the part is cut.
forms-part-cut = Fräsen
forms-part-cut-hover = Aus: das Teil und seine Bearbeitungen bleiben im Projekt, werden aber nicht gefräst
forms-part-cut-check = dieses Teil und seine Bearbeitungen
forms-active = Aktiv
forms-active-hover = Neue Bearbeitungen kommen in das aktive Teil. Rechtsklick auf ein Teil aktiviert es
forms-active-yes = ja: neue Bearbeitungen kommen hierher
forms-activate = Aktivieren
forms-operations = Bearbeitungen
forms-operations-none = noch keine
forms-copies-one = eine
forms-copies = Kopien
forms-parts-note = Teile werden nacheinander gefräst, jedes mit seinen Kopien.
forms-copies-hover = Das Teil mehrmals in einem Raster fräsen. Die Kopien sind blass gezeichnet; nur die Zeichnung selbst kann ausgewählt werden.
forms-copies-grid = mehrere, im Raster
forms-columns-rows = Spalten × Zeilen
# Copy to copy distance, in X and Y.
forms-pitch = Teilung X Y
forms-gap = Abstand
forms-gap-hover = Setzt die Teilung einmalig so, dass so viel zwischen den Schnitten benachbarter Kopien bleibt. Das Projekt behält die Teilung.
forms-set-pitch = Teilung setzen
forms-order = Reihenfolge

# The machine. "Post": the post-processor that writes the G-code.
forms-post = Post
forms-post-yours = { $name } (eigener)
forms-post-folder-hover = Ihre eigenen Postprozessoren gehören in { $folder }
forms-post-builtin-hover = Die eingebauten Postprozessoren
forms-gcode-units = G-Code-Einheiten
forms-gcode-units-hover = Die Einheiten, in denen der G-Code geschrieben wird: mm (G21) oder Zoll (G20), wie Ihre Steuerung eingestellt ist. Alles andere im Projekt bleibt in mm
forms-gcode-mm = mm (G21)
forms-gcode-inch = Zoll (G20)
forms-spindle = Spindel
forms-spindle-cw = Rechtslauf (M3)
forms-spindle-ccw = Linkslauf (M4)
forms-max-feed = Max. Vorschub
forms-tool-changes = Werkzeugwechsel
forms-tool-changes-hover = Mit mehreren Werkzeugen: eine G-Code-Datei je Werkzeug (zwischen den Dateien das Werkzeug wechseln und Z-Null setzen) oder eine Datei mit M6-Werkzeugwechseln, die Ihr Sender ausführt (GRBL selbst lehnt M6 ab)
forms-tool-change-files = eine Datei je Werkzeug
forms-tool-change-program = eine Datei mit M6 (Ihr Sender führt den Wechsel aus)
forms-travel = Verfahrweg X Y Z
forms-max-feed-not-set = nicht gesetzt
forms-travel-not-checked = nicht geprüft
forms-revert-hover = „{ $name }“ hat { $value }: klicken, um das zu übernehmen
forms-post-yours-copied = Eigener, ins Projekt kopiert
forms-post-changed = Ihre Postprozessor-Datei hat sich geändert
forms-post-update = Aktualisieren
forms-post-update-hover = Den Postprozessor in seinem jetzigen Stand ins Projekt kopieren
forms-post-file-missing = Eigener; das Projekt behält seine Kopie (die Datei ist nicht vorhanden)

# A tool's values.
forms-tool-type = Typ
forms-diameter = Durchmesser
forms-spindle-speed = Drehzahl
forms-centre-cutting = Über Mitte schneidend
forms-yes = ja
forms-no = nein
forms-unknown = unbekannt
forms-angle = Winkel
forms-angle-hover = Der Spitzenwinkel des V; die V-Gravur braucht ihn
forms-tip = Spitze
forms-tip-hover = Der Durchmesser der flachen Spitze; nicht gesetzt gilt als spitz
forms-pointed = spitz

# A rest pass of a pocket: a smaller tool clears what the one before left.
forms-rest-pass-of = Bearbeitung { $number }
forms-rest-pass-of-named = Bearbeitung { $number } ({ $name })
forms-rest-pass-facts = { $start }–{ $end } { $unit }, Übermaß { $clearance } { $unit }, { $direction }
forms-rest-pass = Restdurchgang
forms-rest-of = Rest von
forms-rest-of-hover = Er fräst die Formen und Tiefen seiner Tasche und räumt nur, was das Werkzeug davor stehen lässt
# A button: show the pocket.
forms-open = Öffnen
forms-from-pocket = Von der Tasche
forms-after-tool = Nach Werkzeug
forms-after-tool-hover = Er räumt, was dieses Werkzeug stehen lässt: das der Tasche oder des Durchgangs davor
forms-cut-it = Fräsen
forms-cut-it-hover = Aus: behalten, aber nicht gefräst
forms-rest-pass-note = Wird nach den benachbarten Taschen seiner Tasche mit demselben Werkzeug gefräst, alle ihre Restdurchgänge zusammen: das kleine Werkzeug kommt nur einmal.
forms-remove-rest-pass = Restdurchgang entfernen

# Font styles, headings in the font list.
forms-font-plain = Schlicht
forms-font-script = Schreibschrift
forms-font-display = Zierschrift
forms-explode-dxf = Auflösen…
forms-explode-dxf-hover = Die Formen dieser DXF in gezeichnete Formen umwandeln, die Sie bearbeiten können; die Verknüpfung zur DXF-Datei geht verloren
