# Look and feel: shades, colour and cards

Status: steps 1 to 3 asked for by the user, 2026-11-06 ("give a little more color and
cleaner/clearer design of the ui"; "do 1-3 first, use accent from logo"), then 4 to 6
("do 4-6"), then 8 and 9 ("do 8 and 9"; Inter and the OFL approved, 2026-11-06). Built on
the branch `look-and-feel`, merged into main 2026-11-06 with the guide's screenshots
renewed.

## 1. Why

The window is one grey: the top bar, the ribbon, the panels and their tab bars are near the
same shade, parted by hard black lines. The ribbon's icons are all one grey, so the
operations don't stand out, and a disabled button is hard to tell from one that works. The
Properties forms are rows floating on the panel: sections are told apart only by their bold
titles, fields have many widths, and a form can say its name twice (*Machine* as the
panel's title and again as its first section).

The canvas already reads well: shapes, toolpaths, tabs and handles have colours of their
own. This is about the window round it.

## 2. The accent comes from the logo

The brand's colours (packaging/brand/README.md): green `#9ab964`, deep green `#6f8f3c`,
orange `#f29e38` (the tool, and only the tool), light `#f3f5f0`, dark `#1d2025`.

1. **The accent is the brand's green.** The orange stays the logo's tool alone.
2. **Two strengths of it:**
   - *strong*, for the one action a screen leads to: **Generate**. Dark theme: the green
     `#9ab964` with dark text `#1d2025`. Light theme: a deeper green `#55732a` with white
     text (the brand's deep green gives white only 3.6:1).
   - *quiet*, for what is selected or switched on (a Browser row, a toggled button, Edit or
     Simulate): a muted green that light text reads on. Dark: `#4d6930` with white. Light:
     `#d5e3bd` with dark text.
3. Every text on an accent keeps 4.5:1 (WCAG AA), as every text does now; the contrast
   test covers both strengths.
4. The canvas's colours don't change: the toolpath green there is the feed's, not the
   accent, and selected shapes stay pink.

## 3. Shades: chrome, panels, cards (proposal 1)

5. **Four shades and a raised one**, each a step apart, slightly cool, after the brand's
   dark:

   | | dark | light |
   |---|---|---|
   | top bar, darkest | `#121416` | `#d9ddd4` |
   | chrome: ribbon, tab bars, status bar | `#191b1f` | `#e6e9e2` |
   | panels | `#22252a` | `#f3f5f0` |
   | cards (sections of a form) | `#2b2f35` | `#ffffff` |
   | fields (inset, darker than the card) | `#15171a` | `#ffffff` with an outline |
   | buttons | `#383d44` | `#e2e6dd` |
   | dividers, 1 px | `#30343a` | `#d0d5cb` |

   The canvas keeps its own background. The project's tab in the top bar is chrome, so it
   joins the ribbon under it.
6. **Soft dividers.** Lines between the dock's panels and under the ribbon are the divider
   shade, not black.
7. **Tabs:** the tab bar is chrome; the active tab is the panel's shade, so it joins its
   panel, its name in the normal text colour; other tabs show their names weak, with no box
   round them. No accent line under the active tab: egui_dock 0.21 has no place for one,
   and the shade says it.
8. The panel's ▼ (fold) and × keep their place, in the weak colour.

## 4. Colour by meaning (proposal 2)

9. **Operations keep one colour each, everywhere** (`style::op_color`): the Browser's
   operation icons already have them (profile teal, pocket violet, V-carve amber, follow
   path blue, holes rose, surface olive). The ribbon's operation buttons now paint their
   icons in the same colours, and so does an operation's title in Properties.
10. **The Browser's other rows get quiet tints:** drawings and layers a soft blue (the
    shapes' world), setups and stock wood (the simulation's stock), tools and the machine
    steel. A hidden row stays dimmed, as now.
11. **The rest of the ribbon stays neutral**, in the text colour. Generate is the only
    filled button.
12. **Disabled is clearly off:** a disabled button's icon and name at 40 % of the weak
    text colour, a coloured icon greyed the same, so *Profile* with nothing selected no
    longer looks like a button that works. Its hover still says why (as now).

## 5. Cards in Properties (proposal 3)

13. **Each section of a form is a card:** the card shade, corners of 6, 8 px inside, 6 px
    between cards. The header (arrow, bold title, weak summary) sits at the card's top; a
    folded section is a card of one line. A form's rows that are in no section (an
    operation's name, shapes and tool; a text; a tool) are a card too. Inside a card a
    section's rows are indented 6 px, not egui's 18: the card already shows what belongs
    together, and a narrow panel needs the room. Every form is laid out for the card's
    inner width (`set_form_width`), so label columns worked out before a card fit in it.
14. **One heading:** the panel's title line names what is shown; a form whose first
    section has the same name as the title leaves out that section's title and shows its
    rows straight under the heading, in a card.
15. **An operation's cards have its colour** as a 3 px stripe down their left edge, so a
    long form still says what kind it is.
16. **Fields line up:** in a card's grid, text fields and choice boxes are 110 px (less in a
    narrow panel) and number fields at least 72 px (at most 60 % of the value column, so a
    button fits beside one), so the values form a column. Fields side by side (X Y Z, X Y,
    a number with its buttons) share the value column (`share_row`).
17. Labels stay weak and values bright, as decided 2026-11-03.

## 6. The report, easy to scan (proposal 4)

18. **The headline is a pill** in its tone: a faint fill of the tone's colour, the mark and
    the line in bold (✔ green, ⚠ amber, ✖ red).
19. **Warnings and errors come first,** in a card of their own with a stripe in the worst
    one's colour; each still opens its operation.
20. **The time estimate is a table** in a card: the total (with cutting, rapids, spindle and
    tool changes) in bold, then a row per part, operation (under its part) and file, its
    name beside its time; an operation's row opens it. The rows come from the pipeline as
    data (`estimate::estimate_rows`), which `estimate_lines` also builds its sentences
    from, so the command line and the setup sheet read as before, byte for byte.
21. **The other notes** (the cut outline, lengths and lifts, what the estimate leaves out,
    the work zero, facts) follow in a card, and the drawing's facts in one of their own.
    Changed 2026-11-07 (docs/specs/report-details.md): after a generate they are folded
    under *Details*, the lengths in all left out and the caveat in the total's hover.

## 7. A tidier Browser (proposal 5)

22. **The selected row is a rounded highlight** starting at the row's own indent, so the
    guides of the folders above stay clear of it; a hovered row the same in the hover
    shade.
23. **Counts and tool numbers are pills** ("18", "T1"); longer notes at a row's end
    ("drawing origin", "12 mm", "48 s · T2") stay weak text.
24. **The eye shows on hover,** or when the layer is hidden; its place is kept, so names
    don't move.
25. **3 px before each of the project's main groups** (Tools, Drawing, Setups and the like).

## 8. The status bar (proposal 6)

26. **The hint's keys are key caps:** each language's text marks its keys or mouse actions
    in [brackets] (`menu-status-hint`, `menu-status-simulate-hint`), so every language keeps
    its own key names. Items are parted by " · " and left out whole when the bar is short,
    after "…"; the whole hint is in the hover. The facts at the right (pieces, cycle time,
    post, grid, units, active part) are chips instead of text between separators.

## 9. A typeface of our own (proposal 8)

27. **Inter 4.1** (rsms/inter, the static `Inter-Regular.ttf` and `Inter-SemiBold.ttf`, as
    released) is built into the window, so it reads the same on Windows, macOS and Linux.
    It's made for screens: open shapes, a tall x-height, clear at 12 px. It has Latin,
    Cyrillic and Greek, so all seven languages.
28. **SemiBold is the bold:** headings, section titles, the report's total and the like were
    only brighter text; they are now SemiBold too (`style::bold`). Body text is Regular.
29. **The system's fonts stay as fallbacks**, for what Inter lacks (the symbols ✔ ⚠ ✖ ▸ and
    scripts beyond Cyrillic and Greek) and for the monospace text of reports. A computer
    without any usable font now starts all the same.
30. **Licence:** SIL Open Font License 1.1, approved by the user 2026-11-06 (ADR 0008
    amended, OFL-1.1 on the CLAUDE.md list). Inter declares no Reserved Font Name; the
    files are used unchanged, and the licence travels with them (`fonts/LICENSE-OFL.txt`,
    THIRD-PARTY-LICENSES.txt).
31. Not done: tabular figures. egui shapes text without OpenType features, so Inter's
    `tnum` can't be switched on; the files would have to be changed to make it the default,
    which isn't worth a modified font.

## 10. The light theme, reviewed (proposal 9)

32. Every screen of the guide's set is looked at in the light theme, and what reads weak,
    harsh or wrong is changed in the light colours only; the dark theme stays as it is.
    Looked at 2026-11-06, with Inter, in every picture of the set:
    - **Cards get a soft edge** (`card_edge`, `#dce0d6`, 1 px): a white card on the
      near-white panel had nothing to show where it ended. The dark theme has none; its
      shade is enough, and its layout stays to the pixel.
    - Kept as they are: the canvas's cool grey (its own colours, §2), the toolpath's dark
      green and the selection's pink on it, the ghosted setups, the simulation's wood, the
      ribbon's disabled icons, the accents (the contrast test covers them), the floating
      view bar and the picture-trace window.

## 11. Checking it

33. The contrast test (`style.rs`) covers every new pair: text on chrome, panels and cards,
    weak text on cards, text in fields, text on both accents.
34. Pictures before and after (the guide's screenshot set, rendered headless) in the dark
    theme; light theme pictures with `CHIPLOAD_SHOTS_THEME=light`; one narrow window and
    German, the longest language, so nothing gets cut.
35. The guide's screenshots are renewed at the end of the branch, in one commit.

