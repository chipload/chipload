# The report panel under the canvas: what the last action had to say, and the drawing's
# facts (crates/chipload-gui/src/app/report.rs).

# The panel's name, a noun.
report-heading = Rapport
report-hide-hover = Dölj rapporten
report-show-hover = Visa rapporten
# On the closed panel's bar: warnings below the headline that can't be seen.
report-more-warnings = { $count ->
        [one] { $count } varning till
       *[other] { $count } varningar till
    }
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
