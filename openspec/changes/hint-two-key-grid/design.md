# Design

## Context

See `proposal.md` for why. The current shape that matters here:

- `Lattice` (`src/hint/lattice.rs`) holds one number, `cell`, and derives
  `columns = floor(width / cell)`, `rows = floor(height / cell)`, then places one
  point at each cell's centre, distributing the cells evenly so they tile the
  output. `Target` (`src/mode/hint.rs`) therefore carries a point, not a region.
- `labels::assign(count, charset)` picks one length for every label, starting at
  one character and growing while `charset.len().pow(length) < count`, capped at
  8.
- `draw_hint` (`src/render/mod.rs`) draws a `badge` — a translucent pill with a
  hairline edge and the label inside — at each visible target.
- `hint_charset` is validated as alphanumeric only (`src/config.rs`), so `;`
  would be rejected.

## Goals / Non-Goals

**Goals:**

- A two-character label under the default alphabet, on any output size.
- A label length that is a consequence of the cell count, not of the output
  size: the same configuration reads the same on a laptop panel and on a 4K
  monitor.

**Non-Goals:**

- Recursive subdivision (stochos' `bisect` mode) and any refinement step that
  stays inside hint mode. Refinement stays where it already is: the hand-over to
  normal mode.
- Labelling real UI targets rather than lattice cells.
- Changing grid mode's visuals.

## Decisions

### The cell count is bounded by the alphabet, and the default spacing follows from it

`hint_cell` keeps its meaning and its code path. What changes is its default: it
is derived as `sqrt(width * height / len(charset)^2)`, which is the spacing at
which the output is filled with exactly `len(charset)^2` cells. For the default
alphabet that is 400 cells: 26x15 on a 16:9 output of any size.

The alternative — making the lattice count-based (`Lattice { columns, rows }`) —
was rejected: it replaces one field with two, changes the meaning of the
`hint_cell` option users already have, and buys nothing, because
`sqrt(area / budget)` produces the same partition.

### The label length is a runtime check, not a configuration check

The cell count depends on the output size, which is only known once the session
has its overlay. So the length check happens where the lattice is built
(`reset_hint`), not in `apply`: if `labels::assign` would need four characters,
the session refuses to start and names `hint_cell` as the option responsible.
That keeps the configuration parser free of output-size knowledge and gives one
error path instead of two.

### The maximum label length moves from 8 to 3

`labels::assign` grows the length while the count does not fit; the cap becomes
the thing the session refuses at. Three characters name 8000 cells with the
default alphabet, which is past any real output at a legible cell size, so the
cap is a guard against a nonsensical configuration rather than a limit anyone
meets.

### A cell is a translucent rectangle, and the label keeps its own panel

`Lattice` grows a `cells` method returning `(x, y, w, h)` per cell beside the
centre it already yields, and `Target` carries that rectangle. `draw_hint` fills
it with a new theme value — `panel` at low alpha, `0x28`, next to the existing
`panel` and `panel_edge` — strokes the hairline, and then draws the label with
the same `badge` grid mode uses.

The label keeps a panel because the fill cannot do that job. A capture against a
white desktop measured the cell interior at luminance 219 against the label's
255: white text on the translucent fill is invisible, and making the fill opaque
enough to carry it would hide the thing being aimed at. Two layers — a quiet cell
for the region, the existing panel for the text — is the only arrangement that
satisfies both requirements in the spec, and it reuses the badge unchanged.

Alternatives: keeping the pill alone and only lowering the spacing (rejected —
the request is the grid look); drawing cell edges without a fill (rejected — the
fill is what makes a cell read as a target region rather than as a table);
adapting the label colour to the background (rejected — the overlay would have to
sample what is under it, which it has no access to).

### `;` is accepted by relaxing the alphabet validation to printable, non-whitespace

The current check is `is_ascii_alphanumeric()`. It tightens nothing that matters
— the value is only ever compared against typed characters, and the typing path
(`printable`) already accepts any printable ASCII — and it rejects the default
alphabet the request asks for. The binding parser is untouched: a charset
character is not a binding.

## Risks / Trade-offs

- **Precision drops from 40 px to about 68 px worst case.** The hand-over to
  normal mode is where that is recovered, which is why the follow-up change adds
  speed presets there. Until it lands, fine positioning on a 4K output is three
  nudges at the default step.
  → Accepted deliberately: two keystrokes against four is the trade this change
  makes.
- **Two characters read worse than three if the alphabet's 400 cells feel
  coarse.** → The fallback needs no code: lowering `hint_cell` raises the labels
  to three characters, which is exactly what the option is for.
- **A translucent fill over a busy background can hurt label legibility.** The
  capture against white settled this: the fill alone leaves white text at 1.2:1
  contrast, so the label keeps its panel. → Done, not a residual risk.
- **`;` on a non-US layout.** The character is matched against the keysym the
  layout produces, so a layout that cannot produce `;` simply never matches that
  label prefix. → Nothing to do; it is the same behaviour every other charset
  character has.

## Migration Plan

`hint_cell`'s meaning is unchanged, so an existing configuration keeps working;
only its default moves. A configuration that pinned a spacing dense enough to
need four characters now fails at start with a message naming the option, which
is the one behaviour a user has to act on.
