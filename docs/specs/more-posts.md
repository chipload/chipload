# Spec: more post-processors

Status: **draft for review, 2026-11-22.** From the review of capability gaps (the user:
"Do 1, 2 and 4"). Builds on user-posts.md (posts as data, the checker's list of codes),
inch-units.md and tool-per-operation.md.

## 1. What is missing today

Three posts are built in: GRBL, LinuxCNC and Mach3/4. Many routers people buy run other
controllers: MASSO, Centroid Acorn (CNC12), UCCNC; and GRBL's successors grblHAL and
FluidNC. Their owners have to find out which built-in post is close enough, or write their
own.

## 2. What is added (decision 1)

Each post is a TOML config and a template (ADR 0003/0007), written from the controller's
own public documentation (CLAUDE.md, clean room), never from another CAM program's post.

| Post | Controller | What differs from the posts we have |
|---|---|---|
| `masso` | MASSO G2/G3 | **G04 P is milliseconds** (MASSO's documentation of G04); `.nc`; the codes in its supported-codes list only |
| `acorn` | Centroid Acorn / CNC12 | per its operator's manual, checked when built (§3) |
| `uccnc` | UCCNC | per its manual: Mach3-style codes, checked when built (§3) |

**grblHAL and FluidNC** read GRBL's dialect, so they get no post of their own (decision 2).
The GRBL post's description and the guide say it is for **GRBL, grblHAL, FluidNC and
Carbide Motion**, and `post list` shows that, so the right post is easy to find: post
configs get an optional `controllers` list (no schema bump: missing means just the post's
name), shown by `post list`, `post show` and the machine's Post box.

## 3. Each post, from its manual

When each post is written, its manual is read for, and the spec's *As built* records:
- arc centres: I and J relative to the start (G91.1, or the default), else refused;
- the dwell's unit (seconds or milliseconds) and format;
- the start of a program: which of G17, G21/G20, G40, G49, G54, G80, G90, G91.1, G94 it
  accepts, and whether G64 is supported and what its P means;
- the end of a program (M30 or M2), comments, line numbers (none unless needed), the usual
  file extension;
- tool changes: one file per tool (as LinuxCNC and Mach3 today), since a program with M6
  needs tool lengths from the controller's table, which the checker can't see.

A post whose manual can't be found, or leaves one of these open, is not shipped; it waits
(decision 3). Each ships with a note on the setup sheet and in the program, "Not yet tested
on a real … machine: check the program and air-cut first", as LinuxCNC's and Mach3's do,
until someone reports it working.

## 4. The checker

The checker reads what the posts write. A dwell in milliseconds needs it to know the
unit: the post config gets `dwell_unit = "s" | "ms"` (optional, default `s`; post configs
stay schema 1, since a missing field means what it did), and the checker is told it. The
checker's other rules are unchanged; a post that needs a code outside its list gets that
code added only after it is checked against the standard and the manual.

## 5. Tests

- Each new post renders every golden project; the programs pass the checker. New goldens:
  one program per post (the bracket, `v43_bracket`), with arcs; one in inches.
- MASSO's dwell is `G04 P3000` for 3 s, and the checker reads it as 3 s.
- `post list` names the controllers each post is for.

## 6. Decisions (recommendations first)

1. **MASSO, Centroid Acorn and UCCNC**, each written from its public manual.
   *Alternatives:* more (PlanetCNC, Fanuc-style generic), later on request.
2. **grblHAL and FluidNC use the GRBL post,** named as such. *Alternative:* their own
   posts with M6 tool changes through the controller's tool-length probe (grblHAL's tool
   change modes), a feature of its own, later.
3. **A post ships only when its manual answers §3;** otherwise it waits.
4. **`dwell_unit` in the post config,** optional, no schema bump.

## 7. Order of work

One commit per post, the checker's `dwell_unit` with MASSO, the GRBL post's description
first.
