# The report panel under the canvas: what the last action had to say, and the drawing's
# facts (crates/chipload-gui/src/app/report.rs).

# The panel's name, a noun.
report-heading = Report
# The Report's fold under the time estimate after a generate: its notes and the drawing's facts, how many.
report-details = Details ({ $count })
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
# A line of a list in the report, indented under its heading: $line is a message from
# the project (a problem). Only the two spaces in front are this text's.
report-indented = {"  "}{ $line }
report-select-tool-hover = Open tool T{ $tool }
report-update-tools-hover = Give these tools your library's values, as one undo step
# The estimate as a table in the report (docs/specs/look-and-feel.md §6): a part's row,
# and a file's when the job is written one file per tool.
report-part = Part { $number } · { $name }
report-file = File { $number }
