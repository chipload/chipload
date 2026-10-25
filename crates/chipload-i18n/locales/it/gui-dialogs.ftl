# File dialogs, typed-path prompts, the About window and the window title
# (crates/chipload-gui/src/app/dialogs.rs).

# Help > User guide, until the guide is published online. Keep the file names as they are.
dialogs-user-guide = La guida utente è inclusa in ogni release come chipload-guide-<version>.zip (apri index.html). Dai sorgenti: packaging/docs/build-book.sh.

# Names of file types in the system's file dialogs.
dialogs-filter-projects = Progetti Chipload
dialogs-filter-dxf = Disegni DXF

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
dialogs-path-open = Un progetto .chipload o un .dxf:
dialogs-path-save-title = Salva progetto
dialogs-path-save = Salva il progetto in:
dialogs-path-export-title = Esporta G-code
dialogs-path-export = Esporta il G-code verificato in:
dialogs-path-update-title = Aggiorna disegno
dialogs-path-update = Il DXF:
dialogs-path-image-title = Importa immagine
dialogs-path-image = Un'immagine (PNG, JPEG o BMP):
dialogs-path-add-dxf-title = Aggiungi DXF
dialogs-path-add-dxf = Il DXF da aggiungere al progetto:
dialogs-path-update-dxf-title = Aggiorna DXF
dialogs-path-update-dxf = La nuova versione del DXF:

# In the window's title bar, for a project never saved.
dialogs-title-new-project = nuovo progetto

## Explode a DXF, join close ends (docs/specs/explode-and-join.md §5)

# $name is the DXF's name in the Browser.
dialogs-explode-title = Esplodi { $name } in forme
dialogs-explode-body = Le sue forme diventano forme disegnate che puoi modificare, spostare ed eliminare. Le lavorazioni continuano a lavorare le stesse forme. Il progetto non conosce più il file DXF: non sarà possibile aggiornarlo dal DXF.
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
