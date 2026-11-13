# 3D models in the window: File > Add 3D model…, their rows in the Browser and their
# properties (crates/chipload-gui/src/app/models.rs, sources.rs, forms/sources.rs).

models-stl-filter = STL-modeller
models-cannot-read = Kan inte läsa { $path }: { $error }
models-reading = Läser { $file }…
models-cannot-add = Kan inte lägga till { $path }:
models-cannot-place = Modellen kunde inte placeras så:
models-removed = Tog bort modell { $id }
dialogs-path-model-title = Lägg till 3D-modell
dialogs-path-model = En STL-fil (binär eller text):
menu-add-model = Lägg till 3D-modell…
menu-add-model-hover = En STL-fil, fräst ner i skivan uppifrån som en relief
menu-add-model-path = Lägg till 3D-modell från sökväg…
sources-model-title = { $name } (modell { $id })
sources-add-model = Lägg till 3D-modell…
sources-add-model-hover = En STL-fil, fräst ner i skivan uppifrån som en relief
sources-model-hover = En 3D-modell: dess kontur på skivan. Dubbelklicka för läge, storlek och djup
sources-model-new-relief = Ny relief
sources-model-new-relief-hover = Fräs den här modellen: en Relief-operation inom dess kontur, i den aktiva delen
sources-model-properties = Egenskaper
sources-remove-model-hover = Ta bort modellen och dess kontur; dess reliefer finns kvar, var och en med ett problem tills den får en annan modell
forms-model-file = Fil
forms-model-size = På skivan
forms-model-size-hover = Bredd × höjd × djup för den placerade modellen
forms-model-position-hover = Nedre vänstra hörnet av modellens kontur, innan den vrids
forms-model-reset-position = Tillbaka till ritningens origo
forms-model-rotation-hover = Vriden moturs kring konturens mitt
forms-model-reset-rotation = Inte vriden
forms-model-width-hover = Modellens bredd på skivan; höjden följer
forms-model-depth = Djup
forms-model-depth-hover = Hur djupt reliefen går, från modellens topp till dess lägsta punkt. Reliefer är oftast flatare än modellen
forms-model-proportional = Behåll proportioner
forms-model-proportional-hover = Djupet följer bredden, i modellens egna proportioner
forms-model-up = Upp
forms-model-up-hover = Vilken av modellens axlar som pekar upp från skivan. Många filer är modellerade med Y uppåt
forms-model-layer-hover = Lagret som modellens kontur ritas på
forms-model-new-relief = Ny relief
forms-model-new-relief-hover = Fräs den här modellen: en Relief-operation inom dess kontur, i den aktiva delen
forms-remove-model-hover = Ta bort modellen och dess kontur; dess reliefer finns kvar, var och en med ett problem tills den får en annan modell

# Reliefs and their roughing pass (docs/specs/stl-relief.md §9, §14).
models-could-not-add-roughing = Grovpasset kunde inte läggas till: { $error }
models-no-roughing-tool = Ingen pinnfräs eller vanlig kulfräs i projektet eller ditt bibliotek
