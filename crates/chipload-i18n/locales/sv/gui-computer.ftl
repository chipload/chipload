# "This computer" in the Browser: Preferences and post-processors (crates/chipload-gui/src/app/computer.rs).

computer-posts = Postprocessorer
computer-posts-hover = Det som gör om verktygsbanor till G-kod för din maskin: inbyggda och dina
# Notes at the right end of a post's row in the Browser.
computer-post-built-in = inbyggd
computer-post-yours = din
computer-post-this-project = detta projekt

computer-preferences = Inställningar
computer-preferences-about = För alla projekt på den här datorn.
computer-settings-folder = Inställningsmapp
# $var is the name of an environment variable.
computer-settings-folder-none = ingen: sätt { $var } för att spara inställningar och ett bibliotek
computer-setup-sheet = Uppsättningsblad
computer-setup-sheet-hover = En HTML-sida för operatören bredvid de exporterade filerna: ämne, nollpunkt, verktyg i ordning, operationer, tider och varningar
computer-setup-sheet-write = skriv ett med G-koden
computer-toolpath = Verktygsbana
computer-toolpath-auto = uppdatera automatiskt (även i menyn Visa)
computer-toolpath-auto-hover = Beräkna av sig själv en halv sekund efter varje ändring
computer-units = Enheter
computer-units-hover = I vilka enheter längder och matningar visas och skrivs in. Projekt förblir i mm; G-kodens enheter är maskinens (Maskin)
computer-theme = Färgtema
computer-theme-system = System
computer-theme-light = Ljust
computer-theme-dark = Mörkt

computer-posts-count = { $builtin } inbyggda, { $yours } egna.
computer-posts-folder = Dina egna läggs i { $folder }
computer-posts-no-folder = Ingen inställningsmapp, så bara de inbyggda.
# NAME.toml is a file name: keep it as it is.
computer-posts-about = En egen postprocessor är två filer: NAME.toml med dess inställningar, och mallen den anger. Utgå från en kopia av en inbyggd; handbokens sida Postprocessorer förklarar båda.
computer-posts-reload = Läs mappen igen
computer-posts-reload-hover = Efter att du lagt till eller ändrat en postprocessors filer

computer-post = Postprocessor
computer-post-gone = Denna postprocessor finns inte längre.
computer-post-unusable = Kan inte användas: { $error }
computer-post-use = Använd i detta projekt
computer-post-use-hover = Gör den till projektets postprocessor
computer-post-use-copy-hover = Gör den till projektets postprocessor: en kopia läggs i projektet
computer-post-in-use = Projektet använder den
computer-post-no-project = Öppna ett projekt först
