# Texts of the app that name a length or its unit, which follow the display units
# (Settings > Units): $unit is "mm" or "in", a length argument brings its own unit
# ("10 mm", "0.3937 in").

# Where X, Y and Z zero are, after "Work zero:". $at is one of the work zero choices
# ("stock bottom left"), $point where that is in the drawing ("(0.000, 0.000) mm"), $z one
# of the two below.
units-text-work-zero = { $at } = drawing { $point }, Z0 = { $z }
units-text-zero-stock-top = stock top
units-text-zero-stock-bottom = stock bottom

# Edit > Paste. $shift: how far a paste lands from the copied shapes.
units-text-paste-hover = { $shift } right and up from the copied shapes; Ctrl+V over the canvas pastes at the cursor
# $diameter: the new tool's diameter.
units-text-new-tool-hover = A { $diameter } endmill to edit; operations pick it in their Tool row
units-text-oversize-hover = How much more than drawn is cut, { $unit } (clearance in the file): positive makes a pocket larger or a part smaller, negative leaves stock on for a later cut. Less than the tool radius. Other CAM programs call the opposite "stock to leave".
units-text-picture-position-hover = The lower-left corner of the picture's box, { $unit }
units-text-text-position-corner-hover = The lower-left corner of the text's box, { $unit }
units-text-text-position-centre-hover = The centre of the text's box, { $unit }
units-text-dxf-position-hover = The lower-left corner of the drawing's box, { $unit }. Shapes drawn in Chipload and other DXFs don't move.
units-text-pitch-hover = Copy to copy, { $unit }. Negative grows to the left or down.
