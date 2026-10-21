# Spec Delta

## MODIFIED Requirements

### Requirement: The lattice target source

By default hint mode SHALL generate a regular lattice across the target output as
its target source: the output is partitioned into a grid of cells, and each cell
carries one target at its centre and one label from the configured alphabet. The
partition SHALL cover the whole output, and SHALL NOT have more cells on either
axis than the alphabet has characters, so that a cell grows rather than a label
growing. The label set SHALL be prefix-free (no label is a prefix of another), so
that reaching a unique match while typing is enough to position immediately. The
label length SHALL be the shortest the alphabet can name the chosen number of
cells in, and the spare room in that budget SHALL become one-character labels for
the first targets in the lattice's order.

#### Scenario: Generating lattice hints

- **WHEN** the user presses `f` in normal mode to enter hint mode
- **THEN** the overlay draws a grid of cells with a label centred in each, the
  labels are taken from the configured alphabet, every label is one or two
  characters long, and no two labels where one is a prefix of the other are
  produced

#### Scenario: The cells tile the output

- **WHEN** the lattice is generated for an output
- **THEN** the cells cover the output without gaps or overlap, and each cell's
  label sits at that cell's centre

#### Scenario: More targets than the alphabet can label

- **WHEN** the configured lattice spacing asks for more cells on an axis than the
  alphabet has characters
- **THEN** that axis is capped at the alphabet's length, the cells grow to cover
  the output, and every label stays at two characters rather than growing to
  three

#### Scenario: More targets than the alphabet can name at all

- **WHEN** the configured lattice spacing asks for more cells than the alphabet
  can name
- **THEN** the session runs with the capped lattice rather than refusing to
  start, and never produces a duplicate label

#### Scenario: Density is configurable

- **WHEN** the configuration sets the hint lattice spacing
- **THEN** the number of cells follows that spacing up to the alphabet's cap, and
  the labels stay prefix-free

#### Scenario: Spare budget shortens the first labels

- **WHEN** the number of cells leaves room in the two-character budget
- **THEN** as many of the first targets as that room allows carry a
  one-character label, and the remaining labels do not start with any character
  those short labels used

#### Scenario: A label keeps its angular size

- **WHEN** the output is taller than 720 logical pixels
- **THEN** the label type size grows in proportion, so a label covers the same
  fraction of the output rather than getting smaller as the output grows
