# The report panel under the canvas: what the last action had to say, and the drawing's
# facts (crates/chipload-gui/src/app/report.rs).

# The panel's name, a noun.
report-heading = Report
report-hide-hover = Hide the report
report-show-hover = Show the report
# On the closed panel's bar: warnings below the headline that can't be seen.
report-more-warnings = { $count ->
        [one] { $count } more warning
       *[other] { $count } more warnings
    }
# The heading of the drawing's facts. An issue is something the import repaired or ignored.
report-drawing-title = Drawing: { $contours ->
        [one] { $contours } contour
       *[other] { $contours } contours
    }, { $issues ->
        [0] no issues
        [one] { $issues } issue
       *[other] { $issues } issues
    }
report-select-operation-hover = Select operation { $number }
