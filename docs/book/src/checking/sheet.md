# Setup sheet

With the G-code, Chipload writes a **setup sheet** for the operator: an HTML page next to
it, `part-sheet.html`. Open it in a browser, or print it. It holds:

- **Time:** the estimated cycle time.
- **Stock and zero:** the stock's size and thickness, where X0 Y0 and Z0 are, and the
  copies. With several [parts](../operations/parts.md), each part with its copies.
- **A top view:** the stock, the part, each file's or tool's cuts, the tabs, and the X0
  Y0 mark.
- **Files and tools in order:** for each file or tool change, the tool to put in, the
  operations it cuts, and the time.
- **Operations:**
  - with several parts, under each part's name;
  - their kind, side and direction;
  - their depths and passes;
  - feeds with the chip load;
  - entry, tabs and time.
- **Watch out:** every warning and check warning.

The sheet is written:
- in the app, by *File > Export G-code…* when Job settings > Machine > *Setup sheet* is on
  (the default; it applies to every project);
- on the command line, by `chipload run` unless you add `--no-sheet`;
- alone, by `chipload sheet part.chipload -o sheet.html`.
