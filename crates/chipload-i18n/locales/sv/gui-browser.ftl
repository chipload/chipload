# The Browser: the project as one tree of folders and rows (crates/chipload-gui/src/app/browser.rs).

# The panel's caption, shown in capitals.
browser-caption = Bläddrare
browser-no-project = Inget projekt öppet.
browser-new-project = Nytt projekt

# A row's eye (shown on the canvas or not) and tick (cut or not).
browser-eye-hide-hover = Dölj den på ritytan; den fräses ändå
browser-eye-show-hover = Dold på ritytan (fräses ändå): visa den
browser-tick-on-hover = Fräses: bocka ur för att utesluta den ur programmet
browser-tick-off-hover = Fräses inte: bocka i för att fräsa den

# A project tool's link to the user's tool library, at its row's right end ($name: the
# library tool's name).
browser-link-changed = { $name } (ändrat)
browser-link-missing = (inte i biblioteket)

# The kinds of operation, and what a new one of each does.
browser-profile = Konturfräsning
browser-profile-hover = En konturfräsning för de markerade konturerna
browser-pocket = Ficka
browser-pocket-hover = En fickfräsning för de markerade konturerna
browser-vcarve = V-gravyr
browser-vcarve-hover = En V-fräs längs mittlinjen av de markerade slutna konturerna, så djupt att spåret blir lika brett som formen
browser-follow-path = Följ bana
browser-follow-path-hover = Verktyget på linjen av de markerade formerna, öppna eller slutna: gravering, spår och ritslinjer
browser-holes = Hål
browser-holes-hover = De markerade cirklarna: borrade med borr, eller helixborrade med pinnfräs
browser-surface = Planfräsning
browser-surface-hover = Hela ämnet planfräst med en bred pinnfräs: plana offerskivan eller planfräs ämnet. Kräver ingen markering

# Job settings and Tools.
browser-job-settings = Jobbinställningar
browser-job-settings-hover = Ämne, maskin, postprocessor och verktyg
browser-work-zero = Arbetsnollpunkt
browser-work-zero-hover = Var G-kodens X0 Y0 Z0 ligger. Klicka för att ställa in den, eller markera den och dra den i vyn.
browser-job-settings-problems-hover = Ämne, maskin, postprocessor och verktyg. { $count ->
        [one] { $count } problem
       *[other] { $count } problem
    }
browser-tools = Verktyg
browser-tools-hover = Projektets verktyg: klicka för att se alla och lägga till ett

# Parts: groups of operations, each cut with its copies before the next.
browser-parts = Detaljer
browser-parts-hover = Varje detalj fräses med sina kopior före nästa; högerklicka för en ny detalj
browser-new-part = Ny detalj
# At a part row's right end: the part new operations go into.
browser-part-active = aktiv
browser-part-active-hover = Den aktiva detaljen: nya operationer hamnar här. Klicka för dess kopior; högerklicka för mer
browser-part-hover = Klicka för dess kopior; dubbelklicka för att göra den aktiv; högerklicka för mer
# A verb: make this the active part.
browser-activate = Aktivera
browser-activate-hover = Nya operationer hamnar i den aktiva detaljen
browser-export-part = Exportera G-kod för den här detaljen…
browser-export-part-hover = Skriv G-koden för bara den här detaljen, som om alla andra detaljer var avstängda; även dess uppsättningsblad när den inställningen är på.
browser-new-operation = Ny operation
browser-select-shapes-first = Markera former först
browser-move-up = Flytta upp
browser-move-down = Flytta ner
# $count: the part's operations.
browser-delete-part = { $count ->
        [0] Radera detalj
        [one] Radera detalj och dess operation
       *[other] Radera detalj och dess { $count } operationer
    }
browser-delete-part-disabled-hover = Ett projekt behöver minst en detalj
browser-part-empty-active = Markera former och lägg sedan till en operation
browser-part-empty = Inga operationer: dra hit en

# An operation's row and its right-click menu.
browser-no-name = (inget namn)
# $count: contours the operation cuts that are hidden on the canvas.
browser-op-hidden = { $count } dolda
browser-duplicate = Duplicera
browser-add-rest-pass = Lägg till restbearbetning
# A tool for a rest pass: $tool the project tool's number, $size its diameter.
browser-rest-project-tool = T{ $tool } · { $size } pinnfräs
browser-rest-library-tool = { $name } · { $size } pinnfräs
browser-from-library = Från ditt verktygsbibliotek
browser-copied-into-project-hover = Kopieras till projektets Verktyg
browser-add-rest-pass-hover = Ett mindre verktyg som bara tar bort det som fickans verktyg lämnar, i dess former och djup. Den följer fickan och fräses efter fickorna intill med samma verktyg
browser-make-rest-pass = Gör den till restbearbetning av operation { $number }
browser-make-rest-pass-hover = Den kopierar den fickan: som dess restbearbetning följer den fickan och fräses efter fickorna intill med samma verktyg
browser-delete = Radera

# A rest pass's row. "Rest": a rest pass, a smaller tool after the pocket's own.
browser-rest = Rest
browser-rest-pass-hover = En restbearbetning: den tar bort det som verktyget före lämnar, i fickans former och djup, och fräses efter fickorna intill sin ficka med samma verktyg
browser-add-floor-pass = Lägg till bottenpass
browser-add-floor-pass-hover = En pinnfräs som rensar botten mellan väggarna i streck som är för breda att gravera, i den här V-gravyrens former och djup. Den fräses före V-gravyren
browser-floor = Botten
browser-floor-pass-hover = Ett bottenpass: en pinnfräs rensar botten mellan V-gravyrens väggar, före V-gravyren; V-fräsen rensar efter där den inte når
browser-remove = Ta bort

# An operation row's hover text: its lines.
browser-op-rest-after = rest efter T{ $tool }
browser-op-not-cut = Fräses inte (urbockad)
browser-op-hidden-hover = Fräser { $count } { $count ->
        [one] dold kontur
       *[other] dolda konturer
    }: visa dem under Ritning för att se dem
browser-op-drag-hover = Dra för att ändra ordning; högerklicka för mer

# Properties: what is selected on the canvas.
browser-selection = Markering
browser-copy-as-selector = Kopiera som selektor
browser-copy-as-selector-hover = Markeringen som JSON, för en operation i en projektfil

# A thin line between operation rows where the tool changes; $tool the new tool's number.
browser-tool-change = verktygsbyte → T{ $tool }
