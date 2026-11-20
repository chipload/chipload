# Spec: what doesn't fit goes onto more boards; nest every setup again

Status: **approved and built 2026-11-20** (decisions in §4, all as recommended; §6). From the user's
testing of nesting: "parts that don't fit when nesting need to move to a new setup, right
now they are switched off and hard to find or easy to forget", and "we should be able to
re-nest all setups or just one".

Builds on nesting.md, nesting-search.md and parts-and-setups.md §9 (nesting onto more
boards). No file format or G-code changes.

## 1. What doesn't fit goes onto more boards, always

- **The Nest window, *Nest again* and `chipload nest`** put what doesn't fit on the board
  onto the next setup with the same stock and a size of its own, else onto a new board made
  like this one, as *Onto more boards* did (parts-and-setups.md §9). The window's *What
  doesn't fit* choice goes (one way, the better one).
- **`--overflow`** is still accepted on the command line, for scripts: `new-boards` is what
  happens anyway, and `off` is refused with the reason ("what doesn't fit goes onto more
  boards; remove a setup you don't want").
- **Still switched off:** an instance larger than an empty board of its stock, which no
  board can take. It is no longer easy to miss:
  - **its row in the Browser** is drawn in amber, its hover saying it doesn't fit any
    board;
  - **its Report line** opens it when clicked.

## 2. Nest every setup again

- **Where:** *Nest all setups again* in the project's menu (its Browser row and the
  Browser's empty space) and *Wizards*, with two setups or more that can be nested;
  `chipload nest P --all` on the command line. *Nest again* on one setup stays.
- **What:** the boards of one stock that can be nested (a size of their own; not a back,
  not in pieces) are nested as one pool, in the setups' order: the first filled first, then
  the next, each with its own gap, margin and goal, the parts' turns as kept. What fits on
  none goes onto new boards (§1). Instances keep their numbers; one may move to another
  board.
- **Never worse:** as *Nest again* (nesting-search.md §5), each board is searched from as it
  is, and a board's new layout is kept only when the whole is better: more placed, then
  fewer boards used, then each board's goal.
- **A board left empty** (parts removed, counts lowered) stays, reported ("Setup 3 is empty
  now"), never removed.
- **Several stocks:** each stock's boards are a pool of their own.
- One undo step in the window; the Report lists each board's count.

## 3. Tests

- Pipeline: overflow always (a part too large still off, named); `nest_all` pools two
  boards: removing instances from board 1 lets board 2's move back; never worse; an empty
  board kept and reported; two stocks apart; backs and pieces untouched.
- Command line: `--overflow off` refused, `new-boards` accepted; `--all`.
- Window: no *What doesn't fit* choice; *Nest again* overflows; the amber row and its hover;
  the Report line opens the instance; *Nest all setups again* from the project's menu, one
  undo step.

## 4. Decisions

1. **The "switch off" choice goes;** what doesn't fit always goes onto more boards.
2. ***Nest all setups again* pools the boards** of one stock.
3. **A board emptied by it is kept and reported,** not removed.
4. **An instance too large for any board is marked amber in the Browser,** its Report line
   opening it.

## 5. Order of work

1. Overflow always, the amber row and the Report line (§1).
2. *Nest all setups again* and `nest --all` (§2).

## 6. As built

- **Step 1:** the overflow is always on (`NestRequest` lost its `overflow`); a part
  asking for another stock always goes to a board of its own. The switched-off message
  now reads "… is larger than an empty board: switched off". The amber mark is kept for
  the session, from the last nest, while the instance is off (the file doesn't say why an
  instance is off).
- **Step 2:** a board of its own taking what overflows keeps its own gap, margin and goal
  (it took the first board's before), as §2 asks. `chipload_pipeline::nest::nest_all`: every board of one stock moved onto the
  first and nested from there, overflowing as §1 into the next of that stock in the
  setups' order, then new boards; kept when it places more, or as many on fewer boards;
  else each board nested again on its own. *Nest all setups again* in the project's menu,
  the *Setups* folder's menu and *Wizards* (greyed with one board), on a worker thread,
  one undo step, the Report listing each board's count, boards made and emptied, and what
  no board takes (its line selecting it). `chipload nest --all` (with `--dry-run`,
  `--json`, `-o`; refused with one setup's options).
