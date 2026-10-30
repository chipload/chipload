# The property forms: an operation, the job settings, tools, DXFs, pictures, texts, parts (crates/chipload-gui/src/app/forms.rs).

# Depths, shared by the operations. "Through": a button that sets the end depth through the stock.
forms-start-depth = Startdjup
forms-start-depth-hover = Där fräsningen börjar, nedåt från ämnets ovansida
forms-end-depth = Slutdjup
forms-end-depth-hover = Där fräsningen slutar, nedåt från ämnets ovansida
forms-through = Igenom
forms-through-hover = Fräs igenom ämnet: slutdjupet är dess tjocklek plus djupet under det, och följer tjockleken när den ändras
forms-through-below = Under ämnet
forms-through-below-hover = Hur långt under ämnets undersida fräsningen går, ner i offerskivan. Jobbinställningar > Maskin anger det största tillåtna ({ $depth })
forms-oversize = Förskjutning
forms-oversize-disabled = En konturfräsning på linjen har ingen sida att fräsa mer på
forms-step-down = Skärdjup
# Indented under "Passes": the depth of each pass.
forms-step-down-sub = skärdjup

# Cutting direction (milling).
forms-direction = Riktning
forms-climb = medfräsning
forms-conventional = motfräsning

# Tabs' sloped sides.
forms-flank-angle = flankvinkel
forms-flank-angle-hover = Hur brant bryggans sidor sluttar ned från dess plana topp. 90° är en rak brygga: verktyget går rakt upp och ned. Under det följer verktyget lutningen, så det matas aldrig rakt ned efter en brygga. Att gå ned längs en flank är en ramp: med ett verktyg som inte är centrumskärande, inte brantare än rampvinkeln.
forms-flank-straight = rak
forms-flank-feet = fötter { $length } per sida

# How the tool goes down into the material.
forms-entry = Nedgång
forms-entry-hover = Hur verktyget går ned i materialet. En ramp går ned längs banan (en konturfräsning går i spiral ned runt sin slinga, Följ bana går ned medan den går fram och tillbaka, en ficka rampar ut och tillbaka), en helix (fickor) runt en liten cirkel, så verktyget behöver inte vara centrumskärande; båda använder skärmatningen.
forms-entry-plunge = rak nedmatning
forms-entry-ramp = ramp
forms-entry-helix = helix
forms-entry-ramp-summary = ramp { $angle }
forms-entry-helix-summary = helix r { $radius }, { $angle }
forms-entry-with-lead = { $entry }, inkörning { $radius }
forms-helix-radius = helixradie
forms-helix-radius-hover = För cirkeln som verktygets centrum följer: mindre än verktygsradien, så ingen kärna blir stående
forms-ramp-angle-sub = rampvinkel
forms-ramp-angle = Rampvinkel
forms-ramp-angle-hover = Verktyget går i spiral ned längs konturen, aldrig brantare än detta och aldrig mer än ett skärdjup: ingen rak nedmatning i detaljen. Varje djupsteg går ned med denna vinkel och fortsätter sedan plant; en brantare vinkel ger en kortare ramp.

# The operation kinds, in the form's title.
forms-kind-profile = Konturfräsning
forms-kind-pocket = Ficka
forms-kind-vcarve = V-gravyr
forms-kind-path = Följ bana
forms-kind-holes = Hål
forms-kind-surface = Planfräsning
forms-operation-number = Operation { $number }

forms-name = Namn
forms-geometry = Geometri
forms-whole-stock = Hela ämnet
forms-whole-stock-hover = En planfräsning planar hela ämnet, en gång, oavsett kopior
forms-use-selection = Använd markering
forms-use-selection-count = Använd markering ({ $count })
forms-use-selection-hover = Ersätt med konturerna som är markerade på ritytan.
forms-use-selection-disabled = Markera konturer på ritytan först

# A profile's side of the line.
forms-side = Sida
forms-side-inside = inuti
forms-side-outside = utanför
forms-side-on = på linjen
forms-side-choose = — välj —

# Section titles and their folded summaries. "Cut": how the operation cuts (noun).
forms-section-cut = Fräsning
forms-profile-cut-summary = { $start }–{ $end } { $unit }, steg om { $step } { $unit }, { $direction }, spiralramp { $ramp }°
forms-pocket-cut-summary = { $start }–{ $end } { $unit }, steg om { $step } { $unit }, { $direction }
forms-path-cut-summary = { $start }–{ $end } { $unit }, steg om { $step } { $unit }, på linjen
forms-surface-cut-summary = { $start }–{ $end } { $unit }, steg om { $step } { $unit }, sidosteg { $stepover } × D
forms-vcarve-cut-one-pass = från { $start }, högst { $end } djupt, ett skär
forms-vcarve-cut-steps = från { $start }, högst { $end } djupt, steg om { $step }

# A pocket's clearing, in rings.
forms-section-clearing = Urfräsning
forms-clearing-summary = slingor, sidosteg { $stepover } × D
forms-clearing-summary-rest = slingor, sidosteg { $stepover } × D, rest efter T{ $tool }
forms-stepover = Sidosteg
forms-stepover-hover = Hur långt varje slinga flyttas, som andel av verktygsdiametern
forms-surface-stepover-hover = Hur långt varje skär flyttas i sidled, som andel av verktygsdiametern
forms-surface-end-depth-hover = Hur mycket som tas bort från toppen, nedåt från ämnets ovansida
forms-section-entry-lead = Nedgång och inkörning
forms-section-entry = Nedgång

# V-carving.
forms-vcarve-start-depth-hover = Ytan som graveras, nedåt från ämnets ovansida (0: ovansidan; mer för att gravera i ett fickgolv)
forms-vcarve-end-depth-hover = Det djupaste spetsen går, från ämnets ovansida (end_depth i filen). Bredare streck graveras till detta djup och rapporteras
forms-passes = Djupsteg
forms-in-steps = i steg

# The operation's tool. "T3" is tool number 3.
forms-tool = Verktyg
forms-tool-not-in-project = T{ $tool } (inte i projektet)
forms-from-library = Från ditt bibliotek:
forms-use-cutting-values = Använd verktygets skärdata
forms-use-cutting-values-hover = Från biblioteket: matning { $feed } / nedmatning { $plunge } { $feed_unit }, skärdjup { $step_down }, sidosteg { $stepover }

forms-section-feeds = Matningar
forms-feed = Matning
forms-chip-load = Matning per tand
forms-chip-load-no-flutes = antal skär inte angivet
forms-chip-load-no-rpm = spindelvarvtal inte angivet
forms-chip-load-missing-hover = Ange dem i verktygets egenskaper: Verktyg i bläddraren, sedan verktyget.
forms-flutes = Skär
forms-flutes-hover = Verktygets skär, för matningen per tand. Ej markerad: okänt.
forms-plunge-feed = Nedmatning

# Rest machining: clear only what a larger tool left.
forms-rest-after = Rest efter
forms-rest-after-hover = Ta bara bort det som ett större verktyg lämnar i dessa former: hörnen det lämnar runda och spåren det inte får plats i. Fräs fickan med det verktyget först
forms-rest-none = ingen
forms-rest-endmill = { $diameter } pinnfräs

# Square inside corners.
forms-corners = Hörn
forms-corners-hover = Innerhörn som ett runt verktyg lämnar runda: en dogbone går in i hörnet längs dess mitt, en T-bone längs den längre väggen (urtaget hamnar i den kortare), så att en fyrkantig detalj passar ända in
forms-corners-round = runda
forms-corners-dogbones = dogbones
forms-corners-tbones = T-bones
forms-corners-disabled = En konturfräsning på linjen har inga innerhörn
forms-corners-up-to = upp till
forms-corners-up-to-hover = Bara hörn som är så här spetsiga eller spetsigare (90°-hörn som ritats lite snett räknas ändå)
forms-corners-chosen = { $count ->
    [one] { $count } hörn valt för sig
   *[other] { $count } hörn valda ett och ett
}
forms-corners-chosen-hover = Hörn som klickats på i ritytan och fått ett eget val, vad raden ovan än säger
forms-corners-reset = Återställ
forms-corners-reset-hover = Alla hörn får igen det raden ovan säger

# Holes: drilled, or bored with an endmill round a helix.
forms-holes-drilled-pecks = borrade, stegvis { $peck }
forms-holes-drilled-once = borrade i ett svep
forms-holes-bore = helixborrning, { $step } per varv, { $direction }
forms-holes-cut-summary = { $start }–{ $end } { $unit }, { $method }
forms-holes-drill-end-depth-hover = Där borrspetsen slutar, nedåt från ämnets ovansida: full diameter slutar ungefär 0.3 × borrdiametern högre upp
forms-pecks = Stegborrning
forms-pecks-hover = Borra så här djupt åt gången och gå upp ur hålet emellan för att få bort spånorna
forms-in-pecks = stegvis
forms-peck = steg
forms-step-per-turn = Steg per varv
forms-step-per-turn-hover = Det mesta helixen går ned på ett varv
forms-bore-ramp-angle-hover = Helixen är aldrig brantare än detta
forms-drill-feeds-summary = { $feed } { $unit } nedåt
forms-drill-feed = Borrmatning

# How a profile holds its part.
forms-holding-none = ingen
forms-holding-tabs = bryggor
forms-holding-skin = tunt skikt
forms-holding-skin-tabs = tunt skikt och bryggor
forms-holding-summary-tabs = { $holding }: { $count } × { $width } { $unit }, { $height } { $unit } höga
forms-holding-summary-skin = { $summary }, skikt { $thickness }
forms-section-holding = Fasthållning
forms-hold-the-part = Håll detaljen
forms-hold-the-part-hover =
    Hur en urfräst detalj stannar på plats.
    Bryggor: lämnas kvar i botten av spåret; detaljen sitter fast tills du skär loss den.
    Tunt skikt: ett tunt skikt lämnas runt om under grovbearbetningen, sedan fräser ett sista varv efter operationens alla slingor igenom det. Detaljen är lös under det varvet: lägg till bryggor om detaljerna inte får röra sig då.
forms-tab-count = antal bryggor
forms-tab-width = bredd
forms-tab-height = höjd
forms-tabs-placed = { $count ->
    [one] { $count } brygga placerad för hand
   *[other] { $count } bryggor placerade för hand
}
forms-tabs-placed-hover = Bryggor som dragits, lagts till eller tagits bort i ritytan; konturer utan några får antalet ovan, jämnt spridda
forms-tabs-reset = Återställ
forms-tabs-reset-hover = Alla konturers bryggor sprids jämnt igen
forms-skin-thickness = skikttjocklek
forms-skin-thickness-hover = Lämnas ovanför slutdjupet till sista varvet: högst ett skärdjup, och tunnare än bryggorna

# A profile's finishing pass.
forms-finish-none = inget
forms-finish-summary = { $stock } kvar, sedan ett varv
forms-finishing-pass = Finskär
forms-finishing-pass-hover = Grovbearbeta med så här mycket kvar på väggen och fräs sedan bort det i ett lätt varv på fullt djup: en renare vägg utan stegmärken. Varvet går ned i grovbearbetningens spår, aldrig på den färdiga väggen.
forms-finishing-lap = finskärsvarv
forms-finishing-disabled = En konturfräsning på linjen har ingen vägg att finbearbeta
# The material left on the wall for the finishing lap.
forms-finish-stock = kvar

# A pocket's lead-in and lead-out.
forms-lead = Inkörning
forms-lead-hover = En båge in mot väggen från den urfrästa fickan och ut igen, så att verktyget aldrig stannar mot väggen. En inkörning utan utrymme krymps eller utelämnas (rapporten säger det).
forms-lead-in-out = inkörning och utkörning
forms-lead-radius = radie

# The G-code's X0 Y0.
forms-work-zero = Arbetsnollpunkt X Y
forms-work-zero-hover = Punkten som blir X0 Y0 i G-koden. Geometrin flyttas inte.
forms-zero-drawing-origin = ritningens origo
forms-zero-stock-centre = ämnets centrum
forms-zero-stock-bottom-left = ämnets nedre vänstra hörn
forms-zero-stock-bottom-right = ämnets nedre högra hörn
forms-zero-stock-top-left = ämnets övre vänstra hörn
forms-zero-stock-top-right = ämnets övre högra hörn
forms-zero-point = punkt
forms-zero-on-stock = På ämnet
forms-zero-on-stock-hover = Klicka på ett hörn eller mitten: nollpunkten stannar där när ämnets storlek ändras
forms-zero-drawing-origin-button = Ritningens origo
forms-zero-drawing-origin-hover = X0 Y0 där ritningen har sin egen 0,0
forms-zero-at = Position
forms-zero-at-hover = Var nollpunkten ligger, i ritningens koordinater. Ett inskrivet värde gör den till en egen punkt.
forms-zero-note = G-kodens X0 Y0 Z0. Att flytta den flyttar ingen fräsning, bara siffrorna i programmet. Markera den i vyn och dra den: den fäster vid ämnets hörn och mitt och vid ritningens origo.
forms-zero-link-hover = Visa nollpunktens egenskaper

# An imported DXF.
forms-file = Fil
forms-units = Enheter
forms-dxf-inch = tum (omräknat till mm)
forms-shapes = Former
forms-section-placement = Läge, rotation och skala
forms-update-dxf = Uppdatera från en DXF eller SVG…
forms-update-dxf-hover = En ny version av ritningen: operationerna behåller sina lager
# A button (verb).
forms-remove = Ta bort
forms-remove-dxf-hover = Ta bort denna ritningsfil och dess former

# A picture traced into shapes.
forms-picture = Bild
forms-picture-pixels = { $width } × { $height } pixlar
forms-traced-as = Spårad som
forms-traced-outlines = konturer
forms-traced-centre-lines = mittlinjer
forms-position = Läge
forms-picture-reset-position = Ramens hörn i X0 Y0
forms-rotation = Rotation
forms-picture-rotation-hover = Grader moturs, valfri vinkel, kring bildens centrum. Bilden spåras som den är, sedan vrids dess former.
forms-turn-ccw-hover = 90° moturs
forms-turn-cw-hover = 90° medurs
forms-picture-reset-rotation = Som bilden är (0°)
forms-width = Bredd
forms-picture-width-hover = Bildens bredd innan den vrids; höjden följer med
forms-picture-height = höjd { $height }
forms-layer = Lager
forms-picture-layer-hover = Lagret som dess former ligger på, för operationer som markerar det
forms-trace-settings = Spårningsinställningar…
forms-trace-settings-hover = Tröskel, utjämning, konturer eller mittlinjer, med förhandsvisning
forms-remove-picture-hover = Ta bort bilden och dess former

# A text written in a built-in font.
forms-text = Text
forms-text-hover = Vad den säger; Enter börjar en ny rad
forms-font = Typsnitt
forms-font-hover = Ett inbyggt typsnitt: vanligt, skrivstil eller dekorativt (Användarhandbok: Text)
forms-height = Höjd
forms-text-height-hover = Höjden på en versal
forms-anchor = Ankare
forms-anchor-hover = Vad Läge avser: nedre vänstra hörnet av textens ram, eller dess centrum. En centrerad text förblir centrerad när orden, typsnittet eller storleken ändras
forms-anchor-corner = nedre vänstra hörnet
forms-anchor-centre = centrum
forms-text-reset-position = I X0 Y0
forms-text-rotation-hover = Grader moturs, valfri vinkel, kring textens centrum
forms-text-reset-rotation = Vågrät (0°)
forms-align = Justering
forms-align-hover = Hur raderna ställs upp mot varandra
forms-align-left = vänster
forms-align-centre = centrerad
forms-align-right = höger
forms-letter-spacing = Teckenavstånd
forms-letter-spacing-hover = Extra utrymme mellan bokstäverna; 0: typsnittets eget
forms-line-spacing = Radavstånd
forms-line-spacing-hover = Från en rads baslinje till nästa, i versalhöjder
# After the number: "1.2 × height".
forms-line-spacing-suffix = × höjd
forms-text-layer-hover = Lagret som dess bokstäver ligger på, för operationer som markerar det
forms-size = Storlek
forms-remove-text-hover = Ta bort texten och dess bokstäver

# The job settings. "Stock": the material the parts are cut from.
forms-job-settings = Jobbinställningar
forms-section-stock = Ämne
forms-stock-size = Storlek
forms-thickness = Tjocklek
forms-material = Material
forms-material-hover = Dina egna matningar per verktyg, från Den här datorn > Material. Att välja ett ger varje operation, även de du lägger till senare, det här materialet och dess värden. Ämnet förblir som det är
forms-material-none = Inget
forms-material-not-in-library = { $name } (finns inte i ditt bibliotek)
forms-material-from = Från { $name }
forms-material-from-hover = Matningarna, skärdjupet och sidosteget är materialets värden för det här verktyget
forms-material-changed = Ändrat från { $name }
forms-material-use = Använd { $name }s
forms-material-use-hover = Fräs med materialets värden för det här verktyget
forms-material-save = Spara i { $name }
forms-material-save-hover = Behåll de här matningarna, skärdjupet och sidosteget som materialets värden för det här verktyget, till nästa jobb
forms-material-not-in = { $name } har inga värden för det här verktyget
forms-material-no-values = { $name } (inga värden för det här verktyget)
forms-op-material-hover = Dina egna matningar per verktyg, från Den här datorn > Material: operationen får det här materialets värden för sitt verktyg, och följer dem när du byter verktyg
forms-max-overcut = Max djup under ämnet
forms-max-overcut-hover = Hur långt en fräsning får gå under ämnet (ned i offerskivan). Fräsningar inom gränsen ger en varning; djupare vägras.
forms-stock-position = Ämnets läge
forms-stock-corner-at-origin = hörn i ritningens origo
forms-stock-centred-on-origin = centrerat på ritningens origo
forms-stock-centred-on-drawing = centrerat på ritningen
forms-stock-margin = marginal
forms-stock-fits-hover = Ritningens kontur plus marginalen på varje sida, centrerat på ritningen; följer ritningen
forms-stock-fit = Kontur + marginal
forms-stock-fit-hover = Centrera ämnet på ritningens kontur och gör det lika stort som konturen plus en marginal, och låt det följa den
forms-stock-fixed-size = Fast storlek, centrerad
forms-stock-fixed-size-hover = En storlek du skriver in, för en skiva du vill använda, centrerad på ritningens kontur
forms-drawing-size-note = Ritning { $size }
forms-zero-summary = X0 Y0 { $xy } · Z0 { $z }
forms-material-each-op = Varje operation kan välja sitt eget, under sitt verktyg.
forms-drawing-size-hover = Storleken på ritningens kontur (det som fräses; utan hjälpgeometri). Skärets kontur, med verktyget, står i rapporten efter Beräkna
forms-z-zero = Z-noll
forms-z-zero-top = ämnets ovansida
forms-z-zero-bottom = ämnets undersida
forms-machine-summary = { $post }, säker Z { $safe_z }, { $spindle }
forms-section-machine = Maskin
forms-from-machine = Från maskin
forms-edit-here = Redigera här
forms-edit-here-hover = Ändra maskinens värden bara för detta projekt; den förblir länkad och visar vad som skiljer
forms-safe-z-job = Säker Z (detta jobb)
forms-safe-z-hover = Höjden för snabbförflyttningar i sidled, ovanför ämnet och dess fästen

# The project's tools.
forms-tools = Verktyg
forms-add-from-library = Lägg till från bibliotek…
forms-new-tool = Nytt verktyg
forms-tool-not-used = används inte
# $operations: a list of operation numbers, "1, 3".
forms-tool-used-by = operationer { $operations }
forms-tool-not-used-long = Används inte av någon operation
forms-tool-cuts = Fräser operationer { $operations }
forms-remove-tool = Ta bort verktyg
forms-remove-tool-disabled = Används av operationer { $operations }; ge dem ett annat verktyg först
# An icon button that puts a value back.
forms-reset = Återställ

# A DXF's position, rotation and scale.
forms-dxf-reset-position = Tillbaka dit filen har den (vridning och skala kvarstår)
forms-bottom-left = Nere till vänster
forms-bottom-left-hover = Ritningens hörn { $margin } in från ämnets nedre vänstra hörn
# A button: centre the drawing on the stock.
forms-centre = Centrera
forms-centre-hover = Ritningens centrum på ämnets centrum
forms-dxf-rotation-hover = Grader moturs från filen som den ritats, valfri vinkel, kring ritningens centrum
forms-dxf-reset-rotation = Tillbaka till filens egen vinkel (0°)
forms-scale = Skala
forms-scale-hover = Från filen som den ritats, kring ritningens centrum. Verktygsstorlekar, djup och bryggor skalas inte.
forms-dxf-reset-scale = Tillbaka till filens egen storlek (× 1)

# A part: a group of operations cut one after the other.
forms-part-number = Detalj { $number }
# Label of the on/off checkbox (verb): whether the part is cut.
forms-part-cut = Fräs
forms-part-cut-hover = Av: detaljen och dess operationer finns kvar i projektet men fräses inte
forms-part-cut-check = denna detalj och dess operationer
forms-active = Aktiv
forms-active-hover = Nya operationer hamnar i den aktiva detaljen. Högerklicka på en detalj för att aktivera den
forms-active-yes = ja: nya operationer hamnar här
forms-activate = Aktivera
forms-operations = Operationer
forms-operations-none = inga än
forms-copies-one = en
forms-copies = Kopior
forms-parts-note = Detaljerna fräses en i taget, var och en med sina kopior.
forms-copies-hover = Fräs detaljen flera gånger i ett rutnät. Kopiorna ritas tonade; bara själva ritningen kan markeras.
forms-copies-grid = flera, i ett rutnät
forms-columns-rows = kolumner × rader
# Copy to copy distance, in X and Y.
forms-pitch = delning X Y
forms-gap = mellanrum
forms-gap-hover = Sätter delningen en gång så att så här mycket blir kvar mellan spåren hos intilliggande kopior. Projektet behåller delningen.
forms-set-pitch = Sätt delning
forms-order = ordning

# The machine. "Post": the post-processor that writes the G-code.
forms-post = Post
forms-post-yours = { $name } (din)
forms-post-folder-hover = Dina egna postprocessorer läggs i { $folder }
forms-post-builtin-hover = De inbyggda postprocessorerna
forms-gcode-units = G-kodsenheter
forms-gcode-units-hover = Enheterna G-koden skrivs i: mm (G21) eller tum (G20), som din styrning är inställd. Allt annat i projektet förblir i mm
forms-gcode-mm = mm (G21)
forms-gcode-inch = tum (G20)
forms-spindle = Spindel
forms-spindle-cw = medurs (M3)
forms-spindle-ccw = moturs (M4)
forms-max-feed = Max matning
forms-tool-changes = Verktygsbyten
forms-tool-changes-hover = Med flera verktyg: en G-kodfil per verktyg (byt verktyg och nollställ Z mellan filerna), eller en fil med M6-verktygsbyten som ditt sändarprogram hanterar (GRBL själv vägrar M6)
forms-tool-change-files = en fil per verktyg
forms-tool-change-program = en fil med M6 (ditt sändarprogram hanterar bytet)
forms-travel = Slaglängd X Y Z
forms-max-feed-not-set = inte angiven
forms-travel-not-checked = kontrolleras inte
forms-revert-hover = "{ $name }" har { $value }: klicka för att ta det
forms-post-yours-copied = Din, kopierad till projektet
forms-post-changed = Din postprocessorfil har ändrats
forms-post-update = Uppdatera
forms-post-update-hover = Kopiera postprocessorn som den är nu till projektet
forms-post-file-missing = Din; projektet behåller sin kopia (filen finns inte)

# A tool's values.
forms-tool-type = Typ
forms-diameter = Diameter
forms-spindle-speed = Spindelvarvtal
forms-centre-cutting = Centrumskärande
forms-yes = ja
forms-no = nej
forms-unknown = okänt
forms-angle = Vinkel
forms-angle-hover = V:ets toppvinkel; V-gravyr behöver den
forms-tip = Spets
forms-tip-hover = Diametern på den plana spetsen; ej angiven räknas som spetsig
forms-pointed = spetsig

# A rest pass of a pocket: a smaller tool clears what the one before left.
forms-rest-pass-of = operation { $number }
forms-rest-pass-of-named = operation { $number } ({ $name })
forms-rest-pass-facts = { $start }–{ $end } { $unit }, förskjutning { $clearance } { $unit }, { $direction }
forms-rest-pass = Restbearbetning
forms-rest-of = Rest av
forms-rest-of-hover = Den fräser fickans former och djup, och tar bara bort det som verktyget före lämnar
# A button: show the pocket.
forms-open = Öppna
forms-from-pocket = Från fickan
forms-after-tool = Efter verktyg
forms-after-tool-hover = Den tar bort det som detta verktyg lämnar: fickans, eller föregående restbearbetnings
forms-cut-it = Fräs den
forms-cut-it-hover = Av: behålls men fräses inte
forms-rest-pass-note = Fräses efter fickorna intill sin ficka som använder samma verktyg, alla deras restbearbetningar tillsammans: det lilla verktyget sätts i en gång.
forms-remove-rest-pass = Ta bort restbearbetning
forms-floor-pass = Bottenpass
forms-floor-of = Botten av
forms-floor-of-hover = Den fräser V-gravyrens former till dess slutdjup, på avstånd från de sneda väggarna
forms-from-vcarve = Från V-gravyren
forms-floor-pass-facts = { $start }–{ $end } { $unit }, { $inset } { $unit } innanför konturen
forms-floor-pass-note = Fräses före V-gravyrerna intill som använder samma V-fräs, alla deras bottenpass tillsammans. Där pinnfräsen inte når rensar V-fräsen efter längs mittlinjen.
forms-remove-floor-pass = Ta bort bottenpass
forms-floor-hint = Lämnar en del av botten: en på högst { $largest } skulle rensa hela.
forms-floor-hint-library = I ditt bibliotek: { $tools }

# Font styles, headings in the font list.
forms-font-plain = Vanliga
forms-font-script = Skrivstil
forms-font-display = Dekorativa
forms-explode-dxf = Bryt upp…
forms-explode-dxf-hover = Gör om denna DXF:s former till ritade former som du kan ändra; kopplingen till DXF-filen försvinner
