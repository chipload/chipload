# Glossary for translators

The words the app uses for machining, fixed before the texts are translated
(docs/specs/translations.md decision 7), so a term reads the same in every window. Taken
from the trade's own use in each language: tooling makers' catalogues, controller manuals
and machining textbooks. Never from another CAM program's translation.

**Status: draft (2026-10-24), not yet reviewed by native speakers who machine.** A
correction here is a correction everywhere: change the term, then every text using it.

## Style

| | Address | Buttons and menu items | Notes |
|---|---|---|---|
| de | *Sie* | infinitive: *Speichern*, *Werkzeugweg berechnen* | nouns capitalised, as always |
| fr | *vous* | infinitive: *Enregistrer* | space before `: ; ! ?` (non-breaking where possible) |
| es | *tú* | infinitive: *Guardar* | |
| it | *tu* | imperative: *Salva*, *Apri* | |
| sv | *du* | imperative: *Spara*, *Beräkna verktygsbana* | |

- Keep it short: labels sit left of fields in a narrow panel. Prefer the shorter of two
  correct words.
- Units, G-code words (G0, G2, M3), file extensions and the names GRBL, LinuxCNC, Mach3 and
  Chipload are never translated.
- Numbers keep the decimal point (decision 6).

## Terms

| English | Deutsch | Français | Español | Italiano | Svenska |
|---|---|---|---|---|---|
| project | Projekt | projet | proyecto | progetto | projekt |
| part (a group of operations) | Teil | pièce | pieza | pezzo | detalj |
| operation | Bearbeitung | opération | operación | lavorazione | operation |
| drawing | Zeichnung | dessin | dibujo | disegno | ritning |
| layer | Layer | calque | capa | layer | lager |
| shape | Form | forme | forma | forma | form |
| contour | Kontur | contour | contorno | contorno | kontur |
| profile (operation) | Konturfräsen | contournage | contorneado | contornatura | konturfräsning |
| pocket | Tasche | poche | cajera | tasca | ficka |
| island (in a pocket) | Insel | îlot | isla | isola | ö |
| V-carve | V-Gravur | gravure en V | tallado en V | incisione a V | V-gravyr |
| follow path | Pfad folgen | suivi de tracé | seguir trayectoria | segui percorso | följ bana |
| holes | Bohrungen | perçages | taladros | fori | hål |
| drill (cycle) | bohren | percer | taladrar | forare | borra |
| bore (helical) | Helixbohren | alésage hélicoïdal | taladrado helicoidal | foratura elicoidale | helixborrning |
| surfacing | Planfräsen | surfaçage | planeado | spianatura | planfräsning |
| spoilboard | Opferplatte | martyr | tablero de sacrificio | piano di sacrificio | offerskiva |
| stock | Rohteil | brut | material en bruto | grezzo | ämne |
| work zero | Werkstücknullpunkt | origine pièce | cero pieza | zero pezzo | arbetsnollpunkt |
| safe height | Sicherheitshöhe | hauteur de sécurité | altura de seguridad | quota di sicurezza | säker höjd |
| tool | Werkzeug | outil | herramienta | utensile | verktyg |
| end mill | Schaftfräser | fraise | fresa | fresa | pinnfräs |
| V-bit | V-Fräser | fraise à graver en V | fresa en V | fresa a V | V-fräs |
| tool library | Werkzeugbibliothek | bibliothèque d'outils | biblioteca de herramientas | libreria utensili | verktygsbibliotek |
| machine | Maschine | machine | máquina | macchina | maskin |
| spindle | Spindel | broche | husillo | mandrino | spindel |
| spindle speed (rpm) | Drehzahl | vitesse de broche | velocidad del husillo | velocità mandrino | spindelvarvtal |
| feed | Vorschub | avance | avance | avanzamento | matning |
| plunge (feed) | Eintauchen | plongée | penetración | affondamento | nedmatning |
| chip load | Zahnvorschub | avance par dent | avance por diente | avanzamento per dente | matning per tand |
| flutes | Schneiden | dents | filos | taglienti | skär |
| depth per pass (step down) | Zustellung | profondeur de passe | profundidad de pasada | profondità di passata | skärdjup |
| step over | seitliche Zustellung | pas latéral | paso lateral | passo laterale | sidosteg |
| climb milling | Gleichlauf | en avalant | en concordancia | concorde | medfräsning |
| conventional milling | Gegenlauf | en opposition | en oposición | discorde | motfräsning |
| inside / outside / on the line | innen / außen / auf der Linie | intérieur / extérieur / sur la ligne | interior / exterior / sobre la línea | interno / esterno / sulla linea | inuti / utanför / på linjen |
| tabs | Haltestege | attaches | puentes | ponticelli | bryggor |
| ramp | Rampe | rampe | rampa | rampa | ramp |
| helix (entry) | Helix | hélice | hélice | elica | helix |
| finishing pass | Schlichtgang | passe de finition | pasada de acabado | passata di finitura | finskär |
| onion skin | Restschicht | peau d'oignon | piel de cebolla | strato sottile | tunt skikt |
| dogbone | Dogbone | dogbone | dogbone | dogbone | dogbone |
| T-bone | T-Bone | T-bone | T-bone | T-bone | T-bone |
| rest machining | Restbearbeitung | reprise des restes | mecanizado de restos | ripresa dei residui | restbearbetning |
| toolpath | Werkzeugweg | parcours d'outil | trayectoria | percorso utensile | verktygsbana |
| rapid (move) | Eilgang | déplacement rapide | movimiento rápido | rapido | snabbförflyttning |
| generate (the toolpath) | berechnen | calculer | calcular | calcolare | beräkna |
| G-code | G-Code | G-code | código G | G-code | G-kod |
| post-processor | Postprozessor | post-processeur | postprocesador | post-processore | postprocessor |
| setup sheet | Einrichteblatt | fiche de réglage | hoja de preparación | scheda di attrezzaggio | uppsättningsblad |
| air cut | Luftschnitt | usinage à vide | corte en vacío | taglio in aria | luftkörning |
| simulation | Simulation | simulation | simulación | simulazione | simulering |
| material left | Restmaterial | matière restante | material restante | materiale residuo | kvarvarande material |
| cycle time | Bearbeitungszeit | temps de cycle | tiempo de ciclo | tempo ciclo | cykeltid |
| problem (in the project) | Problem | problème | problema | problema | problem |
| Browser (the tree panel) | Browser | navigateur | navegador | browser | bläddrare |
| copies (grid) | Kopien | copies | copias | copie | kopior |
| traced picture | nachgezeichnetes Bild | image vectorisée | imagen vectorizada | immagine vettorizzata | spårad bild |
| centre line | Mittellinie | ligne médiane | línea central | linea mediana | mittlinje |
| job settings | Jobeinstellungen | paramètres d'usinage | ajustes del trabajo | impostazioni lavoro | jobbinställningar |
| ribbon groups (FILE, DRAW, OPERATION, LIBRARY, VIEW) | Datei, Zeichnen, Bearbeitung, Bibliothek, Ansicht | Fichier, Dessiner, Opération, Bibliothèque, Vue | Archivo, Dibujar, Operación, Biblioteca, Vista | File, Disegna, Lavorazione, Libreria, Vista | Arkiv, Rita, Operation, Bibliotek, Visa |
| oversize (cuts more, past the line) | Übermaß | surcote | holgura | maggiorazione | förskjutning |
| rest pass | Restdurchgang | passe de reprise | pasada de restos | passata di ripresa | restbearbetning |
| lead (in/out) | Anfahrt | approche | aproximación | raccordo | inkörning |
| pecks (drilling) | Entspanen | débourrages | picoteo | a passi | stegborrning |

**Oversize is not stock to leave.** The usual words for stock to leave (*Aufmaß*,
*surépaisseur*, *sobrematerial*, *sovrametallo*, *övermått*) mean the opposite sign, so
they are never used for it.

## Suggestions for reviewers

Raised by the translators, not yet applied; a native speaker who machines decides.

- **de:** onion skin *Restschicht* sits close to *Restmaterial*, *Restbearbeitung*,
  *Restdurchgang*: *Haut*? Step over could be shortened to *Seitl. Zustellung* in labels.
- **fr:** *navigateur* reads as a web browser: *arborescence*? Rest machining simply
  *reprise*; V-bit shorter as *fraise en V*; spoilboard *plateau martyr*.
- **es:** follow path *seguir trayectoria* reads as "follow the toolpath": *seguir línea*?
  Pocket *cajera* is Spain's word; *vaciado* or *cavidad* in Latin America. Stock: allow
  *material* alone in labels.
- **it:** onion skin *pelle* or *strato di fondo*; plunge *tuffo*; spoilboard also *martire*.
- **sv:** flutes *antal skär* (not to mix with *finskär*); Browser *projektträd*; setup sheet
  *riggningsblad*; traced picture *vektoriserad bild*.
