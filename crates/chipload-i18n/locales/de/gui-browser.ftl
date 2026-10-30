# The Browser: the project as one tree of folders and rows (crates/chipload-gui/src/app/browser.rs).

# The panel's caption, shown in capitals.
browser-caption = Browser
browser-no-project = Kein Projekt geöffnet.
browser-new-project = Neues Projekt

# A row's eye (shown on the canvas or not) and tick (cut or not).
browser-eye-hide-hover = Auf der Zeichenfläche ausblenden; wird trotzdem gefräst
browser-eye-show-hover = Auf der Zeichenfläche ausgeblendet (wird trotzdem gefräst): einblenden
browser-tick-on-hover = Wird gefräst: Haken entfernen, um es aus dem Programm zu lassen
browser-tick-off-hover = Wird nicht gefräst: Haken setzen, um es zu fräsen

# A project tool's link to the user's tool library, at its row's right end ($name: the
# library tool's name).
browser-link-changed = { $name } (geändert)
browser-link-missing = (nicht in Bibliothek)

# The kinds of operation, and what a new one of each does.
browser-profile = Konturfräsen
browser-profile-hover = Eine Konturbearbeitung für die ausgewählten Konturen
browser-pocket = Tasche
browser-pocket-hover = Eine Taschenbearbeitung für die ausgewählten Konturen
browser-vcarve = V-Gravur
browser-vcarve-hover = Ein V-Fräser entlang der Mittellinie der ausgewählten geschlossenen Konturen, so tief, dass der Schnitt so breit wird wie die Form
browser-follow-path = Pfad folgen
browser-follow-path-hover = Das Werkzeug auf der Linie der ausgewählten Formen, offen oder geschlossen: Gravuren, Nuten und Ritzlinien
browser-holes = Bohrungen
browser-holes-hover = Die ausgewählten Kreise: mit einem Bohrer gebohrt oder mit einem Schaftfräser per Helix gebohrt
browser-surface = Planfräsen
browser-surface-hover = Das ganze Rohteil mit einem breiten Schaftfräser plan abgetragen: die Opferplatte planen oder das Rohteil planfräsen. Braucht keine Auswahl

# Job settings and Tools.
browser-job-settings = Jobeinstellungen
browser-job-settings-hover = Rohteil, Maschine, Postprozessor und Werkzeuge
browser-work-zero = Nullpunkt
browser-work-zero-hover = Wo X0 Y0 Z0 des G-Codes liegt. Klicken, um ihn einzustellen, oder auswählen und in der Ansicht ziehen.
browser-job-settings-problems-hover = Rohteil, Maschine, Postprozessor und Werkzeuge. { $count ->
        [one] { $count } Problem
       *[other] { $count } Probleme
    }
browser-tools = Werkzeuge
browser-tools-hover = Die Werkzeuge des Projekts: klicken, um alle zu sehen und eines hinzuzufügen

# Parts: groups of operations, each cut with its copies before the next.
browser-parts = Teile
browser-parts-hover = Jedes Teil wird mit seinen Kopien vor dem nächsten gefräst; Rechtsklick für ein neues Teil
browser-setups = Aufspannungen
browser-setups-hover = Die Platten, aus denen das Projekt nacheinander gefräst wird: jede ein eigenes Programm, mit ihrem Rohteil und Nullpunkt. Klicken macht eine zur aktiven
browser-add-setup = Aufspannung hinzufügen
browser-add-setup-hover = Eine weitere Platte: eigenes Rohteil, eigener Nullpunkt und eigene Teile, als eigenes Programm gefräst
browser-setup-parts = { $count ->
    [one] { $count } Teil
   *[other] { $count } Teile
}
browser-setup-active-hover = Die aktive Aufspannung: in voller Farbe gezeigt, simuliert und berechnet. Ihr Rohteil und Nullpunkt stehen unter Jobeinstellungen und Nullpunkt
browser-setup-hover = Klicken macht sie zur aktiven Aufspannung: gezeigt, simuliert und berechnet
browser-move-to-setup = In Aufspannung verschieben
browser-new-part = Neues Teil
# At a part row's right end: the part new operations go into.
browser-part-active = aktiv
browser-part-active-hover = Das aktive Teil: neue Bearbeitungen kommen hierher. Klicken für seine Kopien; Rechtsklick für mehr
browser-part-hover = Klicken für seine Kopien; Doppelklick macht es zum aktiven Teil; Rechtsklick für mehr
# A verb: make this the active part.
browser-activate = Aktivieren
browser-activate-hover = Neue Bearbeitungen kommen in das aktive Teil
browser-export-part = G-Code dieses Teils exportieren…
browser-export-part-hover = Den G-Code nur dieses Teils schreiben, als wären alle anderen Teile ausgeschaltet; auch sein Einrichteblatt, wenn diese Einstellung an ist.
browser-new-operation = Neue Bearbeitung
browser-select-shapes-first = Zuerst Formen auswählen
browser-move-up = Nach oben
browser-move-down = Nach unten
# $count: the part's operations.
browser-delete-part = { $count ->
        [0] Teil löschen
        [one] Teil und seine Bearbeitung löschen
       *[other] Teil und seine { $count } Bearbeitungen löschen
    }
browser-delete-part-disabled-hover = Ein Projekt braucht mindestens ein Teil
browser-part-empty-active = Formen auswählen, dann eine Bearbeitung hinzufügen
browser-part-empty = Keine Bearbeitungen: ziehen Sie eine hierher

# An operation's row and its right-click menu.
browser-no-name = (kein Name)
# $count: contours the operation cuts that are hidden on the canvas.
browser-op-hidden = { $count } ausgeblendet
browser-duplicate = Duplizieren
browser-add-rest-pass = Restdurchgang hinzufügen
# A tool for a rest pass: $tool the project tool's number, $size its diameter.
browser-rest-project-tool = T{ $tool } · { $size } Schaftfräser
browser-rest-library-tool = { $name } · { $size } Schaftfräser
browser-from-library = Aus Ihrer Werkzeugbibliothek
browser-copied-into-project-hover = Wird in die Werkzeuge des Projekts kopiert
browser-add-rest-pass-hover = Ein kleineres Werkzeug, das nur räumt, was das Werkzeug dieser Tasche stehen lässt, in ihren Formen und Tiefen. Es folgt der Tasche und wird nach den benachbarten Taschen mit demselben Werkzeug gefräst
browser-make-rest-pass = Zum Restdurchgang von Bearbeitung { $number } machen
browser-make-rest-pass-hover = Übernimmt diese Tasche: als ihr Restdurchgang folgt es ihr und wird nach den benachbarten Taschen mit demselben Werkzeug gefräst
browser-delete = Löschen

# A rest pass's row. "Rest": a rest pass, a smaller tool after the pocket's own.
browser-rest = Rest
browser-rest-pass-hover = Ein Restdurchgang: räumt, was das Werkzeug davor stehen lässt, in den Formen und Tiefen seiner Tasche, und wird nach den benachbarten Taschen mit demselben Werkzeug gefräst
browser-add-floor-pass = Bodendurchgang hinzufügen
browser-add-floor-pass-hover = Ein Schaftfräser, der den Boden zwischen den Wänden zu breiter Striche ausräumt, in den Formen und Tiefen dieser V-Gravur. Er wird vor der V-Gravur gefräst
browser-floor = Boden
browser-floor-pass-hover = Ein Bodendurchgang: ein Schaftfräser räumt den Boden zwischen den Wänden seiner V-Gravur aus, vor der V-Gravur; wo er nicht hinkommt, räumt der V-Fräser nach
browser-remove = Entfernen

# An operation row's hover text: its lines.
browser-op-rest-after = Rest nach T{ $tool }
browser-op-not-cut = Nicht gefräst (abgehakt)
browser-op-hidden-hover = Fräst { $count } { $count ->
        [one] ausgeblendete Kontur
       *[other] ausgeblendete Konturen
    }: unter Zeichnung einblenden, um sie zu sehen
browser-op-drag-hover = Ziehen zum Umsortieren; Rechtsklick für mehr

# Properties: what is selected on the canvas.
browser-selection = Auswahl
browser-copy-as-selector = Als Selektor kopieren
browser-copy-as-selector-hover = Die Auswahl als JSON, für eine Bearbeitung in einer Projektdatei

# A thin line between operation rows where the tool changes; $tool the new tool's number.
browser-tool-change = Werkzeugwechsel → T{ $tool }
