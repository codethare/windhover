# Design

## Context

See `proposal.md`. The pieces this touches:

- `Lattice` holds one number, the wanted spacing, and derives the cells from
  `floor(axis / spacing)`.
- `labels::assign` picks one length for everyone and enumerates in base `k`.
- `Hint` stores the cells and their labels; the session builds both.
- The label's type size is `theme.badge_font_size`, a constant.

## Goals / Non-Goals

**Goals:**

- A cell count bounded by the alphabet, so the label length is bounded too.
- Shorter labels where the budget has room, without ever breaking the
  prefix-free property.

**Non-Goals:**

- Ranking targets by where the user is looking. The lattice has no importance to
  rank by; the short labels go to the first cells in the lattice's own order,
  which is top left first, and stay there for the whole session. Proximity
  ranking is the natural next step and belongs in its own change.
- A configurable type scale. The reference size stays the theme's; only the
  output-height factor is new.

## Decisions

### The cap is per axis, not on the total

Capping the total at `k²` would have left a 26x15 lattice alone and only shown up
in odd configurations; capping each axis is what stochos' grid does, and it is
what makes the label length a property of the alphabet rather than of the aspect
ratio. The cost is that a 16:9 output gets 20x15 cells whose width is larger than
their height — bigger cells, not rectangular distortion of the label, which stays
centred.

### The short-label count is the general form of stochos' rule

stochos states `(k^len - n) / (k - 1)`, which is exact for two-character labels.
For any length the cost of one short label is `k^(len-1) - 1` long labels, since
the long labels may not start with the character it took. The two agree at
`len = 2`, which is the only length the capped lattice reaches; the general form
is written down so a smaller alphabet cannot silently produce an infeasible
assignment.

### A one-character alphabet gets one label

`k = 1` has no room to distinguish anything: one character names one cell, so
`assign` returns a single label rather than looping, and the cap makes the lattice
1x1 in that case anyway.

### The type size scales from the theme's reference

`frame.label_size = theme.badge_font_size * height / 720`. Multiplying rather than
replacing keeps the theme as the one place a size is named, and the default 14 at
a 720-high output is `height / 51` — the ratio the request names.

## Risks / Trade-offs

- **Density drops on 16:9**: 390 cells become 300, so the worst-case miss grows
  from 40 px to 47 px on a 1440p panel. That is the price of the cap, and the
  speed presets are what close it.
- **Three-character labels are unreachable now**, so the documented fallback for
  a lattice that needs them is gone with the refusal that guarded it. Restoring
  it means giving up the cap.
- **Short labels sit in the top-left corner**, whatever the user is looking at.
  Deterministic, learnable, and not what stochos does with a detector; recorded as
  the next step rather than pretended.
- **A label that grows with the output needs a bigger badge**, so at 2160 high the
  label is 42 px and the badge about 80 px wide inside a 192 px cell — the cell is
  what caps it.
