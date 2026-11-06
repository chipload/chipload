# Look and feel: shades, colour and cards

Status: steps 1 to 3 asked for by the user, 2026-11-06 ("give a little more color and
cleaner/clearer design of the ui"; "do 1-3 first, use accent from logo"). Built on the
branch `look-and-feel`. Steps 4 to 9 of the proposal (the report, the Browser, the status
bar, a typeface, the light theme's review) come later.

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

## 6. Checking it

18. The contrast test (`style.rs`) covers every new pair: text on chrome, panels and cards,
    weak text on cards, text in fields, text on both accents.
19. Pictures before and after (the guide's screenshot set, rendered headless) in the dark
    theme; light theme pictures with `CHIPLOAD_SHOTS_THEME=light`; one narrow window and
    German, the longest language, so nothing gets cut.
20. The guide's screenshots are renewed at the end of the branch, in one commit.

## 7. Not in this step

The report's layout, the Browser's spacing and pills, the status bar, a bundled typeface
(the fonts in question are OFL, which needs the user's approval), the light theme's review
as a whole.
