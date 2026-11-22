# Spec: signs from a list

Status: **approved 2026-11-22** (the user: "Go ahead"; §5 as recommended; see *As built*,
§8). From the review of capability gaps (the user:
"Do 1, 2 and 4"). Builds on sign-maker.md, sign-size-and-start.md (a fixed size) and
parts.md (the order of cutting).

## 1. What is missing today

*New sign…* makes one sign. Name plates, door signs, place cards, labels for drawers or
gifts are the same sign with different words: twenty names means twenty runs of the
wizard, twenty parts, and a tool change back and forth for each sign.

## 2. What it does

**Signs: One / From a list** in *New sign…*. From a list:
- **The list:** one sign per line, in a box that takes a pasted list. In a line, `\n` (or
  ` | `) starts the sign's second line: `ANNA BERG | ROOM 12`.
- **Every sign the same:** font, frame, borders, style, edge and tools are the dialog's.
  With **Size: Fixed** (the default for a list, decision 2) every sign is that size and
  **every sign gets the same letters' height**: the largest that fits the longest name, so
  the signs match. With *Fit the words* each frame follows its own words.
- **One part for the batch** (decision 1), named after the list's first sign and the count
  ("ANNA BERG +19"). Its operations cut **every sign at once**: the letters of all signs,
  then all borders, then all cut-outs, so the tools change once (V-bit to end mill), not
  twice for every sign.
- **Each sign stays a sign:** its text and frame are grouped, and the frame follows its
  words, so a name corrected later still fits its frame.
- **Laid out in a grid,** a **Gap** apart (10 mm by default, room for the end mill and a
  little more): as near square as
  the count allows when the board is fitted round the signs; as many to a row as fit the
  board's width when the board is kept, more rows below. A batch that doesn't fit a kept
  board is refused with the size it needs (several boards for one batch come later).
- **The preview** shows the grid and says "20 signs, letters 24.5 mm".

**On the command line:**

```
chipload sign add P --list names.txt --size 200x60 --vbit 2 --end-mill 1
chipload sign add P --list-words "ANNA BERG|ROOM 12\nBO LIND|ROOM 14" --size 200x60 …
```

`--list FILE` (one sign per line; `|` or `\n` for the second line) or `--list-words`
instead of the words.

## 3. How the operations select every sign

Each operation selects by **layer**: the batch's letters on layer `<part> letters`, its
borders on `<part> borders`, its outlines on `<part> outline`. Layers keep selecting the
right shapes when a text is changed and its letters are made again (their ids change), and
when a sign is added or removed by hand on those layers. A raised batch's background pocket
selects each sign's group without its outline, as one sign's does, one pocket per sign in
the same operation's order.

## 4. Format

No change: a batch is texts, frames, groups, one part and its operations, all of which
exist. Nothing remembers that they were made as a batch.

## 5. Decisions (recommendations first)

1. **One part with one set of operations for the whole batch,** selecting by layer, so the
   tools change once. *Alternative:* a part per sign, which changes tools twice per sign
   (parts are cut part by part).
2. **A list defaults to Size: Fixed, with one letters' height for all,** the largest that
   fits every name. *Alternative:* each sign's letters as large as its own name allows,
   which makes short names shout.
3. **A grid, not nesting:** signs of one size pack best in a grid, and it is predictable.
   Nesting places instances of parts, which these aren't.
4. **A kept board that's too small refuses the batch** with the size needed; spreading it
   over several boards comes later, with nesting's overflow.

## 6. Tests

- **Pipeline:** 5 names make 5 texts, 5 frames, 5 groups, one part, one V-carve, one
  border and one cut-out operation; the cut-out cuts 5 outlines; tool changes counted as 1;
  every frame the fixed size and every text the same height, fitted to the longest name;
  `|` makes a second line; a grid fits the board or is refused with the size.
- **Changing a name** afterwards: its frame follows, and the letters' operation still cuts
  it (by layer).
- **CLI:** `--list`, `--list-words`, conflicts with the words.
- **Window:** *From a list* shows the list box, the preview the grid and the count.
- **Golden:** a batch of three name plates.

## 7. Order of work

1. The pipeline, the command line, the golden, the guide.
2. The wizard's *From a list*.

## 8. As built

- **Step 1:** `chipload_pipeline::sign_batch::{add_sign_batch, BatchRequest, list_words}`;
  `sign add --list FILE`, `--list-words` and `--list-gap` (`--gap` already names the
  frame's gap between borders). The operations are grouped by tool: letters, borders and
  edge with the V-bit, then a raised list's backgrounds and the cut-outs with the end mill
  (a raised single sign cuts its background first). **Inlaid lists are refused**: each
  plug needs a setup of its own, which stays one sign at a time. The batch's layers are
  named after its part in the app's language. A list counts at most 500 signs. Golden
  `v43_name_plates`.
- **Step 2:** the wizard's *Signs: One / From a list* (`SignSettings::from_list`, a list
  box in place of the words); choosing a list turns *Size* to *Fixed*; the preview shows
  "3 signs, in a grid" and the operations; *Make sign* makes the batch as one undo step.
  The gap is the default 10 mm in the wizard (`--list-gap` on the command line).
