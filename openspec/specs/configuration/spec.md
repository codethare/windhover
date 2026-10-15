# configuration Specification

## Purpose
Defines how a user adjusts key bindings, movement step, partition, hint density
and colours without recompiling: where the configuration file lives, its syntax,
how it handles mistakes, and how the command line, the file and the built-in
defaults take precedence.

## Requirements

### Requirement: Location and precedence

The configuration SHALL be read from `$XDG_CONFIG_HOME/windhover/config`,
falling back to `~/.config/windhover/config` when `XDG_CONFIG_HOME` is unset, and
the command line's `--config <path>` SHALL replace that location. Precedence,
highest first, SHALL be: command-line arguments, the configuration file, the
built-in defaults. A missing configuration file SHALL not be an error; the session
runs on the built-in defaults.

#### Scenario: Using the default location

- **WHEN** the user gives no `--config` and a configuration file exists at the
  default location
- **THEN** the session takes effect according to that file

#### Scenario: No configuration file

- **WHEN** no configuration file exists at the default location
- **THEN** the session runs on the built-in defaults, without reporting an error
  and without exiting

#### Scenario: A named file that is missing

- **WHEN** the user gives `--config <path>` and that file does not exist or cannot
  be read
- **THEN** the session exits non-zero and says why, rather than silently falling
  back to the defaults

### Requirement: Syntax

The configuration file SHALL be text of one `<option>: <value>` per line, with
blank lines ignored and lines starting with `#` ignored as comments. Whitespace
around the option name and the value SHALL be tolerated. For options that take
several values, the values are separated by whitespace and are interchangeable.

#### Scenario: Comments and blank lines

- **WHEN** the file contains blank lines and lines starting with `#`
- **THEN** those lines are ignored and do not affect how the rest is parsed

#### Scenario: Equivalent ways to list several values

- **WHEN** the file contains `hint_mode: f` and later `hint_mode: a`
- **THEN** both `f` and `a` become keys that enter hint mode

#### Scenario: A value containing whitespace

- **WHEN** an option's value needs to contain content other than leading and
  trailing whitespace (a font family name, for instance)
- **THEN** everything after the first `:` on the line is used as the value, with
  leading and trailing whitespace removed

### Requirement: Binding and unbinding keys

Key options SHALL bind by the resolved key symbol. Repeated occurrences of one
option SHALL keep every binding, and if one key ends up bound to several options,
the latest definition wins. `unbind` SHALL be a special value that removes that
option's existing bindings.

#### Scenario: Rebinding a key

- **WHEN** the configuration binds `normal_up` to a new key
- **THEN** that new key moves up, and the original default key no longer does

#### Scenario: Unbinding

- **WHEN** the configuration gives `unbind` as the value of a key option
- **THEN** every default binding for that option is removed, and pressing the
  original default key produces no such action

#### Scenario: The later definition wins

- **WHEN** the same key is first bound to a movement and then to a mouse button
- **THEN** that key performs the later mouse-button action

### Requirement: Configurable options

The configuration SHALL cover at least the following options, with the value in
parentheses as the default: the quit key (`Escape`), the four normal-mode
direction keys (`h` `j` `k` `l`), the normal-mode step (derived from the output
size, about 10 logical pixels), the three edge-jump keys (`H` `M` `L`), the three
mouse-button keys (`m` `,` `.`), the drag toggle (`v`), the keys that enter grid
and hint mode (`g` `f`), the grid partition (`2x2`), the grid cell key table, the
grid minimum size, the grid undo key (`BackSpace`), the grid warp key (`Return`),
the grid warp-and-click key (`Space`), the hint lattice spacing, the hint label
alphabet, whether to click after a warp (no), the hint editing key (`BackSpace`),
the two scroll keys (`u` up, `d` down), the font family, and the colours used by
grid and hint mode.

#### Scenario: Changing the movement step

- **WHEN** the configuration sets the normal-mode step to a larger value
- **THEN** each directional movement covers the correspondingly larger distance

#### Scenario: Changing the hint density

- **WHEN** the configuration sets the hint lattice spacing to a smaller value
- **THEN** more targets are generated, and the labels stay prefix-free

#### Scenario: Changing the colours

- **WHEN** the configuration provides a new accent colour
- **THEN** grid mode and hint mode use the same colour rules with that accent for
  labels, borders and grid lines alike

### Requirement: Handling configuration errors

A single bad line SHALL not make the session fail silently: an option that cannot
be parsed SHALL produce a warning on standard error that identifies where it came
from, and the rest of the configuration SHALL still apply. The session SHALL exit
non-zero only when the error makes it unable to run (a named file that cannot be
read, for instance).

#### Scenario: An unknown option

- **WHEN** the configuration file contains an option name that is not recognised
- **THEN** the session warns with the content of that line, ignores it, and applies
  the rest as usual

#### Scenario: An illegal value

- **WHEN** an option's value is illegal (a partition of zero, for instance)
- **THEN** the session warns and uses that option's default, rather than running
  with the illegal value

#### Scenario: Warnings do not break the session

- **WHEN** the configuration file contains several ignored lines
- **THEN** the session still runs as an interactive session, and exits with status
  0
