# Drawing shapes with the mouse: the status bar while drawing, and snapping
# (crates/chipload-gui/src/draw.rs). Esc, Enter and Backspace are keyboard keys; the
# grid is the step clicks are rounded to, e.g. "1 mm".

draw-status-circle-start = Ritar en cirkel: klicka på centrum (rutnät { $grid }, Esc avbryter)
draw-status-circle = Ritar en cirkel med radie { $radius }: klicka på en punkt på cirkeln (rutnät { $grid }, Esc avbryter)
draw-status-rectangle-start = Ritar en rektangel: klicka på ett hörn (rutnät { $grid }, Esc avbryter)
# The rectangle's size so far: along X, then along Y.
draw-status-rectangle = Ritar en rektangel { $length } x { $width }: klicka på motstående hörn (rutnät { $grid }, Esc avbryter)
draw-status-polyline-start = Ritar en polylinje { $count ->
        [one] { $count } punkt
       *[other] { $count } punkter
    }: klicka på första punkten (rutnät { $grid }, Esc avbryter)
draw-status-polyline = Ritar en polylinje { $count ->
        [one] { $count } punkt
       *[other] { $count } punkter
    }: klicka på nästa punkt; Enter eller dubbelklick avslutar den öppen, första punkten sluter den, Backspace tar bort senaste punkten (rutnät { $grid }, Esc avbryter)

# Why a click did nothing.
draw-no-size-circle = cirkeln skulle sakna storlek
draw-no-size-rectangle = rektangeln skulle sakna storlek
draw-polyline-too-short = en polylinje behöver minst 2 punkter

# The point the cursor snapped to, on contour #4.
draw-snap-end = ände av #{ $contour }
draw-snap-centre = centrum av #{ $contour }
draw-snap-middle = mitten av #{ $contour }
draw-snap-intersection = skärning mellan #{ $contour } och #{ $other }
draw-snap-intersection-self = skärning på #{ $contour }
draw-snap-quadrant = kvadrantpunkt på #{ $contour }
draw-snap-perpendicular = vinkelrätt mot #{ $contour }
draw-snap-tangent = tangent till #{ $contour }
draw-snap-nearest = på #{ $contour }

# A value typed while drawing that can't be used.
draw-entry-unreadable = kan inte läsas: skriv x,y  @dx,dy  en längd, eller längd<vinkel
draw-entry-needs-point = första punkten: skriv x,y
draw-entry-no-direction = flytta markören från den senaste punkten för att ange riktningen
draw-status-arc-start = Ritar en båge: klicka där den börjar (rutnät { $grid }, Esc avbryter)
draw-status-arc-end = Ritar en båge, korda { $length }: klicka där den slutar (rutnät { $grid }, Esc avbryter)
draw-status-arc = Ritar en båge med radie { $radius }: klicka en punkt den går genom, eller skriv radien (rutnät { $grid }, Esc avbryter)
draw-status-slot-start = Ritar ett spår: klicka mitten av ena änden (rutnät { $grid }, Esc avbryter)
draw-status-slot-end = Ritar ett spår, { $length } mellan ändarna: klicka mitten av andra änden (rutnät { $grid }, Esc avbryter)
draw-status-slot = Ritar ett spår med bredd { $width }: klicka för att ange bredden, eller skriv den (rutnät { $grid }, Esc avbryter)
draw-arc-on-a-line = de tre punkterna ligger på en linje: använd polylinjeverktyget
draw-no-size-slot = spåret skulle sakna längd eller bredd
draw-entry-radius-too-small = radien måste vara minst halva avståndet mellan ändarna
draw-entry-no-side = flytta markören åt det håll bågen ska bukta
draw-status-polygon-start = Ritar en polygon: klicka centrum (rutnät { $grid }, Esc avbryter)
draw-status-polygon = Ritar en polygon med radie { $radius }: klicka ett hörn (rutnät { $grid }, Esc avbryter)
modify-offset-pick = Förskjut: markera formerna (klicka på en; Ctrl lägger till), Esc avslutar
modify-offset-status = Förskjutning { $distance } { $side }: klicka, eller skriv avståndet (Alt: utanför rutnätet, Esc avslutar)
modify-offset-done = { $count ->
    [one] La till en förskjutning
   *[other] La till { $count } förskjutningar
} på { $distance }.
modify-side-outside = utåt
modify-side-inside = inåt
modify-side-left = åt vänster
modify-side-right = åt höger
modify-corners-round = Runda hörn
modify-corners-sharp = Skarpa hörn
modify-corners-hover = Runda: den exakta förskjutningen, bågar runt hörnen. Skarpa: sidorna förlängda tills de möts, som en ram.
modify-offset-count = Antal
modify-corner-pick = Markera en ritad form (klicka på den), klicka sedan dess hörn; Esc avslutar
modify-fillet-status = Avrunda: klicka ett hörn; Skift+klick rundar alla hörn (Esc avslutar)
modify-chamfer-status = Fasa: klicka ett hörn; Skift+klick fasar alla hörn (Esc avslutar)
modify-fillet-radius = Radie
modify-chamfer-distance = Avstånd
modify-trim-status = Trimma: klicka den bit av en ritad form som ska bort; den visas innan du klickar (Esc avslutar)
modify-extend-status = Förläng: klicka nära änden av en öppen ritad form för att förlänga den till nästa form (Esc avslutar)
modify-break-status = Bryt: klicka en ritad form där den ska delas (Esc avslutar)
modify-extend-closed = en sluten form har ingen ände att förlänga
modify-combine-pick = Välj först två eller fler slutna ritade former
modify-subtract-status = Subtrahera: klicka på formen som de valda ska skäras ut ur (Esc avslutar)
modify-array-pick = Välj först de ritade former som ska kopieras
modify-array-status = Matris: ställ in kopiorna under Markering, sedan Skapa (Esc avslutar)
modify-polar-status = Polär matris: klicka på mittpunkten, ställ in kopiorna under Markering, sedan Skapa (Esc avslutar)
array-columns = Kolumner
array-rows = Rader
array-dx = Kolumnavstånd
array-dy = Radavstånd
array-count = Antal
array-center = Mittpunkt
array-angle = Vinkel
array-angle-hover = 360° fördelar kopiorna jämnt runt om; mindre lägger den första och sista vid dess ändar; negativt går medurs
array-rotate = Vrid kopiorna
array-create = Skapa
