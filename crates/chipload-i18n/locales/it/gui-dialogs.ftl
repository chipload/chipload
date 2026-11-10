# File dialogs, typed-path prompts, the About window and the window title
# (crates/chipload-gui/src/app/dialogs.rs).

# Help > User guide, until the guide is published online. Keep the file names as they are.
dialogs-user-guide = La guida utente è inclusa in ogni release come chipload-guide-<version>.zip (apri index.html). Dai sorgenti: packaging/docs/build-book.sh.

# Names of file types in the system's file dialogs.
dialogs-filter-projects = Progetti Chipload
dialogs-filter-dxf = Disegni (DXF, SVG)

dialogs-selector-copied = Selettore copiato negli appunti

dialogs-about-title = Informazioni su Chipload
dialogs-about-appimage = L'AppImage per Linux contiene anche libxkbcommon e libxcb-xkb (licenze di tipo MIT); i loro file di copyright sono in usr/share/doc/chipload/bundled dentro l'AppImage.
dialogs-third-party = Licenze di terze parti

# Asked when a DXF doesn't say whether it was drawn in millimetres or inches.
dialogs-units-title = Unità del disegno
dialogs-units-unknown = { $file } non indica le unità che usa.
dialogs-units-question = In quali unità è stato disegnato?
# A button: the drawing was drawn in inches.
dialogs-inch = pollici
dialogs-units-cancelled = Disegno non importato: le sue unità sono sconosciute.
dialogs-px-title = Pixel per pollice
dialogs-px-unknown = { $file } indica la sua dimensione solo in pixel.
dialogs-px-question = Quanti pixel fanno un pollice?
dialogs-px-96 = 96 (Inkscape, browser)
dialogs-px-72 = 72 (vecchi Illustrator)
dialogs-px-other = Altro:
dialogs-px-use = Usa
dialogs-px-cancelled = Disegno non importato: la sua dimensione è sconosciuta.
dialogs-export-dxf-title = Esporta disegno come DXF
dialogs-export-dxf-inch = In pollici (altrimenti mm)
dialogs-export-dxf-hidden = Anche layer e gruppi nascosti
dialogs-export-dxf-construction = Anche la geometria di costruzione (su layer propri)
dialogs-export-dxf-selection = Solo le forme selezionate
dialogs-export-dxf-save = Esporta…
dialogs-export-dxf-path = Esporta in un percorso…
dialogs-path-export-dxf = Il DXF da scrivere:
dialogs-filter-dxf-file = Disegno DXF

dialogs-unsaved-title = Modifiche non salvate
dialogs-unsaved-body = Il progetto ha modifiche non salvate. Salvarle prima?
dialogs-save-hover = Salva, poi continua
dialogs-dont-save = Non salvare
dialogs-dont-save-hover = Scarta le modifiche e continua

# Another program saved the open project's file.
dialogs-changed-title = { $file } è cambiato sul disco
dialogs-changed-body = Un altro programma ha salvato il progetto. Caricare la sua versione? Le tue modifiche non salvate restano recuperabili con Ctrl+Z.
dialogs-load-it = Caricala
dialogs-load-it-hover = Mostra la versione del file; Ctrl+Z riporta la tua
dialogs-keep-mine = Tieni la mia
dialogs-keep-mine-hover = Tieni le tue modifiche; il prossimo Salva sovrascrive il file

# Windows where a file's path is typed in: a title and what to type.
dialogs-path-open-title = Apri percorso
dialogs-path-open = Un progetto .chipload, o un .dxf o .svg:
dialogs-path-save-title = Salva progetto
dialogs-path-save = Salva il progetto in:
dialogs-path-export-title = Esporta G-code
dialogs-path-export = Esporta il G-code verificato in:
dialogs-path-update-title = Aggiorna disegno
dialogs-path-update = Il DXF o SVG:
dialogs-path-image-title = Importa immagine
dialogs-path-image = Un'immagine (PNG, JPEG o BMP):
dialogs-path-add-dxf-title = Aggiungi DXF o SVG
dialogs-path-add-dxf = Il DXF o SVG da aggiungere al progetto:
dialogs-path-update-dxf-title = Aggiorna file di disegno
dialogs-path-update-dxf = La nuova versione del DXF o SVG:

# In the window's title bar, for a project never saved.
dialogs-title-new-project = nuovo progetto

## Explode a DXF, join close ends (docs/specs/explode-and-join.md §5)

# $name is the DXF's name in the Browser.
dialogs-explode-title = Esplodi { $name } in forme
dialogs-explode-body = Le sue forme diventano forme disegnate che puoi modificare, spostare ed eliminare. Le lavorazioni continuano a lavorare le stesse forme. Il progetto non conosce più il file DXF: non sarà possibile aggiornarlo dal DXF.
dialogs-explode-image-body = Le sue forme diventano forme disegnate che puoi modificare, spostare ed eliminare. Le lavorazioni continuano a lavorare le stesse forme. L'immagine stessa esce dal progetto: non potrà essere vettorizzata di nuovo.
dialogs-explode-text-body = Le sue lettere diventano forme disegnate che puoi modificare, spostare ed eliminare. Le lavorazioni continuano a lavorare le stesse forme. Le parole non potranno più essere modificate.
dialogs-explode-join = Unisci estremi più vicini di
dialogs-explode = Esplodi
dialogs-join-title = Unisci estremi
dialogs-join-body = Le polilinee aperte tra { $count ->
    [one] la forma selezionata
   *[other] le { $count } forme selezionate
} i cui estremi sono più vicini della distanza diventano una sola, sullo stesso layer. Una catena che torna al suo inizio viene chiusa.
dialogs-join-body-all = Le polilinee disegnate aperte i cui estremi sono più vicini della distanza diventano una sola, sullo stesso layer. Una catena che torna al suo inizio viene chiusa.
dialogs-join-gap = Unisci estremi più vicini di
dialogs-join = Unisci

# V-carve inlays (docs/specs/vcarve-inlay.md §4)
inlay-ribbon = Intarsio…
inlay-ribbon-hover = Un intarsio intagliato a V delle forme scelte: una tasca qui, e un tassello specchiato in una tavola a sé, incollato dentro
inlay-menu = Nuovo intarsio…
inlay-title = Intarsio intagliato a V
inlay-body = Le forme scelte vengono intagliate come tasca in questa tavola, e come tassello, specchiato, in un nuovo piazzamento: una tavola a sé. Il tassello si incolla a faccia in giù nella tasca e poi se ne pialla il retro.
inlay-vbit = Fresa a V
inlay-none = Nessuna
inlay-depth = Profondità della tasca
inlay-depth-hover = Quanto è profonda la tasca nella tavola di base
inlay-plug-start = Profondità iniziale del tassello
inlay-plug-start-hover = A che profondità nella sua tavola si prende il contorno del tassello: la sua faccia sta così in fondo nella tasca, e il resto della tasca resta per la colla
inlay-gap = Spazio per la colla
inlay-gap-hover = Quanto più a fondo si intaglia la tavola del tassello, perché il retro sporga tanto sopra la base e la colla che esce abbia spazio
inlay-floor = Fresa cilindrica del fondo
inlay-floor-hover = Una fresa cilindrica per le passate di fondo di entrambi. Il canale del tassello è quasi tutto fondo
inlay-thickness = Spessore della tavola del tassello
inlay-thickness-hover = La tavola da cui si ricava il tassello. Ciò che resta sotto il canale lo tiene insieme finché non viene piallato
inlay-make = Crea intarsio
inlay-no-vbit = Scegli una fresa a V: non ne ha né il progetto né la tua libreria utensili
inlay-made = Intarsio creato: l’operazione { $pocket } è la tasca, l’operazione { $plug } il tassello, nel piazzamento { $setup }, il suo pezzo specchiato da sinistra a destra.
inlay-refused = Impossibile creare l’intarsio.
sign-title = Nuova insegna
sign-body = Le parole, una cornice attorno che le segue e ogni operazione che taglia l'insegna, in un pezzo a sé. Tutto si può cambiare dopo.
sign-words = Parole
sign-words-hint = Cosa dice l'insegna; Invio inizia una nuova riga
sign-around = Attorno alla selezione
sign-around-body = { $count ->
    [one] L'insegna va attorno alla forma selezionata, scolpita con la fresa a V. La cornice la segue, anche un'immagine ricalcata.
   *[other] L'insegna va attorno alle { $count } forme selezionate, scolpite con la fresa a V. La cornice le segue, anche un'immagine ricalcata.
}
sign-around-words = Scrivi invece delle parole
sign-around-words-hover = Fai l'insegna attorno a parole nuove, come senza selezione
sign-font = Carattere
sign-height = Altezza
sign-style = Stile
sign-style-hover = Incisa: le lettere scolpite a V. In rilievo: lo sfondo dentro il bordo più interno ribassato attorno alle lettere. Intarsio: le lettere intarsiate da una seconda tavola.
sign-style-engraved = Incisa
sign-style-raised = In rilievo
sign-style-inlay = Intarsio
sign-relief = Profondità dello sfondo
sign-relief-hover = Quanto in profondità viene ribassato lo sfondo attorno alle lettere in rilievo, con la fresa cilindrica
sign-edge = Bordo
sign-edge-chamfer = Smusso
sign-edge-hover = Uno smusso di questa larghezza attorno al contorno, tagliato con la fresa a V prima di ritagliare l'insegna
sign-vbit = Fresa a V
sign-end-mill = Fresa cilindrica
wizard-library-tool-hover = Dalla tua libreria utensili: copiato nel progetto quando confermi, non prima
wizard-library-tool-gone = Un utensile scelto dalla tua libreria non c'è più: scegline un altro
sign-board = Tavola
sign-fit-board = Adatta attorno all'insegna
sign-fit-board-hover = Il grezzo adattato attorno al disegno con questo margine, di questo spessore. Spento: il grezzo resta com'è.
sign-thickness = Spessore
sign-board-margin = Margine
sign-adds = Aggiunge: { $operations }
sign-make = Crea insegna
sign-no-tools = Scegli una fresa a V e una fresa cilindrica, del progetto o della tua libreria utensili.
sign-made = Insegna creata: { $count } operazioni in un pezzo a sé. Cambia le parole e la cornice le segue.
sign-refused = Impossibile creare l'insegna.
inlay-mark-gap = retro, { $gap } sopra
inlay-mark-surface = superficie
inlay-mark-face = faccia del tassello, { $depth }
inlay-mark-pocket = tasca, { $depth }
