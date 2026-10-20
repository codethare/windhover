# Configuration

`windhover` reads `$XDG_CONFIG_HOME/windhover/config`, falling back to
`~/.config/windhover/config`. `--config PATH` replaces that location, and a path
given on the command line has to exist: naming a file is a statement of intent,
so a missing one is an error rather than a quiet fall back to the defaults.

A missing file at the default location is not an error. Every option has a
built-in default, so a configuration file only ever needs the lines that
differ.

## Syntax

One option per line:

```
<option>: <value>
```

- Blank lines and lines whose first non-space character is `#` are ignored. A
  comment is a whole line: a `#` inside a value belongs to the value, because
  `#0a84ff` is a colour.
- Space around the option name and the value is not significant.
- Only the first `:` splits the line, so a value may contain colons.
- Options that take keys accept several space-separated keys on one line, and
  may also be repeated. Every value listed is equivalent.
- The value `unbind` removes an option's bindings, including its defaults.

A line the parser cannot use produces a warning on standard error naming the
line and the reason, and is otherwise ignored. `windhover` keeps running with the
remaining configuration.

The first line that sets a key option replaces that option's defaults; any later
line for the same option adds to it. If one key ends up bound to several
actions, the last binding wins.

## Sample

<!-- sample-config -->
```
# ~/.config/windhover/config

# Session
quit: Escape

# Normal mode movement
normal_left: h
normal_down: j
normal_up: k
normal_right: l
step: 10

# Jumps to the left edge, the horizontal middle and the right edge
edge_left: H
edge_middle: M
edge_right: L

# Wheel
scroll_up: u
scroll_down: d

# Buttons and dragging
button_left: m
button_middle: ,
button_right: .
drag: v

# Mode entry
grid_mode: g
hint_mode: f

# Grid mode
grid_columns: 2
grid_rows: 2
grid_cells: q w a s
grid_min_span: 40
grid_left: h
grid_down: j
grid_up: k
grid_right: l
grid_back: BackSpace
grid_warp: Return
grid_warp_click: space

# Hint mode
hint_cell: 56
hint_charset: asdfghjkl
click_after_warp: false
hint_back: BackSpace

# Appearance
font_family: sans-serif
accent: 0A84FF
text: FFFFFF
panel: 1C1C1EE0
```

## Options

### Keys

Every option below takes one or more key names. A name is either an xkb key
symbol name (`Escape`, `BackSpace`, `Return`, `space`, `F12`) or a single
character (`h`, `,`, `.`). Case matters: `h` and `H` are different keys, which
is what lets the shift level of a movement key jump to an edge.

| Option | Default | Action |
|---|---|---|
| `quit` | `Escape` | End the session (bound in every mode) |
| `normal_left` `normal_right` `normal_up` `normal_down` | `h` `l` `k` `j` | Move one step |
| `edge_left` `edge_middle` `edge_right` | `H` `M` `L` | Jump to the left edge, the middle, the right edge |
| `scroll_up` `scroll_down` | `u` `d` | Wheel |
| `button_left` `button_middle` `button_right` | `m` `,` `.` | Click |
| `drag` | `v` | Hold the left button, or release it |
| `grid_mode` `hint_mode` | `g` `f` | Enter grid or hint mode |
| `grid_left` `grid_right` `grid_up` `grid_down` | `h` `l` `k` `j` | Pan the grid region by its own size |
| `grid_back` `grid_warp` `grid_warp_click` | `BackSpace` `Return` `space` | Undo a shrink, warp, warp and click |
| `hint_back` | `BackSpace` | Delete a typed label character, or leave hint mode |

### Values

| Option | Default | Meaning |
|---|---|---|
| `step` | derived as `output_height / 72`, at least 4 | Normal-mode step in logical pixels. A held direction key grows this up to four times, by one factor every 200 ms, and `Ctrl` with a direction doubles it |
| `grid_columns` `grid_rows` | `2` `2` | Partition of the grid region |
| `grid_cells` | `q w a s` | Cell keys, row major. Needs at least `grid_columns * grid_rows` entries; the session refuses to start otherwise |
| `grid_min_span` | `40` | A region narrower than this in either direction hands over to normal mode |
| `hint_cell` | `56` | Wanted spacing between hint targets, in logical pixels |
| `hint_charset` | `asdfghjkl` | Label alphabet. Labels are all the same length, so the length grows rather than letting one label prefix another |
| `click_after_warp` | `false` | Click when a hint is selected |
| `font_family` | `sans-serif` | Resolved through `fc-match`, with a built-in path list as a fallback |
| `accent` `text` `panel` | macOS dark palette | `RRGGBB` or `RRGGBBAA`, with or without a leading `#` |

`accent` is the single tint the whole interface is built from. The grid lines are
the accent at a lower alpha and the region border is the accent itself, so one
line moves all three modes together. Change it and everything follows:

```
accent: FF9500
panel: F2F2F7E6
text: 1C1C1E
```

