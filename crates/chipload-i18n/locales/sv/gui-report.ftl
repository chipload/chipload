# The report panel under the canvas: what the last action had to say, and the drawing's
# facts (crates/chipload-gui/src/app/report.rs).

# The panel's name, a noun.
report-heading = Rapport
report-details = Detaljer ({ $count })
# The heading of the drawing's facts. An issue is something the import repaired or ignored.
report-drawing-title = Ritning: { $contours ->
        [one] { $contours } kontur
       *[other] { $contours } konturer
    }, { $issues ->
        [0] inga anmärkningar
        [one] { $issues } anmärkning
       *[other] { $issues } anmärkningar
    }
report-select-operation-hover = Markera operation { $number }
# A line of a list in the report, indented under its heading: $line is a message from
# the project (a problem). Only the two spaces in front are this text's.
report-indented = {"  "}{ $line }
report-select-tool-hover = Öppna verktyg T{ $tool }
report-update-tools-hover = Ge de här verktygen bibliotekets värden, som ett ångra-steg
report-place-dowels = Placera styrhålen igen
report-place-dowels-hover = Flytta de två styrhålscirklarna dit de hör nu, fria från båda sidornas skär, som ett ångra-steg. Inte när framsidan redan är fräst: dess hål är borrade
report-part = Detalj { $number } · { $name }
report-file = Fil { $number }
